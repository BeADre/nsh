//! `nsh/ui/desktop` through the generated JS glue: its components render
//! at desktop sizes, track the pointer and keyboard states, and live next
//! to `nsh/ui/mobile`'s in one app.

use std::time::Duration;

use crossbeam_channel::Receiver;
use nsh_core::{
    ApproxTextEngine, EventKind, EventPayload, MountBatch, MountItem, Size,
    nsh_animation::{AnimationOp, Driver, Finished},
};
use nsh_engine::{Bundle, Engine, EngineConfig, Environment};

use crate::ChannelSink;

/// The app, compiled by nsh with both UI sets and loaded as bytecode.
fn start(app: &str) -> (Engine, Receiver<MountBatch>) {
    static APPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "nsh-desktop-ui-{}-{}",
        std::process::id(),
        APPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let entry = dir.join("App.tsx");
    std::fs::write(&entry, app).unwrap();
    let sets = || vec![nsh_ui_mobile::registry(), nsh_ui_desktop::registry()];
    let bundle = nsh_bundler::bundle(&nsh_bundler::BundleOptions {
        entry,
        minify: false,
        split: false,
        ui_sets: sets(),
    })
    .unwrap();
    let bytecode = nsh_js::compile_bytecode_with(&bundle.code, false, &sets()).unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    let config = EngineConfig {
        // SAFETY: compiled just now by this runtime.
        app: nsh_engine::App::Js(unsafe { Bundle::bytecode(bytecode) }),
        viewport: Size::new(1200.0, 800.0),
        trace: false,
        network: None,
        assets: None,
        environment: Environment::default(),
        header_heights: (44.0, 96.0),
        limits: nsh_engine::Limits::default(),
        ui_sets: sets(),
        runtime: None,
    };
    (
        Engine::start(config, || ApproxTextEngine, ChannelSink(tx)).unwrap(),
        rx,
    )
}

/// The id of the text node showing `wanted`.
fn text_id(batch: &MountBatch, wanted: &str) -> Option<u32> {
    batch.items.iter().find_map(|i| match i {
        MountItem::Text { id, text, .. } if text.text == wanted => Some(*id),
        _ => None,
    })
}

/// The parent `child` is inserted into.
fn parent_of(batch: &MountBatch, child: u32) -> Option<u32> {
    batch.items.iter().find_map(|i| match i {
        MountItem::Insert {
            parent, child: c, ..
        } if *c == child => Some(*parent),
        _ => None,
    })
}

/// The last frame `id` got: (width, height).
fn size_of(batch: &MountBatch, id: u32) -> Option<(f32, f32)> {
    batch.items.iter().rev().find_map(|i| match i {
        MountItem::Frame { id: f, frame } if *f == id => Some((frame.width, frame.height)),
        _ => None,
    })
}

/// The states `id` asked the platform to track.
fn tracked(batch: &MountBatch, id: u32) -> Option<u8> {
    batch.items.iter().rev().find_map(|i| match i {
        MountItem::TrackStates { id: t, states } if *t == id => Some(*states),
        _ => None,
    })
}

const BUTTONS: &str = r#"import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/ui/desktop";
import { Button as MobileButton } from "nsh/ui/mobile";

const presses = signal(0);
mount(() => (
  <View class="items-start gap-2 p-4">
    <Button title="Save" onPress={() => presses.update((p) => p + 1)} />
    <Button title="Small" size="sm" variant="secondary" />
    <MobileButton title="Phone" />
    <Text>{`presses ${presses()}`}</Text>
  </View>
));
"#;

