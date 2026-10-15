//! The canvas's operations (`CV_*`, after the drawing's `OP_DRAW`) and the
//! part of the 2D context that only records them: the methods that push
//! their code and arguments, and the drawing-state properties that push
//! their code and value. `canvas.ts` merges the generated methods and
//! properties into its classes and writes the rest (overloads, gradients,
//! images, `save` / `restore`); the decoder is `crates/nsh-js/src/canvas.rs`.

codes! { CODES:
    CV_SAVE = 0,
    CV_RESTORE = 1,
    CV_TRANSLATE = 2,
    CV_SCALE = 3,
    CV_ROTATE = 4,
    CV_TRANSFORM = 5,
    CV_SET_TRANSFORM = 6,
    CV_BEGIN_PATH = 7,
    CV_MOVE_TO = 8,
    CV_LINE_TO = 9,
    CV_QUAD_TO = 10,
    CV_CUBIC_TO = 11,
    CV_ARC = 12,
    CV_ARC_TO = 13,
    CV_ELLIPSE = 14,
    CV_RECT = 15,
    CV_ROUND_RECT = 16,
    CV_CLOSE = 17,
    CV_FILL = 18,
    CV_STROKE = 19,
    CV_CLIP = 20,
    CV_FILL_RECT = 21,
    CV_STROKE_RECT = 22,
    CV_CLEAR_RECT = 23,
    CV_FILL_STYLE = 24,
    CV_STROKE_STYLE = 25,
    CV_LINE_WIDTH = 26,
    CV_LINE_CAP = 27,
    CV_LINE_JOIN = 28,
    CV_MITER_LIMIT = 29,
    CV_LINE_DASH = 30,
    CV_LINE_DASH_OFFSET = 31,
    CV_GLOBAL_ALPHA = 32,
    CV_COMPOSITE = 33,
    CV_SHADOW_COLOR = 34,
    CV_SHADOW_BLUR = 35,
    CV_SHADOW_OFFSET = 36,
    CV_FONT = 37,
    CV_TEXT_ALIGN = 38,
    CV_TEXT_BASELINE = 39,
    CV_FILL_TEXT = 40,
    CV_STROKE_TEXT = 41,
    CV_DRAW_IMAGE = 42,
    /// SVG path data, parsed natively (`new Path2D("M0 0 L10 10")`)
    CV_SVG_PATH = 50,
}

