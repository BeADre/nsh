# Canvas

- **Canvas** (`Canvas` component, `<canvas>` element): an HTML Canvas 2D subset recorded in JS and replayed natively
  (`nsh_core::canvas`: `Drawing { ops, slots }`). `draw={(ctx, { width, height }) => ...}` runs again when the signals it reads
  change (sizes come from `onLayout`). `CanvasRenderingContext2D`: state (`save` / `restore`, `globalAlpha`,
  `globalCompositeOperation`, line styles and dashes, shadows, `font` (CSS shorthand), `textAlign` / `textBaseline`),
  transforms, paths (`arc`, `arcTo`, `ellipse`, curves, `rect`, `roundRect`), `fill` / `stroke` / `clip` (also of a `Path2D`,
  which takes SVG path data; arcs become cubics), `fillRect` / `strokeRect` / `clearRect`, `fillText` / `strokeText` /
  `measureText` (synchronous, through the shared text engine: `__nsh_measure_text`; only `width`), linear / radial / conic
  gradients, `drawImage` (all three forms, images by URI or asset). Any number argument and colors may be native values
  (`createNativeValue`, `interpolate`, ...): those become slots bound like style values (`AnimProp::CanvasArg`) and the
  platform redraws on its own while they animate, without JS. Drawings travel in their own op (`OP_DRAW`), recorded into a
  separate array: creating native values while drawing pushes ops of its own. Each operation is one variant of
  `CanvasOp` / `PathOp` with `#[derive(JsOps)]` (`nsh_core::js::ops`): `#[op(code = CV_…)]` reads its fields in order
  (`OpArg`: numbers or native values, paints, `Path2D` lists, string unions; an invalid color or font drops the
  operation, as in HTML Canvas), and `method = …, params = …` / `property = …, initial = …` generate the methods and
  properties that only record (`moveTo`, `fillRect`, `lineWidth`, ...) into `canvas.gen.ts`, which `canvas.ts` takes into
  its classes' prototypes; what has logic (overloads, `roundRect`'s radii, `save` / `restore`, gradients, images) is
  written in `canvas.ts`. The core reads drawings straight from QuickJS values (`nsh_core::js::canvas::decode`). The calls cost what hand-written ones did
  (`docs/dev/performance.md`). iOS replays into the `drawRect:` context,
  keeping its own transform and path in view space; Android `CanvasBox.java` interprets an `int[]` (`encode.rs`). Not
  supported: `getImageData` / `putImageData`, filters, `isPointInPath`, SVG images.

The drawing format is documented in `crates/nsh-core/src/canvas.rs`.
