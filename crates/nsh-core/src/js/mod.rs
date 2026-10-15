//! What parts of the JS runtime are generated from, so the two sides of the
//! op buffer cannot drift apart: its codes ([`protocol`]), the canvas API
//! that records drawing operations ([`canvas`]) and the style types
//! ([`style`]). The bundler embeds each generator's output as an internal
//! module of `js/runtime` (`protocol.gen.ts`, `canvas.gen.ts`,
//! `style.gen.ts`); the decoders in `nsh-js` match on the same constants.

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
    }
}
