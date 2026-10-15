//! Components written in Rust: the UI sets' components (`nsh-ui-mobile`),
//! run by the core on the engine thread.
//!
//! A [`Component`] renders a tree of elements ([`view`], [`text`],
//! [`image`], other components) styled with typed methods ([`Styled`]:
//! `.px_4()`, `.rounded_xl()`, `.bg(theme.primary)`) that lower into the
//! same rules as Tailwind classes. The core expands the tree into nodes of
//! its own, diffs it against the previous render and lays it out in the same
//! pass as everything else, so a component sizes like any view, with no
//! round trip. Events on its elements go to the component first
//! ([`RenderCx::listener`]); it decides what reaches JS ([`EventCx::emit`])
//! and calls [`EventCx::notify`] to render again.
//!
//! Props are a struct (`#[derive(Props)]`), events an enum
//! (`#[derive(Event)]`); the JS glue and its TypeScript declarations are
//! generated from them ([`typescript`]). In JS a component is one host node:
//! the root of its expansion. The app's `class` is appended to the root's
//! styles and its `style` stays the root's inline style. Colors come from
//! the [`Theme`] (gpui-kit's fields), with a light and a dark value each.
//!
//! Node ids: JS allocates even ids, the core odd ones for the nodes it
//! expands, so both stay dense. How the core runs components:
//! `docs/dev/components.md`.

mod component;
mod element;
mod style;
mod theme;
mod value;

pub use nsh_macros::{Event, PropEnum, Props};

pub(crate) use self::{
    component::AnyInstance,
    element::{ElKind, ErasedEventCx},
};
pub use self::{
    component::{
        Component, ComponentSpec, Event, EventSpec, NoEvent, PropKind, PropSpec, Props, Registry,
        typescript,
    },
    element::{
        Accessible, AnyElement, Attrs, Children, ComponentElement, ElNode, Element, EventCx,
        FluentBuilder, Image, Interactive, IntoElement, Listener, ParentElement, RenderCx, Slot,
        Text, View, image, text, view,
    },
    style::{AdaptiveColor, FontWeight, Length, StyleRefinement, Styled, px, relative, rgb, rgba},
    theme::{Theme, ThemeColors, ThemeError},
    value::{JsValue, PropError, SharedString},
};
pub use crate::{
    AccessibilityRole, AccessibilityState, EventKind, EventPayload, PointerEvents, style::Color,
};

/// Everything a component's module needs: `use nsh_core::ui::prelude::*;`.
pub mod prelude {
    pub use super::{
        AccessibilityRole, AccessibilityState, Accessible, AdaptiveColor, Children, Color,
        Component, ComponentElement, Element, Event, EventCx, EventKind, EventPayload,
        FluentBuilder, FontWeight, Interactive, IntoElement, Length, Listener, NoEvent,
        ParentElement, PointerEvents, PropEnum, Props, RenderCx, SharedString, Slot, Styled, Theme,
        image, px, relative, rgb, rgba, text, view,
    };
}

/// What the derive macros' code names.
#[doc(hidden)]
pub mod __private {
    pub use serde_json::Map as JsonMap;
}
