//! Typed declarations over the existing native widget lifecycle. The props and
//! events are NSH's own types; no parallel schema or Rust UI view is introduced.

use std::{marker::PhantomData, ops::Deref, rc::Rc};

use gpui::{AnyElement, App, Bounds, FocusHandle, Pixels, Window};
use nsh_core::{
    JsonValue,
    ui::{self, Event, Props, Registry},
};

use super::{ComponentContext, Components, NativeComponent};

/// A component's typed event emitter and its existing native subtree context.
pub struct TypedComponentContext<E: Event> {
    context: ComponentContext,
    event: PhantomData<fn() -> E>,
}
impl<E: Event> Clone for TypedComponentContext<E> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            event: PhantomData,
        }
    }
}
impl<E: Event> From<ComponentContext> for TypedComponentContext<E> {
    fn from(context: ComponentContext) -> Self {
        Self {
            context,
            event: PhantomData,
        }
    }
}
impl<E: Event> Deref for TypedComponentContext<E> {
    type Target = ComponentContext;
    fn deref(&self) -> &ComponentContext {
        &self.context
    }
}
impl<E: Event> TypedComponentContext<E> {
    /// The original context, including live children, slots and native sizing.
    pub fn raw(&self) -> &ComponentContext {
        &self.context
    }
    fn report_error(&self, message: String) {
        log::error!("{message}");
        self.context.emit("error", JsonValue::String(message));
    }
    /// Emits typed data through NSH's existing JS value codec. Invalid data
    /// reports an error through the ordinary native error event.
    pub fn emit(&self, event: E) {
        match ui::event_json(&event) {
            Ok(data) => self
                .context
                .emit(event.name(), data.unwrap_or(JsonValue::Null)),
            Err(error) => self.report_error(error.to_string()),
        }
    }
}

/// A real GPUI component with NSH-typed props. Implement only its retained
/// state, actual rendering and lifecycle hooks; decoding and declarations are
/// shared by every component.
pub trait TypedNativeComponent: 'static {
    /// JSON-value props, decoded with their existing NSH `JsValue` codec.
    type Props: Props;
    /// Typed JSON-value events; children and slots use the live tree instead.
    type Event: Event;
    /// Renders the original widget with the latest accepted typed props.
    fn render(
        &mut self,
        props: &Self::Props,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement>;
    /// Checks domain rules on the full candidate before it replaces accepted
    /// props. Return an error to reject this batch once and retain prior state.
    fn validate_props(&self, _props: &Self::Props) -> Result<(), ui::PropError> {
        Ok(())
    }
    /// Called once after a complete accepted batch, including unchanged
    /// acknowledged fields. Compare the widget's real state when applying a veto.
    fn props_changed(
        &mut self,
        _props: &Self::Props,
        _changed: &[String],
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }
    /// Existing fire-and-forget NodeRef commands; not a Rust API RPC channel.
    fn command(
        &mut self,
        _props: &Self::Props,
        _name: &str,
        _args: &JsonValue,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }
    /// Actual composed placement, or `None` after removal.
    fn placed(&mut self, _bounds: Option<Bounds<Pixels>>) {}
    /// The original widget's primary focus, including programmatic-only focus.
    /// Override `focus_handles` to exclude it from sequential Tab traversal.
    fn focus_handle(&self) -> Option<FocusHandle> {
        None
    }
    /// Sequential focus priority. Zero preserves native tree order, positive
    /// values sort first, and negative values are not keyboard Tab stops.
    fn tab_index(&self) -> isize {
        0
    }
    /// The widget's ordered Tab stops, excluding disabled entries.
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focus_handle().into_iter().collect()
    }
}

