//! Apps end to end:
//! one TSX file importing only `nsh`, bundled as an ES module, run on the
//! native modules through bytecode, tapped after the idle GC.

use std::time::Duration;

use crossbeam_channel::Receiver;
use nsh_core::{ApproxTextEngine, EventKind, EventPayload, MountBatch, MountItem, Size};
use nsh_engine::{Bundle, Engine, EngineConfig};

use crate::ChannelSink;

const APP: &str = r#"import { computed, mount, signal, Text, View } from "nsh";

function Counter(props: { label: string }) {
  const count = signal(0);
  const double = computed(() => count() * 2);
  return (
    <View testID="counter" onPress={() => count.update((c) => c + 1)}>
      <Text>{props.label}: {count()} / {double()}</Text>
      <If when={count() > 1} else={<Text>few</Text>}>
        <Text>many</Text>
      </If>
    </View>
  );
}

function App() {
  const items = signal([{ id: "a" }, { id: "b" }]);
  return (
    <View>
      <Counter label="Taps" />
      <View testID="rotate" onPress={() => items.update((i) => [...i.slice(1), i[0]])}>
        <Each of={items()} key={(i) => i.id}>{(item, index) => <Text>{index()}{item().id}</Text>}</Each>
      </View>
    </View>
  );
}

mount(App);
"#;

