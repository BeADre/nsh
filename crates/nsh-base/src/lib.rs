//! `nsh/base`: the behavior and infrastructure foundation of NSH apps' UI,
//! written in Rust, with no look of its own (as `gpui-base` is GPUI Kit's):
//! controls (`Button`), navigation (stacks, screens, modals), safe areas,
//! the system's dialogs, the software keyboard and the status bar.
//!
//! What it renders lays out or tells the platform, never styles: the look
//! of an app is the app's, drawn with nsh's primitives or the components of
//! its own crate. Its JS face (the `nsh/base` export and its TypeScript
//! declarations) is generated from the components' props and events and the
//! modules' functions. Every host serves it.

mod action_sheet;
mod alert;
mod button;
mod keyboard;
mod safe_area;
mod status_bar;

use std::sync::Arc;

use nsh_core::ui::{ComponentSpec, Registry};

pub use crate::{
    action_sheet::{ActionSheet, ActionSheetOptions},
    alert::{Alert, AlertButton, AlertButtonStyle, AlertOptions},
    button::{Button, ButtonEvent, ButtonProps},
    keyboard::{AvoidBehavior, Keyboard, KeyboardAvoidingView, KeyboardAvoidingViewProps},
    safe_area::{Edge, SafeAreaView, SafeAreaViewProps},
    status_bar::{
        BarAnimation, BarStyle, StatusBar, StatusBarAndroid, StatusBarProps, StatusBarWeb,
    },
};

/// The JS module apps import the foundation as.
pub const SPECIFIER: &str = "nsh/base";

/// Every component and module of `nsh/base`, for an engine to serve.
pub fn registry() -> Registry {
    let mut registry = Registry::ui_set(SPECIFIER);
    registry
        .register::<Button>()
        .register::<SafeAreaView>()
        .register::<KeyboardAvoidingView>()
        .register::<StatusBar>()
        .module::<Alert>()
        .module::<ActionSheet>()
        .module::<Keyboard>()
        .module::<StatusBar>()
        // The environment's accessors and hooks, as they are.
        .reexport(&["safeArea", "safeAreaInsets", "keyboard"], &["Insets"])
        // Navigation: made by `nsh-js` (`navigation.rs`), declared by hand.
        .native_values(&["Stack", "Screen", "Modal", "createNavigator"]);
    registry
}

/// The specs of `nsh/base`, for the generated JS glue.
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
                "KeyboardAvoidingView",
                "SafeAreaView",
                "StatusBar"
            ]
        );
        let modules: Vec<_> = registry.module_specs().iter().map(|m| m.name).collect();
        assert_eq!(modules, ["ActionSheet", "Alert", "Keyboard", "StatusBar"]);
    }
}
