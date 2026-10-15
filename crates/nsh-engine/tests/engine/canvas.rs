//! `nsh/canvas` end to end: what a `draw` records (the generated methods and
//! properties, and the hand-written ones) as the core decodes it.

use std::time::Duration;

use nsh_core::{
    ApproxTextEngine, MountBatch, MountItem, Size,
    canvas::{CanvasOp, LineCap, Num, PathOp},
};
use nsh_engine::{Bundle, Engine, EngineConfig};

use crate::ChannelSink;

const APP: &str = r##"import { render, root } from "nsh";
import { Canvas, Path2D } from "nsh/canvas";

render(
  () => (
    <Canvas
      style={{ width: 100, height: 50 }}
      draw={(ctx, size) => {
        ctx.save();
        ctx.lineWidth = 3;
        ctx.lineCap = "round";
        ctx.translate(size.width / 2, 0);
        ctx.beginPath();
        ctx.moveTo(1, 2);
        ctx.lineTo(3, 4);
        ctx.arc(5, 6, 7, 0, 1);
        ctx.roundRect(0, 0, 10, 10, [2, 4]);
        ctx.closePath();
        ctx.stroke();
        ctx.restore();
        // Restored: the state's value again.
        if (ctx.lineWidth !== 1) throw new Error(`lineWidth ${ctx.lineWidth}`);
        ctx.fillStyle = "#ff0000";
        ctx.fillRect(0, 0, size.width, size.height);
        ctx.fill(new Path2D("M0 0 L10 10"));
      }}
    />
  ),
  root,
);
"##;

fn start(app: &str) -> (Engine, crossbeam_channel::Receiver<MountBatch>) {
    let dir = std::env::temp_dir().join(format!("nsh-canvas-{}", std::process::id()));
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
        runtime: None,
    };
    (
        Engine::start(config, || ApproxTextEngine, ChannelSink(tx)).unwrap(),
        rx,
    )
}

#[test]
fn a_drawing_records_what_the_context_was_told() {
    let (_engine, rx) = start(APP);
    let mut drawing = None;
    while let Ok(batch) = rx.recv_timeout(Duration::from_secs(2)) {
        drawing = batch.items.into_iter().find_map(|i| match i {
            MountItem::Canvas { drawing, .. } => Some(drawing),
            _ => None,
        });
        if drawing.is_some() {
            break;
        }
    }
    let ops = &drawing.expect("a drawing").ops;
    let c = Num::Const;
    assert!(matches!(ops[0], CanvasOp::Save));
    assert!(matches!(ops[1], CanvasOp::LineWidth(Num::Const(3.0))));
    assert!(matches!(ops[2], CanvasOp::LineCap(LineCap::Round)));
    assert!(matches!(
        ops[3],
        CanvasOp::Translate(Num::Const(50.0), Num::Const(0.0))
    ));
    assert!(matches!(ops[4], CanvasOp::BeginPath));
    assert!(matches!(ops[5], CanvasOp::Path(PathOp::MoveTo(a, b)) if a == c(1.0) && b == c(2.0)));
    assert!(matches!(ops[6], CanvasOp::Path(PathOp::LineTo(a, b)) if a == c(3.0) && b == c(4.0)));
    // The default `counterclockwise`.
    assert!(matches!(
        ops[7],
        CanvasOp::Path(PathOp::Arc { ccw: false, .. })
    ));
    assert!(matches!(ops[8], CanvasOp::Path(PathOp::RoundRect { .. })));
    assert!(matches!(ops[9], CanvasOp::Path(PathOp::Close)));
    assert!(matches!(ops[10], CanvasOp::Stroke(None)));
    assert!(matches!(ops[11], CanvasOp::Restore));
    assert!(matches!(ops[12], CanvasOp::FillStyle(_)));
    assert!(matches!(ops[13], CanvasOp::FillRect([_, _, w, h]) if w == c(100.0) && h == c(50.0)));
    assert!(matches!(ops[14], CanvasOp::Fill(Some(_), _)));
    assert_eq!(ops.len(), 15, "{ops:?}");
}
