//! What parts of the JS runtime are generated from, so the two sides of the
//! op buffer cannot drift apart: its codes ([`protocol`]), the canvas API
//! that records drawing operations and their decoding ([`canvas`], with
//! [`ops`]' `#[derive(JsOps)]`), the style types
//! ([`style`]), the host elements' props ([`host`]: their decoding and
//! types, from `#[derive(HostProps)]` on [`crate::Prop`]) and the easings
//! and layout animations ([`anim`]). The bundler
//! embeds each generator's output as an internal module of `js/runtime`
//! (`protocol.gen.ts`, `canvas.gen.ts`, `style.gen.ts`, `elements.gen.ts`,
//! `anim.gen.ts`);
//! the decoders in `nsh-js` match on the same constants and call the
//! derived decoders.

use std::time::Duration;

/// nsh's own native ES modules, which bundles compiled with nsh's JSX
/// compiler import (served by `nsh-js`, left as imports by the bundler):
/// `nsh`, the API apps write against; `nsh:rt`, the renderer compiled JSX
/// calls; `nsh/device` and `nsh/accessibility`, the environment. Registries
/// add theirs (`Registry::specifier`: `nsh/ui/mobile`, `nsh/android`, ...).
pub const NATIVE_MODULES: &[&str] = &["nsh", "nsh:rt", "nsh/device", "nsh/accessibility"];

/// Resource limits of one VM.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// QuickJS heap, in bytes (0: no limit). Running out stops the VM.
    pub memory_bytes: usize,
    /// Interpreter stack, in bytes: deep recursion becomes a `RangeError`
    /// rather than a native stack overflow. Must stay below the engine
    /// thread's stack.
    pub stack_bytes: usize,
    /// Evaluating the bundle.
    pub load_budget: Duration,
    /// Any other call into JS: one event (with the rendering it causes), one
    /// list bind, one timer, one frame's callbacks, the microtask drain.
    pub call_budget: Duration,
    /// Values one commit may send (the JS op buffer's length). The buffer is
    /// decoded into Rust memory, which the heap limit does not cover.
    pub max_op_values: usize,
    /// Operations one canvas drawing may record.
    pub max_canvas_ops: usize,
    /// Nodes the core keeps (each may have a native view, whose memory the
    /// heap limit does not cover either).
    pub max_nodes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            memory_bytes: 256 << 20,
            stack_bytes: 4 << 20,
            load_budget: Duration::from_secs(10),
            call_budget: Duration::from_secs(2),
            max_op_values: 8_000_000,
            max_canvas_ops: 1_000_000,
            max_nodes: 200_000,
        }
    }
}

/// Constants of the op buffer and a table of them for the generators.
macro_rules! codes {
    ($table:ident: $($(#[doc = $doc:literal])* $name:ident = $value:literal),* $(,)?) => {
        $(
            #[doc = concat!("`", stringify!($name), "`.")]
            $(#[doc = $doc])*
            pub const $name: i32 = $value;
        )*

        /// The codes by name, with their documentation (the operands).
        pub const $table: &[(&str, i32, &str)] = &[$((stringify!($name), $value, concat!($($doc),*))),*];
    };
}

pub mod anim;
pub mod canvas;
pub mod host;
pub(crate) mod host_values;
pub mod ops;
pub mod protocol;
pub mod style;

/// The banner of a generated file.
fn banner(what: &str) -> String {
    format!("// Generated from {what}: do not edit.\n\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_generated_runtime_matches_the_tables() {
        let protocol = super::protocol::typescript();
        assert!(protocol.contains("export const OP_DRAW = 9;"));
        assert!(protocol.contains("export const ANIM_RELEASE_COMPOSITE = 11;"));
        assert!(protocol.contains("export const CV_SVG_PATH = 50;"));
        let canvas = super::canvas::typescript();
        assert!(canvas.contains("  moveTo(x, y) {\n    this.out.push(8, x, y);\n  },"));
        assert!(canvas.contains("counterclockwise = false) {"));
        assert!(canvas.contains("      this.out.push(26, v);"));
        assert!(canvas.contains("  lineCap: \"butt\","));
        assert!(canvas.contains("  lineCap: LineCap;"));
        assert!(canvas.contains("export type LineCap = \"butt\" | \"round\" | \"square\";"));
        let style = super::style::typescript();
        // Every property the core parses has a type, in its interface.
        for (name, ts) in [
            ("boxShadow", "string"),
            ("gridTemplateColumns", "string"),
            ("transform", "TransformStep[]"),
            ("tintColor", "ColorValue"),
        ] {
            assert!(style.contains(&format!("  {name}?: {ts};")), "{name}");
        }
        let text = style.find("export interface TextStyle").unwrap();
        assert!(style[text..].contains("  textDecorationStyle?:"));
        // Each host prop in the interface of the elements it is for.
        let elements = super::host::typescript();
        for (interface, prop) in [
            ("HostProps", "  pointerEvents?: PointerEvents;"),
            ("AndroidHostProps", "  elevation?: number;"),
            ("InputHostProps", "  keyboardType?: KeyboardType;"),
            ("IOSInputHostProps", "  inputAccessoryViewID?: string;"),
            (
                "ScrollingHostProps",
                "  decelerationRate?: \"normal\" | \"fast\" | number;",
            ),
            (
                "ListHostProps",
                "  contentContainerStyle?: ListContentStyle;",
            ),
            (
                "IOSModalHostProps",
                "  presentationStyle?: ModalPresentation;",
            ),
        ] {
            let at = elements
                .find(&format!("export interface {interface} {{"))
                .unwrap_or_else(|| panic!("{interface}"));
            let end = at + elements[at..].find("\n}").unwrap();
            assert!(elements[at..end].contains(prop), "{interface}: {prop}");
        }
        assert!(elements.contains("\"number-pad\" | \"numeric\""));
        let anim = super::anim::typescript();
        assert!(anim.contains("  back: (s: number = 1.70158): EasingCurve =>"));
        assert!(anim.contains("export type LayoutAnimationType = \"easeInEaseOut\" | \"spring\""));
        assert!(anim.contains("  readonly scaleXY: \"scaleXY\";"));
        assert!(anim.contains(
            "    update: { type: \"spring\", property: \"opacity\", duration: 700, delay: 0, springDamping: 0.4 },"
        ));
        // What the runtime sets itself is not part of the types.
        for internal in ["splice?", "cellIndex?", "tailwindTheme?", "headerSlot?"] {
            assert!(!elements.contains(internal), "{internal}");
        }
    }
}
