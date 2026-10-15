//! Rust components through the generated JS glue: props, events, children,
//! named slots, children placed one by one, and a list of the app's rows;
//! modules (`Alert`, `StatusBar`) and the environment (`SafeAreaView`,
//! `KeyboardAvoidingView`).

use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use nsh_core::{
    ApproxTextEngine, Command, DialogButtonStyle, EventKind, EventPayload, MountBatch, MountItem,
    PayloadValue, ROOT_ID, Size, StatusBarStyle,
};
use nsh_engine::{Bundle, Engine, EngineConfig, Environment, Insets, MountSink};

struct ChannelSink(Sender<MountBatch>);

impl MountSink for ChannelSink {
    fn submit(&self, batch: MountBatch) {
        let _ = self.0.send(batch);
    }
}

const APP: &str = r#"import { createSignal, Show } from "solid-js";
import { render, root } from "nsh";
import { Button, Card } from "nsh/ui/mobile";

function App() {
  const [titled, setTitled] = createSignal(true);
  return (
    <Card
      header={<Show when={titled()}><text>Title</text></Show>}
      footer={<Button title="Hide title" onPress={() => setTitled(false)} />}
    >
      <text>Body</text>
    </Card>
  );
}

render(() => <App />, root);
"#;

fn start(app: &str) -> (Engine, Receiver<MountBatch>) {
    start_in(app, Environment::default())
}

fn start_in(app: &str, environment: Environment) -> (Engine, Receiver<MountBatch>) {
    static APPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "nsh-components-{}-{}",
        std::process::id(),
        APPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let entry = dir.join("App.tsx");
    std::fs::write(&entry, app).unwrap();
    let bundle = nsh_bundler::bundle(&nsh_bundler::BundleOptions {
        entry,
        minify: false,
    })
    .unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    let config = EngineConfig {
        bundle: Bundle::source(bundle.code),
        viewport: Size::new(390.0, 844.0),
        trace: false,
        network: None,
        assets: None,
        environment,
        header_heights: (44.0, 96.0),
        limits: nsh_engine::Limits::default(),
        components: nsh_ui_mobile::registry(),
        runtime: None,
    };
    (
        Engine::start(config, || ApproxTextEngine, ChannelSink(tx)).unwrap(),
        rx,
    )
}

fn text_ids(batch: &MountBatch, wanted: &str) -> Option<u32> {
    batch.items.iter().find_map(|i| match i {
        MountItem::Text { id, text, .. } if text.text == wanted => Some(*id),
        _ => None,
    })
}

#[test]
fn slots_children_and_events_through_the_glue() {
    let (engine, rx) = start(APP);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    for wanted in ["Title", "Body", "Hide title"] {
        assert!(text_ids(&first, wanted).is_some(), "{wanted} rendered");
    }
    // The title (header slot) is above the body (the children).
    let frame = |id| {
        first.items.iter().find_map(|i| match i {
            MountItem::Frame { id: f, frame } if *f == id => Some(*frame),
            _ => None,
        })
    };
    let title_parent = first.items.iter().find_map(|i| match i {
        MountItem::Insert { parent, child, .. } if Some(*child) == text_ids(&first, "Title") => {
            Some(*parent)
        }
        _ => None,
    });
    let body_parent = first.items.iter().find_map(|i| match i {
        MountItem::Insert { parent, child, .. } if Some(*child) == text_ids(&first, "Body") => {
            Some(*parent)
        }
        _ => None,
    });
    let (title_parent, body_parent) = (title_parent.unwrap(), body_parent.unwrap());
    assert!(frame(title_parent).unwrap().y < frame(body_parent).unwrap().y);
    // The Button's host is the one with a press listener.
    let button = first
        .items
        .iter()
        .find_map(|i| match i {
            MountItem::Events { id, events } if events.contains(EventKind::Press) => Some(*id),
            _ => None,
        })
        .expect("a pressable button");

    // Its press hides the title: the header slot empties and is not rendered.
    engine
        .control()
        .dispatch_event(button, EventKind::Press, EventPayload::None);
    let batch = rx.recv_timeout(Duration::from_secs(5)).expect("a batch");
    assert!(
        batch
            .items
            .iter()
            .any(|i| matches!(i, MountItem::Remove { id } if *id == title_parent))
    );
}

const ROWS: &str = r#"import { render, root } from "nsh";
import { Group, ListView } from "nsh/ui/mobile";

const rows = Array.from({ length: 100 }, (_, i) => ({ id: i, label: `Row ${i}` }));