#[test]
fn desktop_buttons_are_dense_track_the_pointer_and_keyboard_and_press() {
    let (engine, rx) = start(BUTTONS);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let button =
        |label: &str| parent_of(&first, text_id(&first, label).expect(label)).expect("the button");
    let (save, small, phone) = (button("Save"), button("Small"), button("Phone"));
    assert_eq!(size_of(&first, save).map(|s| s.1), Some(32.0), "md");
    assert_eq!(size_of(&first, small).map(|s| s.1), Some(24.0), "sm");
    // The mobile set's button, in the same app, is a finger's size.
    assert!(size_of(&first, phone).is_some_and(|s| s.1 >= 44.0));
    // Hover, press and keyboard focus restyle it natively.
    let states = tracked(&first, save).expect("tracked states");
    for state in [
        nsh_core::tailwind::state::HOVERED,
        nsh_core::tailwind::state::PRESSED,
        nsh_core::tailwind::state::FOCUSED,
    ] {
        assert_ne!(states & state, 0, "{states:#b} lacks {state:#b}");
    }
    engine
        .control()
        .dispatch_event(save, EventKind::Press, EventPayload::None);
    let mut shown = false;
    while let Ok(batch) = rx.recv_timeout(Duration::from_millis(500)) {
        if text_id(&batch, "presses 1").is_some() {
            shown = true;
            break;
        }
    }
    assert!(shown, "onPress reached the app");
}

const POPOVER: &str = r#"import { mount, signal, Text, View } from "nsh";
import { Button, Popover } from "nsh/ui/desktop";

const open = signal(false);
const closes = signal(0);
mount(() => (
  <View class="p-6">
    <Popover
      open={open()}
      onClose={() => { closes.update((c) => c + 1); open.set(false); }}
      trigger={<Button title="Options" onPress={() => open.set(true)} />}
    >
      <Text>Menu</Text>
    </Popover>
    <Text>{`closes ${closes()}`}</Text>
  </View>
));
"#;

/// The batches until one shows `wanted` (or nothing came for a second).
fn until_text(rx: &Receiver<MountBatch>, wanted: &str) -> Vec<MountBatch> {
    let mut batches = Vec::new();
    while let Ok(batch) = rx.recv_timeout(Duration::from_secs(1)) {
        let done = text_id(&batch, wanted).is_some();
        batches.push(batch);
        if done {
            break;
        }
    }
    batches
}

#[test]
fn a_popover_opens_below_its_trigger_and_closes_on_a_press_outside_or_escape() {
    let (engine, rx) = start(POPOVER);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let trigger = parent_of(&first, text_id(&first, "Options").unwrap()).unwrap();
    assert_eq!(size_of(&first, trigger).map(|s| s.1), Some(32.0));
    let control = engine.control();
    control.dispatch_event(trigger, EventKind::Press, EventPayload::None);
    // The surface: measured, then placed under the trigger (24 pt of
    // padding, 32 tall, 6 apart).
    let mut modal = None;
    // The modal's children: the backdrop, then the surface.
    let mut layers = Vec::new();
    let mut placed = None;
    while let Ok(batch) = rx.recv_timeout(Duration::from_secs(1)) {
        for item in &batch.items {
            match item {
                MountItem::Create {
                    id,
                    kind: nsh_core::ViewKind::Modal,
                } => modal = Some(*id),
                MountItem::Insert { parent, child, .. } if Some(*parent) == modal => {
                    layers.push(*child);
                }
                MountItem::Frame { id, frame } if layers.get(1) == Some(id) && frame.y > 0.0 => {
                    placed = Some((frame.x, frame.y));
                }
                _ => {}
            }
        }
        if placed.is_some() {
            break;
        }
    }
    let backdrop = layers.first().copied();
    let (x, y) = placed.expect("the surface was placed");
    assert_eq!((x, y), (24.0, 24.0 + 32.0 + 6.0));
    // A press on the backdrop closes it: the app hears it at once, and the
    // surface fades out (`motion::EXIT`) before the modal goes.
    control.dispatch_event(backdrop.unwrap(), EventKind::Press, EventPayload::None);
    let mut fade_out = None;
    let mut closed = false;
    while let Ok(batch) = rx.recv_timeout(Duration::from_secs(1)) {
        closed |= text_id(&batch, "closes 1").is_some();
        fade_out = fade_out.or(batch.items.iter().find_map(|i| match i {
            MountItem::Animation(AnimationOp::Start {
                animation,
                driver: Driver::Timing { to, .. },
                ..
            }) if *to == 0.0 => Some(*animation),
            _ => None,
        }));
        if closed && fade_out.is_some() {
            break;
        }
    }
    assert!(closed, "onClose reached the app");
    let removed = |batch: &MountBatch| {
        batch
            .items
            .iter()
            .any(|i| matches!(i, MountItem::Delete { id } if Some(*id) == modal))
    };
    // Still shown while it fades; gone once the fade ends.
    assert!(
        !rx.recv_timeout(Duration::from_millis(300))
            .is_ok_and(|b| removed(&b))
    );
    control.animations_finished(vec![Finished::new(fade_out.expect("a fade out"), true)]);
    let mut gone = false;
    while let Ok(batch) = rx.recv_timeout(Duration::from_secs(1)) {
        if removed(&batch) {
            gone = true;
            break;
        }
    }
    assert!(gone, "the modal went after the fade");
    // Opened again, Escape (the modal's request to close) closes it too.
    control.dispatch_event(trigger, EventKind::Press, EventPayload::None);
    let mut modal = None;
    while let Ok(batch) = rx.recv_timeout(Duration::from_secs(1)) {
        modal = batch.items.iter().find_map(|i| match i {
            MountItem::Create {
                id,
                kind: nsh_core::ViewKind::Modal,
            } => Some(*id),
            _ => None,
        });
        if modal.is_some() {
            break;
        }
    }
    control.dispatch_event(
        modal.expect("opened again"),
        EventKind::RequestClose,
        EventPayload::None,
    );
    assert!(
        until_text(&rx, "closes 2")
            .iter()
            .any(|b| text_id(b, "closes 2").is_some())
    );
}

