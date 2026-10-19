//! The components of `nsh/ui`, written in Rust: those every form factor
//! shares (buttons, cards, lists, fields, safe areas, navigation, alerts).
//!
//! Each component renders nsh's elements with typed styles and theme colors
//! ([`nsh_core::ui`]): one implementation draws the same on every platform,
//! lays out in the core's pass and handles its own interaction natively.
//! Its JS face (the `nsh/ui` export and its TypeScript declarations) is
//! generated from its props and events by the bundler. What belongs to one
//! form factor lives in that form factor's set (`nsh-ui-mobile`). See
//! README.md for how to write one.

mod alert;
mod button;
mod card;
mod group;
mod list_view;
mod safe_area;
mod select_list;
mod stepper;
mod text_field;

use std::sync::Arc;

use nsh_core::ui::{ComponentSpec, Registry};

pub use crate::{
    alert::{Alert, AlertButton, AlertButtonStyle, AlertOptions},
    button::{Button, ButtonEvent, ButtonProps, ButtonVariant},
    card::{Card, CardProps, CardVariant},
    group::{Group, GroupProps},
    list_view::{ListView, ListViewProps, ListViewVariant},
    safe_area::{Edge, SafeAreaView, SafeAreaViewProps},
    select_list::{SelectList, SelectListEvent, SelectListProps},
    stepper::{Stepper, StepperApi, StepperEvent, StepperProps},
    text_field::{TextField, TextFieldEvent, TextFieldProps},
};

/// The JS module apps import the shared UI as.
pub const SPECIFIER: &str = "nsh/ui";

/// Every component of the shared set, for an engine to render.
pub fn registry() -> Registry {
    let mut registry = Registry::ui_set(SPECIFIER);
    registry
        .register::<Button>()
        .register::<Card>()
        .register::<Group>()
        .register::<ListView>()
        .register::<Stepper>()
        .register::<TextField>()
        .register::<SelectList>()
        .register::<SafeAreaView>()
        .module::<Alert>()
        // The runtime's accessors and hooks of the shared UI, as they are.
        .reexport(&["safeArea", "useSafeAreaInsets"], &["Insets"])
        // Navigation: made by `nsh-js` (`navigation.rs`), declared by hand.
        .native_values(&["Stack", "Screen", "Modal", "createNavigator"]);
    registry
}

/// The specs of the shared set, for the generated JS glue.
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
        assert_eq!(
            components,
            [
                "Button",
                "Card",
                "Group",
                "ListView",
                "SafeAreaView",
                "SelectList",
                "Stepper",
                "TextField",
            ]
        );
        let modules: Vec<_> = registry.module_specs().iter().map(|m| m.name).collect();
        assert_eq!(modules, ["Alert"]);
    }
}