fn start() -> (Engine, Receiver<MountBatch>) {
    let dir = std::env::temp_dir().join(format!("nsh-compiled-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("App.tsx"), APP).unwrap();
    let bundle = nsh_bundler::bundle(&nsh_bundler::BundleOptions {
        entry: dir.join("App.tsx"),
        minify: true,
        split: false,
        ui_sets: vec![nsh_base::registry()],
    })
    .unwrap();
    assert!(!bundle.code.contains("solid"), "only the app's code");
    let bytecode =
        nsh_js::compile_bytecode_with(&bundle.code, true, &[nsh_base::registry()]).unwrap();
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
        ui_sets: Vec::new(),
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

const NAVIGATION: &str = r#"import { mount, signal, Text, View } from "nsh";
import { Dialog, NavPage, NavStack, createNavStack, safeAreaInsets } from "nsh/base";

type Route = { name: string };
const nav = createNavStack<Route>([{ name: "home" }]);
const open = signal(false);

function Insets() {
  const insets = safeAreaInsets();
  return <Text>{`inset ${insets().top}`}</Text>;
}

mount(() => (
  <View style={{ flex: 1 }}>
    <NavStack state={nav} style={{ flex: 1 }}>
      {(route) => (
        <NavPage title={route.name} headerRight={<Text onPress={() => open.set(true)}>open</Text>}>
          <View>
            <Text onPress={() => nav.push({ name: `page${nav.routes().length}` })}>push {route.name}</Text>
            <Insets />
          </View>
        </NavPage>
      )}
    </NavStack>
    <Dialog open={open()} onOpenChange={(o) => open.set(o)}>
      <Text>in modal</Text>
    </Dialog>
  </View>
));
"#;

fn start_app(app: &str) -> (Engine, Receiver<MountBatch>) {
    start_app_with(app, (44.0, 96.0))
}

/// [`start_app`] on a platform whose screen headers are `header_heights`
/// ([`nsh_core::NO_NATIVE_HEADER`]: none, as on the desktops and the web).
fn start_app_with(app: &str, header_heights: (f32, f32)) -> (Engine, Receiver<MountBatch>) {
    let dir = std::env::temp_dir().join(format!(
        "nsh-compiled-nav-{}-{}-{}",
        std::process::id(),
        app.len(),
        header_heights.0
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("App.tsx"), app).unwrap();
    let bundle = nsh_bundler::bundle(&nsh_bundler::BundleOptions {
        entry: dir.join("App.tsx"),
        minify: false,
        split: false,
        ui_sets: vec![nsh_base::registry()],
    })
    .unwrap();
    let bytecode =
        nsh_js::compile_bytecode_with(&bundle.code, false, &[nsh_base::registry()]).unwrap();
    let (tx, rx) = crossbeam_channel::unbounded();
    let config = EngineConfig {
        // SAFETY: compiled just now by this runtime.
        app: nsh_engine::App::Js(unsafe { Bundle::bytecode(bytecode) }),
        viewport: Size::new(390.0, 844.0),
        trace: false,
        network: None,
        assets: None,
        environment: Default::default(),
        header_heights,
        limits: nsh_engine::Limits::default(),
        ui_sets: vec![nsh_base::registry()],
        runtime: None,
    };
    (
        Engine::start(config, || ApproxTextEngine, ChannelSink(tx)).unwrap(),
        rx,
    )
}

/// The next batch `wanted` accepts (releases of popped screens' views may
/// come in batches of their own first).
fn next(rx: &Receiver<MountBatch>, wanted: impl Fn(&MountBatch) -> bool) -> MountBatch {
    loop {
        let batch = rx.recv_timeout(Duration::from_secs(5)).expect("a batch");
        if wanted(&batch) {
            return batch;
        }
    }
}

fn text_id(batch: &MountBatch, wanted: &str) -> Option<u32> {
    batch.items.iter().find_map(|i| match i {
        MountItem::Text { id, text, .. } if text.text == wanted => Some(*id),
        _ => None,
    })
}

fn titles(batch: &MountBatch) -> Vec<(u32, String)> {
    batch
        .items
        .iter()
        .filter_map(|i| match i {
            MountItem::ScreenConfig { id, config } => {
                Some((*id, config.title.as_deref()?.to_owned()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn navigation_stacks_screens_and_modals() {
    let (engine, rx) = start_app(NAVIGATION);
    let control = engine.control();
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let home = titles(&first);
    assert_eq!(home.last().map(|t| t.1.as_str()), Some("home"), "{home:?}");
    let home_screen = home[0].0;
    assert!(text_id(&first, "inset 0").is_some(), "{:?}", texts(&first));
    // The screen reports its insets: what it renders sees them.
    let insets = |top: f64| {
        EventPayload::Map(vec![
            ("top", nsh_core::PayloadValue::Number(top)),
            ("right", nsh_core::PayloadValue::Number(0.0)),
            ("bottom", nsh_core::PayloadValue::Number(34.0)),
            ("left", nsh_core::PayloadValue::Number(0.0)),
        ])
    };
    control.dispatch_event(home_screen, EventKind::Insets, insets(100.0));
    let batch = rx.recv_timeout(Duration::from_secs(5)).expect("insets");
    assert!(
        texts(&batch).contains(&"inset 100".to_owned()),
        "{:?}",
        texts(&batch)
    );
    // Push: a second screen.
    let push = text_id(&first, "push home").unwrap();
    control.dispatch_event(push, EventKind::Press, EventPayload::None);
    let batch = rx.recv_timeout(Duration::from_secs(5)).expect("pushed");
    let page = titles(&batch);
    assert_eq!(page.last().map(|t| t.1.as_str()), Some("page1"), "{page:?}");
    let page_screen = page[0].0;
    // The user pops it natively: its route goes, so the next push is page1
    // again.
    control.dispatch_event(page_screen, EventKind::Dismiss, EventPayload::None);
    while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}
    control.dispatch_event(push, EventKind::Press, EventPayload::None);
    let batch = next(&rx, |b| !titles(b).is_empty());
    assert_eq!(titles(&batch).last().map(|t| t.1.as_str()), Some("page1"));
    // The header's button opens the modal.
    let open = text_id(&first, "open").expect("a header item");
    control.dispatch_event(open, EventKind::Press, EventPayload::None);
    let batch = next(&rx, |b| {
        b.items.iter().any(|i| {
            matches!(
                i,
                MountItem::Create {
                    kind: nsh_core::ViewKind::Modal,
                    ..
                }
            )
        })
    });
    assert!(text_id(&batch, "in modal").is_some());
}

const MODAL_DISMISS: &str = r#"import { mount, signal, Text, View } from "nsh";
import { Dialog } from "nsh/base";

const open = signal(true);
const status = signal("waiting");

mount(() => (
  <View style={{ flex: 1 }}>
    <Text>{status()}</Text>
    <Dialog open={open()} onCancel={() => open.set(false)} closeOnEscape={false} onClose={() => status.set("dismissed")}>
      <Text>in modal</Text>
    </Dialog>
  </View>
));
"#;

/// An app removes its modal when asked to close it; the platform reports
/// the dismissal once the modal has gone, and the app still hears it.
#[test]
fn removed_modal_hears_its_dismissal() {
    let (engine, rx) = start_app(MODAL_DISMISS);
    let control = engine.control();
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let modal = first
        .items
        .iter()
        .find_map(|i| match i {
            MountItem::Create {
                id,
                kind: nsh_core::ViewKind::Modal,
                ..
            } => Some(*id),
            _ => None,
        })
        .expect("a modal");
    control.dispatch_event(modal, EventKind::RequestClose, EventPayload::None);
    next(&rx, |b| {
        b.items
            .iter()
            .any(|i| matches!(i, MountItem::Remove { id } if *id == modal))
    });
    control.dispatch_event(modal, EventKind::Dismiss, EventPayload::None);
    let batch = next(&rx, |b| text_id(b, "dismissed").is_some());
    assert!(text_id(&batch, "dismissed").is_some());
}

const SAME_ROUTE_TWICE: &str = r#"import { mount, Text, View } from "nsh";
import { NavPage, NavStack, createNavStack } from "nsh/base";

const nav = createNavStack(["home"]);

mount(() => (
  <View style={{ flex: 1 }}>
    <NavStack state={nav} style={{ flex: 1 }}>
      {(route, index) => (
        <NavPage title={`${route} ${index()}`}>
          <Text onPress={() => nav.push("detail")}>{`push from ${index()}`}</Text>
          <Text>{`depth ${nav.routes().length}`}</Text>
        </NavPage>
      )}
    </NavStack>
  </View>
));
"#;

/// A route pushed twice is two screens: the user popping the top one
/// natively removes that one only.
#[test]
fn a_route_pushed_twice_is_two_screens() {
    let (engine, rx) = start_app(SAME_ROUTE_TWICE);
    let control = engine.control();
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let push = text_id(&first, "push from 0").unwrap();
    control.dispatch_event(push, EventKind::Press, EventPayload::None);
    let batch = next(&rx, |b| !titles(b).is_empty());
    assert_eq!(
        titles(&batch).last().map(|t| t.1.as_str()),
        Some("detail 1")
    );
    while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}
    control.dispatch_event(push, EventKind::Press, EventPayload::None);
    let batch = next(&rx, |b| !titles(b).is_empty());
    let top = titles(&batch);
    assert_eq!(
        top.last().map(|t| t.1.as_str()),
        Some("detail 2"),
        "{top:?}"
    );
    let top_screen = top[0].0;
    while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}
    // The user pops the top "detail": the other one stays.
    control.dispatch_event(top_screen, EventKind::Dismiss, EventPayload::None);
    let batch = next(&rx, |b| texts(b).iter().any(|t| t.starts_with("depth ")));
    assert!(
        texts(&batch).contains(&"depth 2".to_owned()),
        "{:?}",
        texts(&batch)
    );
}

const DIALOG_EVENTS: &str = r#"import { mount, signal, Text, View } from "nsh";
import { Dialog } from "nsh/base";

const log = signal<string[]>([]);
const add = (s: string) => log.update((l) => [...l, s]);
const open = signal(true);
const swipe = signal(true);

mount(() => (
  <View style={{ flex: 1 }}>
    <Text>{`log ${log().join(",")}`}</Text>
    <Text onPress={() => { swipe.set(false); open.set(true); }}>again</Text>
    <Dialog
      open={open()}
      presentation="pageSheet"
      animation="fade"
      closeOnSwipe={swipe()}
      closeOnBackdropPress={false}
      backdropClass="bg-black/50"
      onOpen={() => add("open")}
      onCancel={() => add("cancel")}
      onOpenChange={(o, reason) => { add(`change ${o} ${reason}`); open.set(o); }}
    >
      <Text>in dialog</Text>
    </Dialog>
  </View>
));
"#;

fn modal_of(batch: &MountBatch) -> Option<u32> {
    batch.items.iter().find_map(|i| match i {
        MountItem::Create {
            id,
            kind: nsh_core::ViewKind::Modal,
            ..
        } => Some(*id),
        _ => None,
    })
}

fn log_of(batch: &MountBatch) -> Option<String> {
    texts(batch).into_iter().find(|t| t.starts_with("log "))
}

/// A dialog reports the user's attempts (`onCancel`) and, when its
/// `closeOn*` flag let it close natively, `onOpenChange(false, reason)`.
#[test]
fn dialogs_report_cancels_and_closes_with_their_reason() {
    let (engine, rx) = start_app(DIALOG_EVENTS);
    let control = engine.control();
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let modal = modal_of(&first).expect("a dialog");
    let config = first
        .items
        .iter()
        .find_map(|i| match i {
            MountItem::ModalConfig { id, config } if *id == modal => Some(*config),
            _ => None,
        })
        .expect("its config");
    assert!(config.close_on_escape && config.close_on_swipe);
    assert!(!config.close_on_backdrop_press);
    assert_eq!(config.presentation, nsh_core::ModalPresentation::PageSheet);
    assert_eq!(config.animation, nsh_core::ModalAnimation::Fade);
    // The backdrop part: a view of the dialog with the app's classes.
    let backdrop = backdrop_of(&first, modal).expect("a backdrop");
    control.dispatch_event(modal, EventKind::Show, EventPayload::None);
    next(&rx, |b| log_of(b).as_deref() == Some("log open"));
    let reason =
        |r: &str| EventPayload::Map(vec![("reason", nsh_core::PayloadValue::Str(r.to_owned()))]);
    // A backdrop press: its flag is false, it only asks.
    control.dispatch_event(backdrop, EventKind::Press, EventPayload::None);
    next(&rx, |b| log_of(b).as_deref() == Some("log open,cancel"));
    // A swipe closes it.
    control.dispatch_event(modal, EventKind::RequestClose, reason("swipe"));
    next(&rx, |b| {
        log_of(b).as_deref() == Some("log open,cancel,cancel,change false swipe")
    });
    // Open again without swiping: no reason is Escape's.
    let again = text_id(&first, "again").unwrap();
    control.dispatch_event(again, EventKind::Press, EventPayload::None);
    let batch = next(&rx, |b| modal_of(b).is_some());
    let modal = modal_of(&batch).unwrap();
    control.dispatch_event(modal, EventKind::RequestClose, reason("swipe"));
    next(&rx, |b| {
        log_of(b).as_deref() == Some("log open,cancel,cancel,change false swipe,cancel")
    });
    control.dispatch_event(modal, EventKind::RequestClose, EventPayload::None);
    next(&rx, |b| {
        log_of(b).is_some_and(|l| l.ends_with(",cancel,cancel,change false cancel"))
    });
}

/// The children `Insert`ed into `parent`, in order.
fn children_of(batch: &MountBatch, parent: u32) -> Vec<u32> {
    batch
        .items
        .iter()
        .filter_map(|i| match i {
            MountItem::Insert {
                parent: p, child, ..
            } if *p == parent => Some(*child),
            _ => None,
        })
        .collect()
}

/// The view of dialog `modal` with a translucent black background: its
/// backdrop (`bg-black/50`).
fn backdrop_of(batch: &MountBatch, modal: u32) -> Option<u32> {
    let black = nsh_core::Color::parse("#00000080").unwrap();
    children_of(batch, modal).into_iter().find(|&c| {
        batch.items.iter().any(|i| {
            matches!(i, MountItem::ViewStyle { id, style } if *id == c && style.background_color == black)
        })
    })
}

const DIALOG_PARTS: &str = r##"import { mount, signal, Text, View } from "nsh";
import { Dialog } from "nsh/base";

const open = signal(true);
const backdrop = signal(false);
const log = signal<string[]>([]);
const add = (s: string) => log.update((l) => [...l, s]);

mount(() => (
  <View style={{ flex: 1 }}>
    <Text>{`log ${log().join(",")}`}</Text>
    <Text onPress={() => backdrop.set(true)}>backdrop</Text>
    <Text onPress={() => { open.set(false); open.set(true); }}>reopen</Text>
    <Text onPress={() => { open.set(false); }}>close</Text>
    <Dialog
      open={open()}
      class="bg-white p-4"
      backdropStyle={backdrop() ? { backgroundColor: "#00000080" } : undefined}
      onCancel={() => add("cancel")}
      onOpenChange={(o, reason) => add(`change ${o} ${reason}`)}
      onClose={() => add("close")}
    >
      <Text>in dialog</Text>
    </Dialog>
  </View>
));
"##;

/// A dialog renders the app's popup, and a backdrop only while the app
/// styles one; a press on it closes the dialog even when the app keeps
/// `open` true, until `open` goes from false to true.
#[test]
fn dialogs_render_their_parts_and_close_from_the_backdrop() {
    let (engine, rx) = start_app(DIALOG_PARTS);
    let control = engine.control();
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let modal = modal_of(&first).expect("a dialog");
    // Only the popup, styled with the dialog's classes; the dialog itself
    // has no style.
    let parts = children_of(&first, modal);
    assert_eq!(parts.len(), 1, "{:?}", first.items);
    let white = nsh_core::Color::parse("#ffffff").unwrap();
    assert!(first.items.iter().any(|i| matches!(
        i,
        MountItem::ViewStyle { id, style } if *id == parts[0] && style.background_color == white
    )));
    assert!(
        !first
            .items
            .iter()
            .any(|i| matches!(i, MountItem::ViewStyle { id, .. } if *id == modal))
    );
    // A backdrop once the app styles one, behind the popup.
    let press = |batch: &MountBatch, label: &str| {
        control.dispatch_event(
            text_id(batch, label).unwrap(),
            EventKind::Press,
            EventPayload::None,
        )
    };
    press(&first, "backdrop");
    let batch = next(&rx, |b| backdrop_of(b, modal).is_some());
    let backdrop = backdrop_of(&batch, modal).unwrap();
    assert!(batch.items.iter().any(|i| matches!(
        i,
        MountItem::Insert { parent, child, before: Some(b) } if *parent == modal && *child == backdrop && *b == parts[0]
    )));
    // Pressing it closes the dialog although `open` stays true.
    control.dispatch_event(backdrop, EventKind::Press, EventPayload::None);
    let batch = next(&rx, |b| {
        b.items
            .iter()
            .any(|i| matches!(i, MountItem::Remove { id } if *id == modal))
    });
    assert_eq!(
        log_of(&batch).as_deref(),
        Some("log cancel,change false backdropPress")
    );
    // The platform reports it gone.
    control.dispatch_event(modal, EventKind::Dismiss, EventPayload::None);
    next(&rx, |b| log_of(b).is_some_and(|l| l.ends_with(",close")));
    // `open` from false to true shows it again.
    press(&first, "close");
    while rx.recv_timeout(Duration::from_millis(200)).is_ok() {}
    press(&first, "reopen");
    let batch = next(&rx, |b| modal_of(b).is_some());
    assert!(text_id(&batch, "in dialog").is_some());
}

const PAGE_EVENTS: &str = r##"import { mount, signal, Text, View } from "nsh";
import { NavPage, NavStack, createNavStack } from "nsh/base";

const nav = createNavStack(["home", "edit"]);
const log = signal<string[]>([]);

mount(() => (
  <View style={{ flex: 1 }}>
    <Text>{`log ${log().join(",")}`}</Text>
    <NavStack state={nav} style={{ flex: 1 }}>
      {(route) => (
        <NavPage
          title={route}
          popOnBack={route !== "edit"}
          popOnSwipe={false}
          animation="fade"
          headerStyle={{ backgroundColor: "#ff0000", color: "#00ff00" }}
          headerClass="bg-white tint-blue-500"
          header={<Text>bar</Text>}
          onBackPress={() => log.update((l) => [...l, `back ${route}`])}
          onDismiss={() => log.update((l) => [...l, `pop ${route}`])}
        >
          <Text>{`depth ${nav.routes().length}`}</Text>
        </NavPage>
      )}
    </NavStack>
  </View>
));
"##;

/// A page whose `popOnBack` is false stays: its back presses call
/// `onBackPress`; a native pop calls `onDismiss` after its route went.
#[test]
fn pages_keep_or_pop_by_their_flags() {
    let (engine, rx) = start_app(PAGE_EVENTS);
    let control = engine.control();
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let configs: Vec<(u32, nsh_core::ScreenConfig)> = first
        .items
        .iter()
        .filter_map(|i| match i {
            MountItem::ScreenConfig { id, config } => Some((*id, (**config).clone())),
            _ => None,
        })
        .collect();
    let config = |title: &str| {
        configs
            .iter()
            .rev()
            .find(|(_, c)| c.title.as_deref() == Some(title))
            .cloned()
            .unwrap_or_else(|| panic!("{title}: {configs:?}"))
    };
    let (edit, edit_config) = config("edit");
    let (home, home_config) = config("home");
    assert!(edit_config.prevent_back && !home_config.prevent_back);
    assert!(!edit_config.gesture_enabled);
    assert_eq!(edit_config.animation, nsh_core::ScreenAnimation::Fade);
    // `headerStyle` wins over `headerClass`, key by key.
    assert_eq!(
        edit_config.background_color,
        nsh_core::Color::parse("#ff0000")
    );
    assert_eq!(edit_config.title_color, nsh_core::Color::parse("#00ff00"));
    assert!(edit_config.tint_color.is_some());
    assert!(edit_config.header_view.is_some());
    // Every page listening to `onBackPress` reports back presses, and
    // only those listen.
    let listens = |id: u32| {
        first.items.iter().any(|i| {
            matches!(i, MountItem::Events { id: v, events } if *v == id && events.contains(EventKind::BackRequest))
        })
    };
    assert!(listens(edit) && listens(home));
    control.dispatch_event(edit, EventKind::BackRequest, EventPayload::None);
    next(&rx, |b| log_of(b).as_deref() == Some("log back edit"));
    // A page that pops on back hears its back presses too.
    control.dispatch_event(home, EventKind::BackRequest, EventPayload::None);
    next(&rx, |b| {
        log_of(b).as_deref() == Some("log back edit,back home")
    });
    control.dispatch_event(edit, EventKind::Dismiss, EventPayload::None);
    let batch = next(&rx, |b| log_of(b).is_some_and(|l| l.ends_with("pop edit")));
    assert!(
        texts(&batch).contains(&"depth 1".to_owned()),
        "{:?}",
        texts(&batch)
    );
}

const PAGE_HEADER: &str = r#"import { mount, Text, View } from "nsh";
import { NavPage, NavStack, createNavStack } from "nsh/base";

const nav = createNavStack(["home"]);

mount(() => (
  <View style={{ flex: 1 }}>
    <NavStack state={nav} style={{ flex: 1 }}>
      {(route) => (
        <NavPage title={route} headerRight={<Text>right</Text>} header={<View testID="bar" style={{ height: 60 }}><Text>bar</Text></View>}>
          <View testID="content" style={{ height: 10 }} />
        </NavPage>
      )}
    </NavStack>
  </View>
));
"#;

fn frame_of(batch: &MountBatch, id: u32) -> Option<nsh_core::Rect> {
    batch.items.iter().rev().find_map(|i| match i {
        MountItem::Frame { id: v, frame } if *v == id => Some(*frame),
        _ => None,
    })
}

fn screen_config_of(batch: &MountBatch) -> Option<(u32, nsh_core::ScreenConfig)> {
    batch.items.iter().rev().find_map(|i| match i {
        MountItem::ScreenConfig { id, config } => Some((*id, (**config).clone())),
        _ => None,
    })
}

/// Without a native header (desktops, the web), a page's `header` sits at
/// its top, as wide as the page and as tall as it lays out, and the content
/// starts below it; with one, the content starts below the native bar.
#[test]
fn pages_place_their_header_without_a_native_one() {
    let (_engine, rx) = start_app_with(PAGE_HEADER, nsh_core::NO_NATIVE_HEADER);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let (screen, config) = screen_config_of(&first).expect("a page");
    let header = config.header_view.expect("its header");
    let frame = frame_of(&first, header).expect("the header's frame");
    assert_eq!(
        (frame.x, frame.y, frame.width, frame.height),
        (0.0, 0.0, 390.0, 60.0)
    );
    assert_eq!(config.header_height, 60.0);
    let content = children_of(&first, screen)
        .into_iter()
        .filter(|&c| Some(c) != config.header_view && Some(c) != config.right)
        .find(|&c| frame_of(&first, c).is_some_and(|f| f.height == 10.0))
        .expect("the content");
    assert_eq!(frame_of(&first, content).unwrap().y, 60.0);

    let (_engine, rx) = start_app(PAGE_HEADER);
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let (_, config) = screen_config_of(&first).expect("a page");
    assert_eq!(config.header_height, 44.0);
}

const INPUT_AND_LIST: &str = r#"import { mount, signal, Input, Text, View } from "nsh";
import { List } from "nsh/list";

const items = signal<number[]>([]);
const log = signal("log");

mount(() => (
  <View style={{ flex: 1 }}>
    <Text>{log()}</Text>
    <Input
      testID="field"
      onChange={(text) => log.set(`change ${text}`)}
      onSubmit={(e) => log.set(`submit ${e.text}`)}
    />
    <Text onPress={() => items.set([1, 2])}>fill</Text>
    <List
      each={items()}
      style={{ flex: 1 }}
      contentStyle={{ padding: 8 }}
      header={() => <Text>head</Text>}
      empty={() => <Text>nothing</Text>}
      separator={() => <Text>sep</Text>}
    >
      {(item) => <Text>{`row ${item()}`}</Text>}
    </List>
  </View>
));
"#;

/// An input reports edits with `onChange` and the return key with
/// `onSubmit`; a list renders its `header`, `empty` and `separator`.
#[test]
fn inputs_and_list_slots() {
    let (engine, rx) = start_app(INPUT_AND_LIST);
    let control = engine.control();
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("first frame");
    let mut seen = texts(&first);
    while let Ok(b) = rx.recv_timeout(Duration::from_millis(200)) {
        seen.extend(texts(&b));
    }
    assert!(seen.contains(&"head".to_owned()), "{seen:?}");
    assert!(seen.contains(&"nothing".to_owned()), "{seen:?}");
    let input = first
        .items
        .iter()
        .find_map(|i| match i {
            MountItem::Input { id, .. } => Some(*id),
            _ => None,
        })
        .expect("an input");
    control.dispatch_event(
        input,
        EventKind::ChangeText,
        EventPayload::Text {
            text: "hi".into(),
            count: 1,
        },
    );
    next(&rx, |b| text_id(b, "change hi").is_some());
    control.dispatch_event(
        input,
        EventKind::SubmitEditing,
        EventPayload::Text {
            text: "hi".into(),
            count: 1,
        },
    );
    next(&rx, |b| text_id(b, "submit hi").is_some());
    let fill = text_id(&first, "fill").unwrap();
    control.dispatch_event(fill, EventKind::Press, EventPayload::None);
    let mut seen = Vec::new();
    while let Ok(b) = rx.recv_timeout(Duration::from_millis(300)) {
        seen.extend(texts(&b));
    }
    for wanted in ["row 1", "row 2", "sep"] {
        assert!(seen.contains(&wanted.to_owned()), "{wanted}: {seen:?}");
    }
}
