//! What parts of the JS runtime are generated from, so the two sides of the
//! op buffer cannot drift apart: its codes ([`protocol`]), the canvas API
//! that records drawing operations and their decoding ([`canvas`], with
//! [`ops`]' `#[derive(JsOps)]`), the style types
//! ([`style`]) and the host elements' props ([`host`]: their decoding and
//! types, from `#[derive(HostProps)]` on [`crate::Prop`]). The bundler
//! embeds each generator's output as an internal module of `js/runtime`
//! (`protocol.gen.ts`, `canvas.gen.ts`, `style.gen.ts`, `elements.gen.ts`);
//! the decoders in `nsh-js` match on the same constants and call the
//! derived decoders.

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
        // What the runtime sets itself is not part of the types.
        for internal in ["splice?", "cellIndex?", "tailwindTheme?", "headerSlot?"] {
            assert!(!elements.contains(internal), "{internal}");
        }
    }
}
