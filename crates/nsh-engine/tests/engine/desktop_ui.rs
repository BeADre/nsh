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

const SELECT: &str = r#"import { mount, signal, Text, View } from "nsh";
import { Select } from "nsh/ui/desktop";

const picked = signal(-1);
mount(() => (
  <View class="w-56">
    <Select options={["Apple", "Banana"]} placeholder="Pick one" onChange={(i: number) => picked.set(i)} />
    <Text>{`picked ${picked()}`}</Text>
  </View>
));
"#;

#[test]
fn a_select_shows_its_choice_and_picks_from_a_popover() {
    let (engine, rx) = start(SELECT);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let field = parent_of(
        &first,
        text_id(&first, "Pick one").expect("the placeholder"),
    )
    .expect("the field");
    assert_eq!(
        size_of(&first, field),
        Some((224.0, 32.0)),
        "as wide as its column"
    );
    let control = engine.control();
    control.dispatch_event(field, EventKind::Press, EventPayload::None);
    let batches = until_text(&rx, "Banana");
    let banana = batches
        .iter()
        .find_map(|b| text_id(b, "Banana").and_then(|t| parent_of(b, t)))
        .expect("the options");
    let list_width = batches
        .iter()
        .find_map(|b| parent_of(b, banana).and_then(|list| size_of(b, list)))
        .map(|s| s.0);
    assert!(list_width.is_some_and(|w| w >= 224.0), "{list_width:?}");
    control.dispatch_event(banana, EventKind::Press, EventPayload::None);
    let batches = until_text(&rx, "picked 1");
    assert!(batches.iter().any(|b| text_id(b, "picked 1").is_some()));
    // The field shows the choice.
    assert!(
        batches
            .iter()
            .any(|b| text_id(b, "Banana") == text_id(&first, "Pick one"))
    );
}

/// The first child inserted into `parent`.
fn first_child(batch: &MountBatch, parent: u32) -> Option<u32> {
    batch.items.iter().find_map(|i| match i {
        MountItem::Insert {
            parent: p, child, ..
        } if *p == parent => Some(*child),
        _ => None,
    })
}

/// The role and checked state `id` was given last, for screen readers.
fn checked_state(
    batch: &MountBatch,
    id: u32,
) -> Option<(nsh_core::AccessibilityRole, Option<nsh_core::Checked>)> {
    batch.items.iter().rev().find_map(|i| match i {
        MountItem::ViewProps { id: v, props } if *v == id => {
            Some((props.accessibility.role, props.accessibility.state.checked))
        }
        _ => None,
    })
}

/// The springs started in `batches`, by their target.
fn springs(batches: &[MountBatch]) -> Vec<f32> {
    batches
        .iter()
        .flat_map(|b| &b.items)
        .filter_map(|i| match i {
            MountItem::Animation(nsh_animation::AnimationOp::Start {
                driver: nsh_animation::Driver::Spring { to, .. },
                ..
            }) => Some(*to),
            _ => None,
        })
        .collect()
}

const CHECKBOXES: &str = r#"import { mount, signal, Text, View } from "nsh";
import { Checkbox } from "nsh/ui/desktop";

const agreed = signal(false);
mount(() => (
  <View class="items-start gap-2 p-4">
    <Checkbox label="Remember me" onChange={(v) => agreed.set(v)} />
    <Checkbox label="Small" size="sm" defaultValue />
    <Text>{`agreed ${agreed()}`}</Text>
  </View>
));
"#;

