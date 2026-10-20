//! The Rust gallery (`examples/gallery-rs`): both the native executable
//! and the TSX entry mount [`Gallery`], and everything it shows is written
//! in Rust: the same demos as the TSX gallery (`examples/gallery`), on
//! nsh's Rust component API. The app imports the set as `gallery/native`
//! (`nsh.json` `"rust"`).

mod assets;
mod demos;
mod nav;
mod shell;
mod ui;

use nsh_core::ui::prelude::*;

use crate::ui::{colors, desktop};

/// The UI set of the gallery: what `App.tsx` imports from `gallery/native`.
#[native_module("gallery/native")]
fn native(registry: &mut Registry) {
    registry.register::<Gallery>();
    registry.assets(assets!());
}

/// The gallery, in the gallery's design for the system's appearance: on
/// phones a native stack of its index and the screens pushed on it; in wide
/// windows a sidebar of the demos and the selected one beside it.
#[component]
pub fn Gallery() -> impl IntoElement {
    view()
        .flex_1()
        .bg(colors::BACKGROUND)
        // Made again when the window crosses the threshold, as TS remounts
        // its shell: what is open carries over.
        .child(If::new(desktop, shell::desktop_layout).otherwise(shell::phone))
}
