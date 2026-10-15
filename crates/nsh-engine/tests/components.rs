//! Rust components through the generated JS glue: props, events, children,
//! named slots, children placed one by one, and a list of the app's rows.

use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use nsh_core::{ApproxTextEngine, EventKind, EventPayload, MountBatch, MountItem, Size};
use nsh_engine::{Bundle, Engine, EngineConfig, MountSink};

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
    let dir = std::env::temp_dir().join(format!(
        "nsh-components-{}-{}",
        std::process::id(),
        app.len()
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
        environment: Default::default(),
        header_heights: (44.0, 96.0),
        limits: nsh_engine::Limits::default(),
        components: nsh_ui_mobile::registry(),
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