render(
  () => (
    <ListView
      each={rows}
      ListHeaderComponent={() => (
        <Group>
          <text>First</text>
          <text>Second</text>
        </Group>
      )}
    >
      {(row) => <text>{row().label}</text>}
    </ListView>
  ),
  root,
);
"#;

#[test]
fn components_place_the_apps_children_and_rows() {
    let (_engine, rx) = start(ROWS);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let parent_of = |id| {
        first.items.iter().find_map(|i| match i {
            MountItem::Insert { parent, child, .. } if *child == id => Some(*parent),
            _ => None,
        })
    };
    let frame = |id| {
        first.items.iter().find_map(|i| match i {
            MountItem::Frame { id: f, frame } if *f == id => Some(*frame),
            _ => None,
        })
    };
    let top_border = |id| {
        first.items.iter().any(|i| {
            matches!(i, MountItem::ViewStyle { id: v, style } if *v == id && style.borders.widths[0] == 1.0)
        })
    };
    // Each child of the Group in a wrapper of the core's (odd ids), with
    // a divider above all but the first.
    let a = parent_of(text_ids(&first, "First").expect("First")).unwrap();
    let b = parent_of(text_ids(&first, "Second").expect("Second")).unwrap();
    assert!(a % 2 == 1 && b % 2 == 1 && a != b);
    assert!(!top_border(a) && top_border(b));
    // The rows the app renders, recycled by the list, wrapped by the
    // ListView: 16pt in, dividers between them.
    let row0 = parent_of(text_ids(&first, "Row 0").expect("Row 0")).unwrap();
    let row1 = parent_of(text_ids(&first, "Row 1").expect("Row 1")).unwrap();
    assert!(row0 % 2 == 1 && row1 % 2 == 1);
    assert_eq!(
        frame(text_ids(&first, "Row 0").unwrap()).map(|f| f.x),
        Some(16.0)
    );
    assert!(!top_border(row0) && top_border(row1));
    assert!(
        text_ids(&first, "Row 99").is_none(),
        "only rows near the viewport exist"
    );
}

const INSET: &str = r#"import { createSignal } from "solid-js";
import { render, root } from "nsh";
import { ListView } from "nsh/ui/mobile";

const [rows, setRows] = createSignal(["A", "B", "C"]);
setTimeout(() => setRows([...rows(), "D"]), 0);

render(() => <ListView variant="inset" each={rows()}>{(row) => <text>{row()}</text>}</ListView>, root);
"#;

#[test]
fn rows_know_whether_they_are_last() {
    let (_engine, rx) = start(INSET);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let parent_of = |batch: &MountBatch, id| {
        batch.items.iter().find_map(|i| match i {
            MountItem::Insert { parent, child, .. } if *child == id => Some(*parent),
            _ => None,
        })
    };
    let corners = |batch: &MountBatch, id| {
        batch.items.iter().find_map(|i| match i {
            MountItem::ViewStyle { id: v, style } if *v == id => Some(style.corner_radius),
            _ => None,
        })
    };
    let rounded = |c: [Option<f32>; 4]| c.iter().map(|r| r.is_some()).collect::<Vec<_>>();
    let wrapper = |name| parent_of(&first, text_ids(&first, name).unwrap()).unwrap();
    let (a, b, c) = (wrapper("A"), wrapper("B"), wrapper("C"));
    // The first row rounds its top corners, the last its bottom ones.
    assert_eq!(
        corners(&first, a).map(rounded),
        Some(vec![true, true, false, false])
    );
    assert_eq!(
        corners(&first, b).map(rounded),
        Some(vec![false, false, false, false])
    );
    assert_eq!(
        corners(&first, c).map(rounded),
        Some(vec![false, false, true, true])
    );

    // A row appended: C is no longer the last, D is.
    let next = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("the new row");
    let d = parent_of(&next, text_ids(&next, "D").expect("D")).unwrap();
    assert_eq!(
        corners(&next, c).map(rounded),
        Some(vec![false, false, false, false])
    );
    assert_eq!(
        corners(&next, d).map(rounded),
        Some(vec![false, false, true, true])
    );
}

const STEPPER: &str = r#"import { createSignal } from "solid-js";
import { render, root } from "nsh";
import { Stepper } from "nsh/ui/mobile";

const [changes, setChanges] = createSignal(0);
render(
  () => (
    <view>
      <Stepper defaultValue={5} min={0} max={100} onChange={() => setChanges(changes() + 1)} />
      <text>{`changes ${changes()}`}</text>
    </view>
  ),
  root,
);
"#;

