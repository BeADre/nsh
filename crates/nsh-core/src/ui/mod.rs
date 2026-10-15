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
//! ([`RenderCx::listener`]); it decides what reaches JS ([`Context::emit`])
//! and calls [`Context::notify`] to render again. State lives in the
//! component (`Self`), set up from the first props ([`Component::setup`]),
//! changed by listeners, timers and async tasks ([`Context::spawn`]); a render
//! subscribes to what it reads of the environment ([`RenderCx::window`]).
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
mod env;
mod style;
mod task;
mod theme;
mod value;

pub use nsh_macros::{Event, PropEnum, Props, methods};

pub(crate) use self::{
    component::AnyInstance,
    element::{ElKind, ErasedCx, ListConfig, PlaceFn},
    task::Executor,
};
pub use self::{
    component::{
        Component, ComponentSpec, Event, EventSpec, Method, MethodSpec, NoEvent, Param, PropKind,
        PropSpec, Props, Registry, typescript,
    },
    element::{
        Accessible, AnyElement, Attrs, Child, Children, ComponentElement, Context, ElNode, Element,
        FluentBuilder, Image, Interactive, IntoElement, Items, ListElement, Listener,
        ParentElement, Position, RenderCx, Slot, SlotElement, Text, View, image, text, view,
    },
    env::Environment,
    style::{AdaptiveColor, FontWeight, Length, StyleRefinement, Styled, px, relative, rgb, rgba},
    task::{AsyncCx, Task, Updated, WeakComponent},
    theme::{Theme, ThemeColors, ThemeError},
    value::{JsValue, PropError, SharedString},
};
pub use crate::{
    AccessibilityRole, AccessibilityState, EventKind, EventPayload, PointerEvents, style::Color,
};

/// Everything a component's module needs: `use nsh_core::ui::prelude::*;`.
pub mod prelude {
    pub use super::{
        AccessibilityRole, AccessibilityState, Accessible, AdaptiveColor, Child, Children, Color,
        Component, ComponentElement, Context, Element, Environment, Event, EventKind, EventPayload,
        FluentBuilder, FontWeight, Interactive, IntoElement, Items, Length, Listener, Method,
        NoEvent, ParentElement, PointerEvents, Position, PropEnum, Props, RenderCx, SharedString,
        Slot, Styled, Task, Theme, WeakComponent, image, methods, px, relative, rgb, rgba, text,
        view,
    };
}

/// What the derive macros' code names.
#[doc(hidden)]
pub mod __private {
    pub use serde_json::Map as JsonMap;
}