struct TypedComponent<C: TypedNativeComponent> {
    component: C,
    props: ui::NativeProps<C::Props>,
    context: TypedComponentContext<C::Event>,
    /// All delivered fields, including failed decodes, so rejection can undo
    /// a widget mutation without accepting a malformed prop candidate.
    delivered: Vec<String>,
}
impl<C: TypedNativeComponent> TypedComponent<C> {
    fn commit(&mut self, window: &mut Window, cx: &mut App) {
        let delivered = std::mem::take(&mut self.delivered);
        match self
            .props
            .commit_with(|props| self.component.validate_props(props))
        {
            Ok(fields) if !fields.is_empty() => {
                self.component
                    .props_changed(self.props.props(), &fields, window, cx);
            }
            Ok(_) => {}
            Err(error) => {
                // Native controls may already have edited retained state before
                // the owner acknowledges it. Reapply the accepted snapshot once.
                self.component
                    .props_changed(self.props.props(), &delivered, window, cx);
                self.context.report_error(error.to_string());
            }
        }
    }
}
impl<C: TypedNativeComponent> NativeComponent for TypedComponent<C> {
    fn render(&mut self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.commit(window, cx);
        self.component.render(self.props.props(), window, cx)
    }
    fn placed(&mut self, bounds: Option<Bounds<Pixels>>) {
        self.component.placed(bounds);
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.component.focus_handle()
    }
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.component.focus_handles()
    }
    fn tab_index(&self) -> isize {
        self.component.tab_index()
    }
    fn command(&mut self, name: &str, args: &JsonValue, window: &mut Window, cx: &mut App) {
        self.commit(window, cx);
        self.component
            .command(self.props.props(), name, args, window, cx);
    }
    fn set_prop(&mut self, name: &str, value: Option<&JsonValue>, _: &mut Window, _: &mut App) {
        if !self.delivered.iter().any(|field| field == name) {
            self.delivered.push(name.to_owned());
        }
        // Conversion still returns Result for direct callers. The wrapper
        // reports the first failure at this batch's commit boundary only.
        let _ = self.props.set_json(name, value);
    }
}

/// Wraps a typed state hook for an existing (including legacy alias) factory.
pub fn typed_component<C: TypedNativeComponent>(
    component: C,
    context: TypedComponentContext<C::Event>,
) -> Box<dyn NativeComponent> {
    Box::new(TypedComponent {
        component,
        props: ui::NativeProps::new(),
        context,
        delivered: Vec::new(),
    })
}
impl Components {
    /// Registers one native factory and its generated JS/TS declarations.
    pub fn register_typed<C: TypedNativeComponent>(
        &self,
        registry: &mut Registry,
        name: &'static str,
        export: &'static str,
        factory: impl Fn(TypedComponentContext<C::Event>) -> C + 'static,
    ) -> &Self {
        registry.register_native::<C::Props, C::Event>(name, export);
        self.register(
            name,
            Rc::new(move |raw| {
                let context = TypedComponentContext::from(raw);
                typed_component(factory(context.clone()), context)
            }),
        );
        self
    }
}

/// The shared declaration entry point for metadata-only tools and real native
/// hosts. A binding module registers its typed factory once against either a
/// `Registry` or a `(&Components, &mut Registry)` host pair.
pub trait NativeRegistry {
    /// Registers the typed props/events and, in a host, its original factory.
    fn register<C: TypedNativeComponent>(
        &mut self,
        name: &'static str,
        export: &'static str,
        factory: impl Fn(TypedComponentContext<C::Event>) -> C + 'static,
    );
    /// The UI set's declarations and event ownership metadata.
    fn metadata(&mut self) -> &mut Registry;
}

impl NativeRegistry for Registry {
    fn register<C: TypedNativeComponent>(
        &mut self,
        name: &'static str,
        export: &'static str,
        _factory: impl Fn(TypedComponentContext<C::Event>) -> C + 'static,
    ) {
        self.register_native::<C::Props, C::Event>(name, export);
    }
    fn metadata(&mut self) -> &mut Registry {
        self
    }
}

impl NativeRegistry for (&Components, &mut Registry) {
    fn register<C: TypedNativeComponent>(
        &mut self,
        name: &'static str,
        export: &'static str,
        factory: impl Fn(TypedComponentContext<C::Event>) -> C + 'static,
    ) {
        self.0.register_typed::<C>(self.1, name, export, factory);
    }
    fn metadata(&mut self) -> &mut Registry {
        self.1
    }
}