#[test]
fn a_stateful_component_steps_while_held() {
    let (engine, rx) = start(STEPPER);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    assert!(text_ids(&first, "5").is_some(), "the default value");
    // The + button: the second view listening to presses.
    let plus = first
        .items
        .iter()
        .filter_map(|i| match i {
            MountItem::Events { id, events } if events.contains(EventKind::PressIn) => Some(*id),
            _ => None,
        })
        .nth(1)
        .expect("the + button");
    let control = engine.control();
    control.dispatch_event(plus, EventKind::PressIn, EventPayload::None);
    // One step at once, more after the hold (400ms, then every 80ms).
    std::thread::sleep(Duration::from_millis(700));
    control.dispatch_event(plus, EventKind::PressOut, EventPayload::None);
    std::thread::sleep(Duration::from_millis(200));
    let mut shown = Vec::new();
    while let Ok(batch) = rx.recv_timeout(Duration::from_millis(100)) {
        for item in batch.items {
            if let MountItem::Text { text, .. } = item {
                shown.push(text.text.clone());
            }
        }
    }
    let value: f64 = shown
        .iter()
        .rev()
        .find_map(|t| t.parse().ok())
        .expect("the value changed");
    assert!(value >= 8.0, "held: {value}");
    let changes: u32 = shown
        .iter()
        .rev()
        .find_map(|t| t.strip_prefix("changes ")?.parse().ok())
        .unwrap();
    assert_eq!(changes as f64, value - 5.0, "one onChange per step");
    // Released: no more steps.
    assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
}

const METHODS: &str = r#"import { createSignal } from "solid-js";
import { render, root } from "nsh";
import { Stepper, type StepperRef } from "nsh/ui/mobile";

const [shown, setShown] = createSignal("waiting");
let stepper!: StepperRef;
setTimeout(async () => {
  stepper.setValue(42.4);
  const value = await stepper.value();
  setShown(`got ${value}`);
}, 0);

render(
  () => (
    <view>
      <Stepper ref={stepper} defaultValue={1} max={50} />
      <text>{shown()}</text>
    </view>
  ),
  root,
);
"#;

#[test]
fn js_calls_a_components_methods() {
    let (_engine, rx) = start(METHODS);
    let mut shown = Vec::new();
    while let Ok(batch) = rx.recv_timeout(Duration::from_millis(500)) {
        for item in batch.items {
            if let MountItem::Text { text, .. } = item {
                shown.push(text.text.clone());
            }
        }
        if shown.iter().any(|t| t.starts_with("got")) {
            break;
        }
    }
    // The call runs in order with the ops before it; the one JS waits for
    // resolves with the component's answer.
    assert!(shown.contains(&"42".to_owned()), "{shown:?}");
    assert!(shown.contains(&"got 42".to_owned()), "{shown:?}");
}

/// The batches until `done` holds for one (or nothing came for 500ms).
fn until(rx: &Receiver<MountBatch>, mut done: impl FnMut(&MountBatch) -> bool) -> Vec<MountBatch> {
    let mut batches = Vec::new();
    while let Ok(batch) = rx.recv_timeout(Duration::from_millis(500)) {
        let stop = done(&batch);
        batches.push(batch);
        if stop {
            break;
        }
    }
    batches
}

fn shows(batch: &MountBatch, wanted: &str) -> bool {
    text_ids(batch, wanted).is_some()
}

const ALERT: &str = r#"import { createSignal } from "solid-js";
import { render, root } from "nsh";
import { Alert } from "nsh/ui/mobile";

const [shown, setShown] = createSignal("waiting");
setTimeout(async () => {
  const i = await Alert.alert("Delete the photo?", "It is gone then.", [
    { text: "Cancel", style: "cancel" },
    { text: "Delete", style: "destructive" },
  ], { cancelable: true });
  setShown(`answer ${i}`);
}, 0);

render(() => <text>{shown()}</text>, root);
"#;

