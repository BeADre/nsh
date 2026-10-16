//! Apps compiled by nsh's JSX compiler (`"compiler": "nsh"`) end to end:
//! one TSX file importing only `nsh`, bundled as an ES module, run on the
//! native modules through bytecode, tapped after the idle GC.

use std::time::Duration;

use crossbeam_channel::Receiver;
use nsh_core::{ApproxTextEngine, EventKind, EventPayload, MountBatch, MountItem, Size};
use nsh_engine::{Bundle, Engine, EngineConfig};

use crate::ChannelSink;

const APP: &str = r#"import { signal, computed, mount } from "nsh";

function Counter(props: { label: string }) {
  const count = signal(0);
  const double = computed(() => count() * 2);
  return (
    <view testID="counter" onPress={() => count.update((c) => c + 1)}>
      <text>{props.label}: {count()} / {double()}</text>
      <if when={count() > 1} else={<text>few</text>}>
        <text>many</text>
      </if>
    </view>
  );
}

function App() {
  const items = signal([{ id: "a" }, { id: "b" }]);
  return (
    <view>
      <counter label="Taps" />
      <view testID="rotate" onPress={() => items.update((i) => [...i.slice(1), i[0]])}>
        <each of={items()} key={(i) => i.id}>{(item, index) => <text>{index()}{item().id}</text>}</each>
      </view>
    </view>
  );
}

mount(App);
"#;

fn start() -> (Engine, Receiver<MountBatch>) {
    let dir = std::env::temp_dir().join(format!("nsh-compiled-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("nsh.json"), r#"{ "compiler": "nsh" }"#).unwrap();
    std::fs::write(dir.join("App.tsx"), APP).unwrap();
    let bundle = nsh_bundler::bundle(&nsh_bundler::BundleOptions {
        entry: dir.join("App.tsx"),
        minify: true,
    })
    .unwrap();
    assert!(!bundle.code.contains("solid"), "only the app's code");
    let bytecode =
        nsh_js::compile_bytecode_with(&bundle.code, true, &[nsh_ui_mobile::registry()]).unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    let config = EngineConfig {
        // SAFETY: compiled just now by this runtime.
        app: nsh_engine::App::Js(unsafe { Bundle::bytecode(bytecode) }),
        viewport: Size::new(390.0, 844.0),
        trace: false,
        network: None,
        assets: None,
        environment: Default::default(),
        header_heights: (44.0, 96.0),
        limits: nsh_engine::Limits::default(),
        components: Default::default(),
        runtime: None,
    };
    (
        Engine::start(config, || ApproxTextEngine, ChannelSink(tx)).unwrap(),
        rx,
    )
}

fn texts(batch: &MountBatch) -> Vec<String> {
    batch
        .items
        .iter()
        .filter_map(|i| match i {
            MountItem::Text { text, .. } => Some(text.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_single_file_app_on_the_native_modules() {
    let (engine, rx) = start();
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let mut shown = texts(&first);
    shown.sort();
    assert_eq!(shown, ["0a", "1b", "Taps: 0 / 0", "few"]);
    let mut pressable: Vec<_> = first
        .items
        .iter()
        .filter_map(|i| match i {
            MountItem::Events { id, events } if events.contains(EventKind::Press) => Some(*id),
            _ => None,
        })
        .collect();
    // Static children are made before dynamic ones: the rotating view
    // comes before the counter component's.
    pressable.sort();
    let [rotate, counter] = pressable[..] else {
        panic!("{pressable:?}")
    };

    // The idle collector runs: the app stays reachable through its views.
    while rx.recv_timeout(Duration::from_millis(1300)).is_ok() {}

    let control = engine.control();
    let tap = |id| {
        control.dispatch_event(id, EventKind::Press, EventPayload::None);
        rx.recv_timeout(Duration::from_secs(5)).expect("an update")
    };
    assert_eq!(texts(&tap(counter)), ["Taps: 1 / 2"]);
    let second = tap(counter);
    assert!(
        texts(&second).contains(&"many".to_owned()),
        "{:?}",
        texts(&second)
    );
    // Rows keep their views and show their new index.
    let rotated = tap(rotate);
    assert!(
        !rotated
            .items
            .iter()
            .any(|i| matches!(i, MountItem::Create { .. })),
        "{:?}",
        rotated.items
    );
    let mut t = texts(&rotated);
    t.sort();
    assert_eq!(t, ["0b", "1a"]);
}