#[test]
fn a_checkbox_is_a_small_square_its_row_toggles_it() {
    use nsh_core::{AccessibilityRole, Checked};
    let (engine, rx) = start(CHECKBOXES);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let row = |label: &str| parent_of(&first, text_id(&first, label).expect(label)).unwrap();
    let (remember, small) = (row("Remember me"), row("Small"));
    let boxed = |row| first_child(&first, row).expect("the box");
    assert_eq!(size_of(&first, boxed(remember)), Some((16.0, 16.0)), "md");
    assert_eq!(size_of(&first, boxed(small)), Some((14.0, 14.0)), "sm");
    // The row is what the keyboard focuses and screen readers read.
    let states = tracked(&first, remember).expect("tracked states");
    assert_ne!(states & nsh_core::tailwind::state::FOCUSED, 0);
    assert_eq!(
        checked_state(&first, remember),
        Some((AccessibilityRole::Checkbox, Some(Checked::False)))
    );
    // `defaultValue`: checked, with its mark.
    assert_eq!(
        checked_state(&first, small).map(|s| s.1),
        Some(Some(Checked::True))
    );
    assert!(first_child(&first, boxed(small)).is_some(), "the mark");
    assert!(first_child(&first, boxed(remember)).is_none());
    // A press on the row (the label's too) checks it and tells the app.
    let control = engine.control();
    control.dispatch_event(remember, EventKind::Press, EventPayload::None);
    let batches = until_text(&rx, "agreed true");
    assert!(batches.iter().any(|b| text_id(b, "agreed true").is_some()));
    assert!(
        batches
            .iter()
            .any(|b| first_child(b, boxed(remember)).is_some()),
        "the mark shows"
    );
    assert!(
        batches
            .iter()
            .any(|b| checked_state(b, remember).map(|s| s.1) == Some(Some(Checked::True)))
    );
    control.dispatch_event(remember, EventKind::Press, EventPayload::None);
    assert!(
        until_text(&rx, "agreed false")
            .iter()
            .any(|b| text_id(b, "agreed false").is_some())
    );
}

const SWITCHES: &str = r#"import { mount, signal, Text, View } from "nsh";
import { Switch } from "nsh/ui/desktop";

const on = signal(false);
const tries = signal(0);
mount(() => (
  <View class="items-start gap-2 p-4">
    <Switch label="Wi-Fi" value={on()} onChange={(v) => on.set(v)} />
    <Switch label="Locked" value={false} onChange={() => tries.update((t) => t + 1)} />
    <Text>{`on ${on()} tries ${tries()}`}</Text>
  </View>
));
"#;

#[test]
fn a_switch_springs_its_thumb_when_the_app_turns_it_on() {
    use nsh_core::{AccessibilityRole, Checked};
    let (engine, rx) = start(SWITCHES);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let row = |label: &str| parent_of(&first, text_id(&first, label).expect(label)).unwrap();
    let (wifi, locked) = (row("Wi-Fi"), row("Locked"));
    let track = first_child(&first, wifi).expect("the track");
    assert_eq!(size_of(&first, track), Some((36.0, 20.0)), "md");
    assert_eq!(
        checked_state(&first, wifi),
        Some((AccessibilityRole::Switch, Some(Checked::False)))
    );
    // Off at first, with no animation.
    assert!(springs(std::slice::from_ref(&first)).is_empty());
    let control = engine.control();
    // A switch the app holds off reports the press and stays off.
    control.dispatch_event(locked, EventKind::Press, EventPayload::None);
    let batches = until_text(&rx, "on false tries 1");
    assert!(
        batches
            .iter()
            .any(|b| text_id(b, "on false tries 1").is_some())
    );
    assert!(springs(&batches).is_empty(), "the thumb stays");
    // The app takes the change: the thumb springs across.
    control.dispatch_event(wifi, EventKind::Press, EventPayload::None);
    let batches = until_text(&rx, "on true tries 1");
    assert!(
        batches
            .iter()
            .any(|b| text_id(b, "on true tries 1").is_some())
    );
    assert_eq!(springs(&batches), [1.0]);
    assert!(
        batches
            .iter()
            .any(|b| checked_state(b, wifi).map(|s| s.1) == Some(Some(Checked::True)))
    );
}

/// The inputs `batch` makes, in order.
fn inputs(batch: &MountBatch) -> Vec<u32> {
    batch
        .items
        .iter()
        .filter_map(|i| match i {
            MountItem::Create {
                id,
                kind: nsh_core::ViewKind::Input,
            } => Some(*id),
            _ => None,
        })
        .collect()
}

/// Whether a batch shows `wanted` before nothing comes for a second.
fn shown(rx: &Receiver<MountBatch>, wanted: &str) -> bool {
    until_text(rx, wanted)
        .iter()
        .any(|b| text_id(b, wanted).is_some())
}