#[test]
fn an_alert_resolves_with_the_button_pressed() {
    let (engine, rx) = start(ALERT);
    let mut dialog = None;
    until(&rx, |batch| {
        dialog = dialog.take().or_else(|| {
            batch.items.iter().find_map(|i| match i {
                MountItem::Command {
                    id: ROOT_ID,
                    command: Command::Dialog(d),
                } => Some(d.clone()),
                _ => None,
            })
        });
        dialog.is_some()
    });
    let dialog = dialog.expect("the alert");
    assert_eq!(dialog.title.as_deref(), Some("Delete the photo?"));
    assert_eq!(dialog.message.as_deref(), Some("It is gone then."));
    assert!(dialog.cancelable && !dialog.sheet);
    let looks: Vec<_> = dialog
        .buttons
        .iter()
        .map(|b| (b.text.as_str(), b.style))
        .collect();
    assert_eq!(
        looks,
        [
            ("Cancel", DialogButtonStyle::Cancel),
            ("Delete", DialogButtonStyle::Destructive)
        ]
    );
    engine.control().dispatch_event(
        ROOT_ID,
        EventKind::Dialog,
        EventPayload::Map(vec![
            ("id", PayloadValue::Number(dialog.id as f64)),
            ("index", PayloadValue::Number(1.0)),
        ]),
    );
    let batches = until(&rx, |b| shows(b, "answer 1"));
    assert!(batches.iter().any(|b| shows(b, "answer 1")));
}

const STATUS_BAR: &str = r##"import { createSignal, Show } from "solid-js";
import { render, root } from "nsh";
import { setStatusBarBackgroundColor } from "nsh/android";
import { StatusBar } from "nsh/ui/mobile";

const [rendered, setRendered] = createSignal(true);
setStatusBarBackgroundColor("#ff0000");
StatusBar.setBarStyle("dark-content");
setTimeout(() => StatusBar.setHidden(true), 50);
setTimeout(() => setRendered(false), 100);

render(
  () => (
    <Show when={rendered()}>
      <StatusBar barStyle="light-content" android={{ backgroundColor: "#00ff00" }} />
    </Show>
  ),
  root,
);
"##;

#[test]
fn rendered_status_bars_win_over_what_functions_set() {
    let (_engine, rx) = start(STATUS_BAR);
    let mut configs = Vec::new();
    until(&rx, |batch| {
        for item in &batch.items {
            if let MountItem::Command {
                id: ROOT_ID,
                command: Command::StatusBar(c),
            } = item
            {
                configs.push(*c);
            }
        }
        configs.len() >= 3
    });
    let red = nsh_core::Color::parse("#ff0000");
    let green = nsh_core::Color::parse("#00ff00");
    let seen: Vec<_> = configs
        .iter()
        .map(|c| (c.style, c.hidden, c.background))
        .collect();
    // One per turn: the functions' and the rendered setting merged, then
    // the hidden bar, then the functions' alone.
    assert_eq!(
        seen,
        [
            (StatusBarStyle::Light, false, green),
            (StatusBarStyle::Light, true, green),
            (StatusBarStyle::Dark, true, red),
        ]
    );
}

const SAFE_AREA: &str = r#"import { render, root } from "nsh";
import { KeyboardAvoidingView, SafeAreaView } from "nsh/ui/mobile";

render(
  () => (
    <KeyboardAvoidingView style={{ flex: 1 }}>
      <SafeAreaView style={{ flex: 1, padding: 4 }} edges={["top", "bottom"]}>
        <text>inside</text>
      </SafeAreaView>
    </KeyboardAvoidingView>
  ),
  root,
);
"#;

fn frame_of(batches: &[MountBatch], id: u32) -> Option<nsh_core::Rect> {
    batches.iter().rev().find_map(|b| {
        b.items.iter().rev().find_map(|i| match i {
            MountItem::Frame { id: f, frame } if *f == id => Some(*frame),
            _ => None,
        })
    })
}

#[test]
fn safe_area_and_keyboard_avoiding_views_pad_by_the_environment() {
    let environment = Environment {
        window: (390.0, 844.0),
        safe_area: Insets {
            top: 47.0,
            right: 10.0,
            bottom: 34.0,
            left: 10.0,
        },
        ..Default::default()
    };
    let (engine, rx) = start_in(SAFE_AREA, environment);
    let first = until(&rx, |b| shows(b, "inside"));
    let text = first
        .iter()
        .find_map(|b| text_ids(b, "inside"))
        .expect("the text");
    // Its own padding, plus the edges it pads (not left and right).
    let frame = frame_of(&first, text).expect("laid out");
    assert_eq!((frame.x, frame.y), (4.0, 51.0));
    // The keyboard: the avoiding view pads its bottom, the safe area view
    // shrinks, the text stays.
    engine.control().set_environment(Environment {
        keyboard_height: 300.0,
        ..environment
    });
    let after = until(&rx, |b| {
        b.items
            .iter()
            .any(|i| matches!(i, MountItem::Frame { frame, .. } if frame.height == 844.0 - 300.0))
    });
    assert!(
        after
            .iter()
            .flat_map(|b| &b.items)
            .any(|i| matches!(i, MountItem::Frame { frame, .. } if frame.height == 844.0 - 300.0)),
        "the safe area view above the keyboard"
    );
}

