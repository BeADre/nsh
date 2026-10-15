//! The components of `nsh/ui/mobile`, written in Rust.
//!
//! Each component renders nsh's primitives ([`nsh_core::component`]): one
//! implementation draws the same on iOS and Android, lays out in the core's
//! pass and handles its own interaction natively. Its JS face (the
//! `nsh/ui/mobile` export and its TypeScript declarations) is generated from
//! its [`Spec`](nsh_core::component::Spec) by the bundler. See README.md for
//! how to write one.

mod button;
mod card;

use nsh_core::component::{Registry, Spec};

pub use crate::{button::Button, card::Card};

/// Every component of the mobile set, for an engine to render.
pub fn registry() -> Registry {
    let mut registry = Registry::new();
    registry.register::<Button>().register::<Card>();
    registry
}

/// The specs of the mobile set, for the generated JS glue.
pub fn specs() -> Vec<&'static Spec> {
    registry().specs()
}