const DIALOG: &str = r#"import { mount, signal, Text, View } from "nsh";
import { Button, Dialog } from "nsh/ui/desktop";

const open = signal(true);
const closes = signal(0);
mount(() => (
  <View>
    <Dialog
      open={open()}
      onClose={() => { closes.update((c) => c + 1); open.set(false); }}
      title="Delete it?"
      description="It goes for everyone."
      footer={<Button title="Delete" variant="danger" />}
    >
      <Text>Details</Text>
    </Dialog>
    <Text>{`closes ${closes()}`}</Text>
  </View>
));
"#;

#[test]
fn a_dialog_shows_its_title_content_and_actions_and_closes_on_the_backdrop() {
    let (engine, rx) = start(DIALOG);
    let mut seen = Vec::new();
    let mut modal = None;
    let mut layers = Vec::new();
    while let Ok(batch) = rx.recv_timeout(Duration::from_secs(1)) {
        for item in &batch.items {
            match item {
                MountItem::Text { text, .. } => seen.push(text.text.clone()),
                MountItem::Create {
                    id,
                    kind: nsh_core::ViewKind::Modal,
                } => modal = Some(*id),
                MountItem::Insert { parent, child, .. } if Some(*parent) == modal => {
                    layers.push(*child);
                }
                _ => {}
            }
        }
        if ["Delete it?", "It goes for everyone.", "Details", "Delete"]
            .iter()
            .all(|t| seen.iter().any(|s| s == t))
        {
            break;
        }
    }
    // The content shows between the title and the actions: lazy children
    // before the footer's slot.
    for t in ["Delete it?", "It goes for everyone.", "Details", "Delete"] {
        assert!(seen.iter().any(|s| s == t), "{t} in {seen:?}");
    }
    let backdrop = *layers.first().expect("the backdrop");
    engine
        .control()
        .dispatch_event(backdrop, EventKind::Press, EventPayload::None);
    assert!(
        until_text(&rx, "closes 1")
            .iter()
            .any(|b| text_id(b, "closes 1").is_some())
    );
}