const RUNTIME_MODULES: &str = r#"import { createSignal } from "solid-js";
import { render, root, type YNode } from "nsh";
import { AccessibilityInfo } from "nsh/accessibility";
import { Toast } from "nsh/android";
import { Image } from "nsh/image";

const [shown, setShown] = createSignal("waiting");
let note!: YNode;
setTimeout(async () => {
  Toast.show("hello", "long");
  AccessibilityInfo.setAccessibilityFocus(note);
  const size = await Image.getSize({ uri: "https://example.com/a.png" });
  let failed = "no";
  try {
    await Image.getSize("https://example.com/missing.png");
  } catch (e) {
    failed = String((e as Error).message);
  }
  setShown(`${size.width}x${size.height} ${failed}`);
}, 0);

render(() => <text ref={note}>{shown()}</text>, root);
"#;

#[test]
fn runtime_modules_are_rust() {
    let (engine, rx) = start(RUNTIME_MODULES);
    let note = rx
        .recv_timeout(Duration::from_secs(5))
        .ok()
        .and_then(|b| text_ids(&b, "waiting"))
        .expect("first frame");
    let mut toast = None;
    let mut focused = None;
    let mut requests = Vec::new();
    let control = engine.control();
    until(&rx, |batch| {
        for item in &batch.items {
            match item {
                MountItem::Command {
                    id: ROOT_ID,
                    command: Command::Toast { text, long },
                } => toast = Some((text.clone(), *long)),
                MountItem::Command {
                    id,
                    command: Command::AccessibilityFocus,
                } => focused = Some(*id),
                MountItem::Command {
                    id: ROOT_ID,
                    command: Command::ImageInfo { id, uri },
                } => {
                    requests.push(uri.clone());
                    // The first one has a size, the second fails.
                    let payload = if requests.len() == 1 {
                        vec![
                            ("id", PayloadValue::Number(*id as f64)),
                            ("width", PayloadValue::Number(640.0)),
                            ("height", PayloadValue::Number(480.0)),
                        ]
                    } else {
                        vec![
                            ("id", PayloadValue::Number(*id as f64)),
                            ("error", PayloadValue::Str("not found".into())),
                        ]
                    };
                    control.dispatch_event(
                        ROOT_ID,
                        EventKind::ImageInfo,
                        EventPayload::Map(payload),
                    );
                }
                _ => {}
            }
        }
        shows(batch, "640x480 not found")
    });
    assert_eq!(toast, Some(("hello".to_owned(), true)));
    // The node JS passed, by id.
    assert_eq!(focused, Some(note));
    assert_eq!(
        requests,
        [
            "https://example.com/a.png",
            "https://example.com/missing.png"
        ]
    );
}

const BACK: &str = r#"import { createSignal } from "solid-js";
import { render, root } from "nsh";
import { BackHandler } from "nsh/android";

const [presses, setPresses] = createSignal(0);
BackHandler.addEventListener("hardwareBackPress", () => {
  setPresses(presses() + 1);
  // The first press is handled, the second one leaves the app.
  return presses() === 1;
});

render(() => <text>{`presses ${presses()}`}</text>, root);
"#;

#[test]
fn unhandled_back_presses_exit_through_rust() {
    let (engine, rx) = start(BACK);
    until(&rx, |b| shows(b, "presses 0"));
    let exits = |batches: &[MountBatch]| {
        batches
            .iter()
            .flat_map(|b| &b.items)
            .filter(|i| {
                matches!(
                    i,
                    MountItem::Command {
                        id: ROOT_ID,
                        command: Command::ExitApp
                    }
                )
            })
            .count()
    };
    let control = engine.control();
    control.dispatch_event(ROOT_ID, EventKind::BackPress, EventPayload::None);
    let first = until(&rx, |b| shows(b, "presses 1"));
    assert_eq!(exits(&first), 0, "handled");
    control.dispatch_event(ROOT_ID, EventKind::BackPress, EventPayload::None);
    let second = until(&rx, |b| exits(std::slice::from_ref(b)) > 0);
    assert_eq!(exits(&second), 1, "not handled: the app exits");
}
