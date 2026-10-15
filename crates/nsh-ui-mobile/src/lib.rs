//! The components of `nsh/ui/mobile`, written in Rust.
//!
//! Each component renders nsh's elements with typed styles and theme colors
//! ([`nsh_core::ui`]): one implementation draws the same on iOS and Android,
//! lays out in the core's pass and handles its own interaction natively.
//! Its JS face (the `nsh/ui/mobile` export and its TypeScript declarations)
//! is generated from its props and events by the bundler. See README.md for
//! how to write one.

mod button;
mod card;
mod group;
mod list_view;

use std::sync::Arc;

use nsh_core::ui::{ComponentSpec, Registry};

pub use crate::{
    button::{Button, ButtonEvent, ButtonProps, ButtonVariant},
    card::{Card, CardProps, CardVariant},
    group::{Group, GroupProps},
    list_view::{ListView, ListViewProps},
};

/// Every component of the mobile set, for an engine to render.
pub fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register::<Button>()
        .register::<Card>()
        .register::<Group>()
        .register::<ListView>();
    registry
}

/// The specs of the mobile set, for the generated JS glue.
pub fn specs() -> Vec<Arc<ComponentSpec>> {
    registry().specs()
}
