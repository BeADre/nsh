//! The components of `nsh/ui/desktop`, written in Rust.
//!
//! The desktop set draws NSH's design language, the one `nsh/ui/mobile`
//! draws, for a pointer and a keyboard: denser controls ([`ControlSize`]),
//! hover highlights, focus rings for keyboard focus. Both sets take their
//! colors, radii and font size from the theme, their motion and dimming
//! from [`nsh_core::ui`], and what controls do from its behaviors
//! (`controllable`, `NumberRange`, `press_repeat`); each draws its own
//! controls. Each component renders nsh's elements with typed styles and
//! theme colors, lays out in the core's pass and handles its own
//! interaction natively. Its JS face (the `nsh/ui/desktop` export and its
//! TypeScript declarations) is generated from its props and events by the
//! bundler. See README.md for how to write one.

mod button;
mod metrics;

use std::sync::Arc;

use nsh_core::ui::{ComponentSpec, Registry};

pub use crate::{
    button::{Button, ButtonEvent, ButtonProps, ButtonVariant},
    metrics::ControlSize,
};

/// The JS module apps import the desktop UI as.
pub const SPECIFIER: &str = "nsh/ui/desktop";

/// Every component of the desktop set, for an engine to render.
pub fn registry() -> Registry {
    let mut registry = Registry::ui_set(SPECIFIER);
    registry.register::<Button>();
    registry
}

/// The specs of the desktop set, for the generated JS glue.
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
        assert_eq!(components, ["Button"]);
        assert!(registry.module_specs().is_empty());
    }
}
