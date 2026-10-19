//! `nsh/ui/desktop` through the generated JS glue: its components render
//! at desktop sizes, track the pointer and keyboard states, and live next
//! to `nsh/ui/mobile`'s in one app.

use std::time::Duration;

use crossbeam_channel::Receiver;
use nsh_core::{ApproxTextEngine, EventKind, EventPayload, MountBatch, MountItem, Size};
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