/// A parameter of a recording method: its name, TypeScript type and JS
/// default (`None`: required).
pub type Param = (&'static str, &'static str, Option<&'static str>);

/// A method that pushes its code and its arguments as they are.
#[derive(Debug, Clone, Copy)]
pub struct Method {
    /// Its JS name.
    pub name: &'static str,
    /// The operation's code.
    pub code: i32,
    /// Its parameters, pushed in order.
    pub params: &'static [Param],
    /// Documentation.
    pub doc: &'static str,
}

/// A drawing-state property: reading it gives the state's value, setting it
/// keeps the value and pushes its code and the value.
#[derive(Debug, Clone, Copy)]
pub struct Property {
    /// Its JS name.
    pub name: &'static str,
    /// The operation's code.
    pub code: i32,
    /// Its TypeScript type.
    pub ts: &'static str,
    /// Its initial value (JS), as in HTML Canvas.
    pub initial: &'static str,
    /// Documentation.
    pub doc: &'static str,
}

const N: &str = "CanvasNumber";
const XY: &[Param] = &[("x", N, None), ("y", N, None)];
const RECT: &[Param] = &[
    ("x", N, None),
    ("y", N, None),
    ("w", N, None),
    ("h", N, None),
];
const MATRIX: &[Param] = &[
    ("a", N, None),
    ("b", N, None),
    ("c", N, None),
    ("d", N, None),
    ("e", N, None),
    ("f", N, None),
];

/// Path building: the context's current path and `Path2D`.
pub const PATH_METHODS: &[Method] = &[
    Method {
        name: "moveTo",
        code: CV_MOVE_TO,
        params: XY,
        doc: "Starts a subpath at (`x`, `y`).",
    },
    Method {
        name: "lineTo",
        code: CV_LINE_TO,
        params: XY,
        doc: "A straight line to (`x`, `y`).",
    },
    Method {
        name: "quadraticCurveTo",
        code: CV_QUAD_TO,
        params: &[
            ("cpx", N, None),
            ("cpy", N, None),
            ("x", N, None),
            ("y", N, None),
        ],
        doc: "A quadratic Bézier curve to (`x`, `y`).",
    },
    Method {
        name: "bezierCurveTo",
        code: CV_CUBIC_TO,
        params: &[
            ("cp1x", N, None),
            ("cp1y", N, None),
            ("cp2x", N, None),
            ("cp2y", N, None),
            ("x", N, None),
            ("y", N, None),
        ],
        doc: "A cubic Bézier curve to (`x`, `y`).",
    },
    Method {
        name: "arc",
        code: CV_ARC,
        params: &[
            ("x", N, None),
            ("y", N, None),
            ("radius", N, None),
            ("startAngle", N, None),
            ("endAngle", N, None),
            ("counterclockwise", "boolean", Some("false")),
        ],
        doc: "A circular arc; angles in radians, clockwise from the x axis.",
    },
    Method {
        name: "arcTo",
        code: CV_ARC_TO,
        params: &[
            ("x1", N, None),
            ("y1", N, None),
            ("x2", N, None),
            ("y2", N, None),
            ("radius", N, None),
        ],
        doc: "An arc tangent to the lines to (`x1`, `y1`) and on to (`x2`, `y2`).",
    },
    Method {
        name: "ellipse",
        code: CV_ELLIPSE,
        params: &[
            ("x", N, None),
            ("y", N, None),
            ("radiusX", N, None),
            ("radiusY", N, None),
            ("rotation", N, None),
            ("startAngle", N, None),
            ("endAngle", N, None),
            ("counterclockwise", "boolean", Some("false")),
        ],
        doc: "An elliptical arc; angles in radians, clockwise.",
    },
    Method {
        name: "rect",
        code: CV_RECT,
        params: RECT,
        doc: "A closed rectangle subpath.",
    },
    Method {
        name: "closePath",
        code: CV_CLOSE,
        params: &[],
        doc: "A straight line back to the subpath's start.",
    },
];

/// The context's own recording methods.
pub const CONTEXT_METHODS: &[Method] = &[
    Method {
        name: "translate",
        code: CV_TRANSLATE,
        params: XY,
        doc: "Moves the origin.",
    },
    Method {
        name: "scale",
        code: CV_SCALE,
        params: XY,
        doc: "Scales the axes.",
    },
    Method {
        name: "rotate",
        code: CV_ROTATE,
        params: &[("angle", N, None)],
        doc: "Rotates the axes; radians, clockwise.",
    },
    Method {
        name: "transform",
        code: CV_TRANSFORM,
        params: MATRIX,
        doc: "Multiplies the transform by the matrix [a c e; b d f].",
    },
    Method {
        name: "setTransform",
        code: CV_SET_TRANSFORM,
        params: MATRIX,
        doc: "Replaces the transform by the matrix [a c e; b d f].",
    },
    Method {
        name: "beginPath",
        code: CV_BEGIN_PATH,
        params: &[],
        doc: "Starts a new current path.",
    },
    Method {
        name: "fillRect",
        code: CV_FILL_RECT,
        params: RECT,
        doc: "Fills a rectangle with `fillStyle` (the current path stays).",
    },
    Method {
        name: "strokeRect",
        code: CV_STROKE_RECT,
        params: RECT,
        doc: "Strokes a rectangle with `strokeStyle`.",
    },
    Method {
        name: "clearRect",
        code: CV_CLEAR_RECT,
        params: RECT,
        doc: "Makes a rectangle transparent.",
    },
];

/// The drawing-state properties that are kept and pushed as they are.
pub const PROPERTIES: &[Property] = &[
    Property {
        name: "lineWidth",
        code: CV_LINE_WIDTH,
        ts: N,
        initial: "1",
        doc: "Stroke width in points.",
    },
    Property {
        name: "lineCap",
        code: CV_LINE_CAP,
        ts: r#""butt" | "round" | "square""#,
        initial: r#""butt""#,
        doc: "How strokes end.",
    },
    Property {
        name: "lineJoin",
        code: CV_LINE_JOIN,
        ts: r#""miter" | "round" | "bevel""#,
        initial: r#""miter""#,
        doc: "How stroke segments join.",
    },
    Property {
        name: "miterLimit",
        code: CV_MITER_LIMIT,
        ts: "number",
        initial: "10",
        doc: "Miter joins longer than this (in line widths) are beveled.",
    },
    Property {
        name: "lineDashOffset",
        code: CV_LINE_DASH_OFFSET,
        ts: N,
        initial: "0",
        doc: "Where the dash pattern starts.",
    },
    Property {
        name: "globalAlpha",
        code: CV_GLOBAL_ALPHA,
        ts: N,
        initial: "1",
        doc: "Opacity of what is drawn next, 0..1.",
    },
    Property {
        name: "globalCompositeOperation",
        code: CV_COMPOSITE,
        ts: "string",
        initial: r#""source-over""#,
        doc: "How what is drawn next combines with the canvas (`source-over`, `multiply`, ...).",
    },
    Property {
        name: "shadowColor",
        code: CV_SHADOW_COLOR,
        ts: "string",
        initial: r#""rgba(0,0,0,0)""#,
        doc: "The color of shadows (transparent: none).",
    },
    Property {
        name: "shadowBlur",
        code: CV_SHADOW_BLUR,
        ts: "number",
        initial: "0",
        doc: "Shadow blur in points.",
    },
    Property {
        name: "font",
        code: CV_FONT,
        ts: "string",
        initial: r#""10px sans-serif""#,
        doc: "CSS `font` shorthand, e.g. `\"600 14px Inter\"` (sizes in px, pt or rem).",
    },
    Property {
        name: "textAlign",
        code: CV_TEXT_ALIGN,
        ts: r#""start" | "end" | "left" | "right" | "center""#,
        initial: r#""start""#,
        doc: "Where text sits horizontally relative to its x.",
    },
    Property {
        name: "textBaseline",
        code: CV_TEXT_BASELINE,
        ts: r#""top" | "hanging" | "middle" | "alphabetic" | "ideographic" | "bottom""#,
        initial: r#""alphabetic""#,
        doc: "Where text sits vertically relative to its y.",
    },
];

/// `canvas.gen.ts`: the methods' and properties' declarations, and their
/// implementations as objects the classes of `canvas.ts` take into their
/// prototypes. Codes are literals in the bodies: a call costs what the
/// hand-written one did.
pub fn typescript() -> String {
    use std::fmt::Write as _;
    let mut out = super::banner("nsh_core::js::canvas");
    out.push_str(
        "import type { NativeNode } from \"./animated\";\n\n\
         /** A number in a drawing: a constant, or a native value the canvas reads\n \
         * while it draws (its animations redraw natively, without JS). */\n\
         export type CanvasNumber = number | NativeNode;\n\n\
         /** Where the methods record. */\n\
         export interface CanvasRecorder {\n  out: unknown[];\n}\n\n\
         /** Where the properties record, and the state they keep. */\n\
         export interface CanvasStateRecorder extends CanvasRecorder {\n  s: CanvasProperties;\n}\n",
    );
    let methods = |out: &mut String, iface: &str, doc: &str, list: &[Method]| {
        let _ = write!(out, "\n/** {doc} */\nexport interface {iface} {{\n");
        for m in list {
            let params = m
                .params
                .iter()
                .map(|(n, ts, default)| {
                    format!("{n}{}: {ts}", if default.is_some() { "?" } else { "" })
                })
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(out, "  /** {} */\n  {}({params}): void;", m.doc, m.name);
        }
        out.push_str("}\n\n");
        let mut value = iface.to_owned();
        if let Some(first) = value.get_mut(..1) {
            first.make_ascii_lowercase();
        }
        let _ = writeln!(
            out,
            "export const {value}: {iface} & ThisType<CanvasRecorder> = {{"
        );
        for m in list {
            let params = m
                .params
                .iter()
                .map(|(n, _, default)| match default {
                    Some(d) => format!("{n} = {d}"),
                    None => (*n).to_owned(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            let args: String = m.params.iter().map(|(n, _, _)| format!(", {n}")).collect();
            let _ = writeln!(
                out,
                "  {}({params}) {{\n    this.out.push({}{args});\n  }},",
                m.name, m.code
            );
        }
        out.push_str("};\n");
    };
    methods(
        &mut out,
        "CanvasPathMethods",
        "Path building, shared by the context (its current path) and `Path2D`.",
        PATH_METHODS,
    );
    methods(
        &mut out,
        "CanvasContextMethods",
        "The context's transforms and rectangles.",
        CONTEXT_METHODS,
    );
    out.push_str("\n/** The drawing state kept as set. */\nexport interface CanvasProperties {\n");
    for p in PROPERTIES {
        let _ = writeln!(out, "  /** {} */\n  {}: {};", p.doc, p.name, p.ts);
    }
    out.push_str("}\n\n/** The state's initial values (HTML Canvas's). */\nexport const initialCanvasProperties: CanvasProperties = {\n");
    for p in PROPERTIES {
        let _ = writeln!(out, "  {}: {},", p.name, p.initial);
    }
    out.push_str("};\n\n/** The properties' accessors. */\nexport const canvasProperties: PropertyDescriptorMap = {\n");
    for p in PROPERTIES {
        let _ = writeln!(
            out,
            "  {0}: {{\n    get(this: CanvasStateRecorder): {1} {{\n      return this.s.{0};\n    }},\n    \
             set(this: CanvasStateRecorder, v: {1}) {{\n      this.s.{0} = v;\n      this.out.push({2}, v);\n    }},\n    \
             configurable: true,\n  }},",
            p.name, p.ts, p.code
        );
    }
    out.push_str("};\n");
    out
}
