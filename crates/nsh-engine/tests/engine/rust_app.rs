//! A Rust app: the engine mounts its root component with no bundle and no
//! VM; it renders, handles presses and runs its tasks.

use std::time::Duration;

use crossbeam_channel::Receiver;
use nsh_core::{
    ApproxTextEngine, EventKind, EventPayload, MountBatch, MountItem, Size,
    ui::{Root, prelude::*},
};
use nsh_engine::{App, Engine, EngineConfig, Environment};

use crate::ChannelSink;

#[derive(Default)]
struct Counter {
    count: u32,
    ticks: u32,
    _timer: Option<Task<()>>,
}

#[derive(Clone, PartialEq, Props)]
struct CounterProps {
    #[prop(default = "Count".into())]
    label: SharedString,
}

impl Component for Counter {
    const NAME: &'static str = "app.counter";
    type Props = CounterProps;
    type Event = NoEvent;

    fn setup(&mut self, _: &CounterProps, cx: &mut Context<'_, Self>) {
        self._timer = Some(
            cx.set_timeout(Duration::from_millis(10), |this: &mut Self, cx| {
                this.ticks += 1;
                cx.notify();
            }),
        );
    }

    fn render(&self, props: &CounterProps, cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view()
            .flex_1()
            .on_press(cx.listener(|this: &mut Self, _, cx| {
                this.count += 1;
                cx.notify();
            }))
            .child(text(format!(
                "{} {} ({} ticks)",
                props.label, self.count, self.ticks
            )))
    }
}

fn start() -> (Engine, Receiver<MountBatch>) {
    let (tx, rx) = crossbeam_channel::unbounded();
    let config = EngineConfig {
        app: App::Rust(Root::new::<Counter>(|| CounterProps {
            label: "Taps".into(),
        })),
        viewport: Size::new(390.0, 844.0),
        trace: false,
        network: None,
        assets: None,
        environment: Environment::default(),
        header_heights: (44.0, 96.0),
        limits: nsh_engine::Limits::default(),
        // Nothing registered: Rust renders its components by type.
        ui_sets: Vec::new(),
        runtime: None,
    };
    (
        Engine::start(config, || ApproxTextEngine, ChannelSink(tx)).unwrap(),
        rx,
    )
}

/// The text shown after the batches received within `wait`, and the view
/// it is in.
fn shown(rx: &Receiver<MountBatch>, wait: Duration, until: &str) -> Option<(String, u32)> {
    let deadline = std::time::Instant::now() + wait;
    let mut last = None;
    let mut host = None;
    while let Ok(batch) = rx.recv_deadline(deadline) {
        for item in &batch.items {
            match item {
                MountItem::Text { text, .. } => last = Some(text.text.clone()),
                MountItem::Insert {
                    parent: nsh_core::ROOT_ID,
                    child,
                    ..
                } => host = Some(*child),
                _ => {}
            }
        }
        if last.as_deref() == Some(until) {
            break;
        }
    }
    Some((last?, host.unwrap_or(0)))
}

#[test]
fn a_rust_app_renders_handles_presses_and_runs_tasks() {
    let (engine, rx) = start();
    let (text, host) = shown(&rx, Duration::from_secs(5), "Taps 0 (0 ticks)").unwrap();
    assert_eq!(text, "Taps 0 (0 ticks)");
    assert_ne!(host, 0, "the root component is in the root view");
    // The timer task wakes the engine and renders again.
    let (text, _) = shown(&rx, Duration::from_secs(5), "Taps 0 (1 ticks)").unwrap();
    assert_eq!(text, "Taps 0 (1 ticks)");
    engine
        .control()
        .dispatch_event(host, EventKind::Press, EventPayload::None);
    let (text, _) = shown(&rx, Duration::from_secs(5), "Taps 1 (1 ticks)").unwrap();
    assert_eq!(text, "Taps 1 (1 ticks)");
}

/// A Rust navigation stack: a screen per route, keyed by it.
#[derive(Default)]
struct Navigation {
    routes: Vec<u32>,
}

impl Component for Navigation {
    const NAME: &'static str = "app.navigation";
    type Props = ();
    type Event = NoEvent;

    fn setup(&mut self, _: &(), _: &mut Context<'_, Self>) {
        self.routes = vec![0, 1];
    }

    fn render(&self, _: &(), cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        // A component's root is a view (its host node).
        view()
            .flex_1()
            .child(stack().flex_1().children(self.routes.iter().map(|&route| {
                screen()
                    .key(route.to_string())
                    .title(format!("page {route}"))
                    .header_right(text("more"))
                    .on_dismiss(cx.listener(move |this: &mut Self, _, cx| {
                        this.routes.retain(|r| *r < route);
                        cx.notify();
                    }))
                    .child(text(format!("content {route}")))
            })))
    }
}

#[test]
fn rust_components_navigate_with_native_stacks() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let config = EngineConfig {
        app: App::Rust(Root::of::<Navigation>()),
        viewport: Size::new(390.0, 844.0),
        trace: false,
        network: None,
        assets: None,
        environment: Environment::default(),
        header_heights: (44.0, 96.0),
        limits: nsh_engine::Limits::default(),
        ui_sets: Vec::new(),
        runtime: None,
    };
    let engine = Engine::start(config, || ApproxTextEngine, ChannelSink(tx)).unwrap();
    let first = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let screens: Vec<(u32, String, Option<u32>)> = first
        .items
        .iter()
        .filter_map(|i| match i {
            MountItem::ScreenConfig { id, config } => Some((
                *id,
                config.title.as_deref().unwrap_or_default().to_owned(),
                config.right,
            )),
            _ => None,
        })
        .collect();
    let titles: Vec<&str> = screens.iter().map(|s| s.1.as_str()).collect();
    assert!(
        titles.contains(&"page 0") && titles.contains(&"page 1"),
        "{titles:?}"
    );
    assert!(
        screens.iter().all(|s| s.2.is_some()),
        "a header item each: {screens:?}"
    );
    let kinds: Vec<_> = first
        .items
        .iter()
        .filter_map(|i| match i {
            MountItem::Create { kind, .. } => Some(format!("{kind:?}")),
            _ => None,
        })
        .collect();
    assert!(kinds.iter().any(|k| k == "Stack"), "{kinds:?}");
    // The user pops the top screen natively: its route goes.
    let top = screens.iter().find(|s| s.1 == "page 1").unwrap().0;
    engine
        .control()
        .dispatch_event(top, EventKind::Dismiss, EventPayload::None);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut deleted = false;
    while let Ok(batch) = rx.recv_deadline(deadline) {
        if batch
            .items
            .iter()
            .any(|i| matches!(i, MountItem::Delete { id } if *id == top))
        {
            deleted = true;
            break;
        }
    }
    assert!(deleted, "the popped screen goes");
}
