//! The components of `nsh/ui/mobile`, written in Rust: those only phones
//! and tablets have (the status bar, the software keyboard, sheets).
//!
//! Each component renders nsh's elements with typed styles and theme colors
//! ([`nsh_core::ui`]): one implementation draws the same on iOS and Android,
//! lays out in the core's pass and handles its own interaction natively.
//! Its JS face (the `nsh/ui/mobile` export and its TypeScript declarations)
//! is generated from its props and events by the bundler. The components
//! every form factor shares are `nsh/ui`'s (`nsh-ui`). See `nsh-ui`'s
//! README.md for how to write one.

mod action_sheet;
mod keyboard;
mod sheet;
mod status_bar;

use std::sync::Arc;

use nsh_core::ui::{ComponentSpec, Registry};

pub use crate::{
    action_sheet::{ActionSheet, ActionSheetOptions},
    keyboard::{AvoidBehavior, Keyboard, KeyboardAvoidingView, KeyboardAvoidingViewProps},
    sheet::{Sheet, SheetEvent, SheetProps},
    status_bar::{BarAnimation, BarStyle, StatusBar, StatusBarProps},
};

/// The JS module apps import the mobile UI as.
pub const SPECIFIER: &str = "nsh/ui/mobile";

/// Every component of the mobile set, for an engine to render.
pub fn registry() -> Registry {
    let mut registry = Registry::ui_set(SPECIFIER);
    registry
        .register::<Sheet>()
        .register::<KeyboardAvoidingView>()
        .register::<StatusBar>()
        .module::<ActionSheet>()
        .module::<Keyboard>()
        .module::<StatusBar>()
        // The runtime's keyboard accessor, as it is.
        .reexport(&["keyboard"], &[]);
    registry
}

/// The specs of the mobile set, for the generated JS glue.
pub fn specs() -> Vec<Arc<ComponentSpec>> {
    registry().specs()
}

#[cfg(test)]
mod tests {
    /// The exports are named after the types (`std::any::type_name`): a
    /// renamed type, or a compiler that formats type names differently,
    /// changes what apps import.
    #[test]
    fn the_exports_are_pinned() {
        let registry = super::registry();
        let components: Vec<_> = registry.specs().iter().map(|s| s.export.clone()).collect();
        assert_eq!(components, ["KeyboardAvoidingView", "Sheet", "StatusBar"]);
        let modules: Vec<_> = registry.module_specs().iter().map(|m| m.name).collect();
        assert_eq!(modules, ["ActionSheet", "Keyboard", "StatusBar"]);
    }
}