const FIELDS: &str = r#"import { mount, signal, Text, View } from "nsh";
import { TextField } from "nsh/ui/desktop";

const name = signal("Ada");
const submitted = signal("");
mount(() => (
  <View class="w-80 gap-2 p-4">
    <TextField
      label="Name"
      value={name()}
      onChange={(t) => name.set(t)}
      onSubmit={(t) => submitted.set(t)}
    />
    <TextField placeholder="Search" size="sm" />
    <Text>{`name ${name()}`}</Text>
    <Text>{`submitted ${submitted()}`}</Text>
  </View>
));
"#;

#[test]
fn a_desktop_text_field_is_dense_and_reports_typing_and_return() {
    let (engine, rx) = start(FIELDS);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let fields = inputs(&first);
    assert_eq!(fields.len(), 2);
    // The labeled one: its box is next to the label, in the field's root.
    let label = text_id(&first, "Name").expect("the label");
    let field_box = |input: u32| parent_of(&first, input).expect("the field's box");
    let (named, search) = match fields[..] {
        [a, b] if parent_of(&first, field_box(a)) == parent_of(&first, label) => (a, b),
        [a, b] => (b, a),
        _ => unreachable!(),
    };
    assert_eq!(
        parent_of(&first, field_box(named)),
        parent_of(&first, label)
    );
    // The boxes around the inputs are the controls' heights.
    let height = |input: u32| size_of(&first, field_box(input)).map(|s| s.1);
    assert_eq!(height(named), Some(32.0), "md");
    assert_eq!(height(search), Some(24.0), "sm");
    let control = engine.control();
    control.dispatch_event(
        named,
        EventKind::ChangeText,
        EventPayload::Text {
            text: "Ada L".into(),
            count: 1,
        },
    );
    assert!(shown(&rx, "name Ada L"), "onChange reached the app");
    control.dispatch_event(
        named,
        EventKind::SubmitEditing,
        EventPayload::Text {
            text: "Ada L".into(),
            count: 1,
        },
    );
    assert!(shown(&rx, "submitted Ada L"), "onSubmit reached the app");
}

const NUMBERS: &str = r#"import { mount, signal, Text, View } from "nsh";
import { NumberInput } from "nsh/ui/desktop";

const n = signal(4);
mount(() => (
  <View class="items-start gap-2 p-4">
    <NumberInput value={n()} onChange={(v) => n.set(v)} min={0} max={5} />
    <Text>{`n ${n()}`}</Text>
  </View>
));
"#;

#[test]
fn a_desktop_number_input_steps_within_its_range_and_reads_what_is_typed() {
    let (engine, rx) = start(NUMBERS);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let field = inputs(&first)[0];
    let field_box = parent_of(&first, field).expect("the field's box");
    assert_eq!(size_of(&first, field_box).map(|s| s.1), Some(32.0));
    let button = |label: &str| parent_of(&first, text_id(&first, label).expect(label)).unwrap();
    let (minus, plus) = (button("−"), button("+"));
    assert_eq!(parent_of(&first, minus), Some(field_box));
    let control = engine.control();
    let press = |button: u32| {
        control.dispatch_event(button, EventKind::PressIn, EventPayload::None);
        control.dispatch_event(button, EventKind::PressOut, EventPayload::None);
    };
    press(plus);
    assert!(shown(&rx, "n 5"), "a step reached the app");
    // At the bound it goes no further.
    press(plus);
    press(minus);
    assert!(shown(&rx, "n 4"));
    // What is typed is read on Return, fitted to the range.
    for kind in [EventKind::ChangeText, EventKind::SubmitEditing] {
        control.dispatch_event(
            field,
            kind,
            EventPayload::Text {
                text: "2.4".into(),
                count: 1,
            },
        );
    }
    assert!(shown(&rx, "n 2"));
    // And when the field loses focus; text that is no number changes
    // nothing.
    for (text, count) in [("abc", 2), ("9", 3)] {
        control.dispatch_event(
            field,
            EventKind::ChangeText,
            EventPayload::Text {
                text: text.into(),
                count,
            },
        );
        control.dispatch_event(field, EventKind::Blur, EventPayload::None);
    }
    assert!(shown(&rx, "n 5"));
}
