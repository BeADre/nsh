//! The elements a component renders ([`view`], [`text`], [`image`], other
//! components, slots and children) and the contexts of renders and
//! handlers.

use std::{any::Any, marker::PhantomData, rc::Rc};

use super::{
    Component, Event, SharedString, Theme,
    style::{StyleRefinement, Styled, Variant},
};
use crate::{
    AccessibilityProp, AccessibilityRole, AccessibilityState, EdgeInsets, EventKind, EventPayload,
    ImageSource, JsonValue, ListContentStyle, NativeProp, NodeId, PointerEvents, Prop, Tag,
};

/// A handler of `E` on an element, made by [`RenderCx::listener`]: it runs
/// with the component that rendered the element.
pub struct Listener<E: ?Sized>(pub(crate) Rc<ErasedListener<E>>);

/// A listener with the component's type erased (it downcasts the state).
pub(crate) type ErasedListener<E> = dyn Fn(&mut dyn Any, &E, &mut ErasedEventCx);

impl<E: ?Sized> Clone for Listener<E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// An event a handler emitted, with its type erased.
pub(crate) trait AnyEvent {
    fn name(&self) -> &'static str;
    fn data(&self) -> JsonValue;
    fn as_any(&self) -> &dyn Any;
}

impl<E: Event> AnyEvent for E {
    fn name(&self) -> &'static str {
        Event::name(self)
    }

    fn data(&self) -> JsonValue {
        Event::data(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// What handlers did, for the core.
#[derive(Default)]
pub(crate) struct ErasedEventCx {
    pub(crate) emits: Vec<Box<dyn AnyEvent>>,
    pub(crate) notify: bool,
}

/// A render's context: the theme, and listeners for the elements.
pub struct RenderCx<'a, C> {
    theme: &'a Theme,
    _component: PhantomData<fn(&mut C)>,
}

impl<'a, C: Component> RenderCx<'a, C> {
    pub(crate) fn new(theme: &'a Theme) -> Self {
        Self {
            theme,
            _component: PhantomData,
        }
    }

    /// The app's theme.
    pub fn theme(&self) -> &'a Theme {
        self.theme
    }

    /// A handler for an element of this render: `f` gets the component (its
    /// state), the event and an [`EventCx`] to emit events or render again.
    pub fn listener<E: ?Sized + 'static>(
        &self,
        f: impl Fn(&mut C, &E, &mut EventCx<'_, C>) + 'static,
    ) -> Listener<E> {
        Listener(Rc::new(move |this, event, cx| {
            if let Some(this) = this.downcast_mut::<C>() {
                f(
                    this,
                    event,
                    &mut EventCx {
                        inner: cx,
                        _component: PhantomData,
                    },
                );
            }
        }))
    }
}

/// A handler's context.
pub struct EventCx<'a, C: Component> {
    inner: &'a mut ErasedEventCx,
    _component: PhantomData<fn(&mut C)>,
}

impl<C: Component> EventCx<'_, C> {
    /// Sends `event` to the app (its `on<Event>` handler), or to the
    /// component that rendered this one.
    pub fn emit(&mut self, event: C::Event) {
        self.inner.emits.push(Box::new(event));
    }

    /// The component's state changed: render it again.
    pub fn notify(&mut self) {
        self.inner.notify = true;
    }
}

/// The props of an element the core diffs field by field (opaque).
#[doc(hidden)]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Attrs {
    pub(crate) disabled: bool,
    pub(crate) accessible: Option<bool>,
    pub(crate) label: Option<SharedString>,
    pub(crate) hint: Option<SharedString>,
    pub(crate) role: AccessibilityRole,
    pub(crate) state: AccessibilityState,
    pub(crate) pointer_events: PointerEvents,
    pub(crate) active_opacity: Option<f32>,
    pub(crate) number_of_lines: u32,
    pub(crate) src: Option<SharedString>,
}

impl Attrs {
    /// The props of the fields that differ from `old`, in a fixed order.
    pub(crate) fn changes(&self, old: &Attrs, out: &mut Vec<Prop>) {
        use AccessibilityProp as A;
        if self.disabled != old.disabled {
            out.push(Prop::Native(NativeProp::Disabled(self.disabled)));
        }
        if self.accessible != old.accessible {
            out.push(Prop::Accessibility(A::Accessible(self.accessible)));
        }
        if self.label != old.label {
            out.push(Prop::Accessibility(A::Label(
                self.label.as_ref().map(SharedString::to_arc),
            )));
        }
        if self.hint != old.hint {
            out.push(Prop::Accessibility(A::Hint(
                self.hint.as_ref().map(SharedString::to_arc),
            )));
        }
        if self.role != old.role {
            out.push(Prop::Accessibility(A::Role(self.role)));
        }
        if self.state != old.state {
            out.push(Prop::Accessibility(A::State(self.state)));
        }
        if self.pointer_events != old.pointer_events {
            out.push(Prop::PointerEvents(self.pointer_events));
        }
        if self.active_opacity != old.active_opacity {
            out.push(Prop::ActiveOpacity(self.active_opacity));
        }
        if self.number_of_lines != old.number_of_lines {
            out.push(Prop::NumberOfLines(self.number_of_lines));
        }
        if self.src != old.src {
            out.push(Prop::Source(self.src.as_ref().map(|uri| ImageSource {
                uri: uri.to_arc(),
                size: None,
            })));
        }
    }
}

/// A nested component's props, with their type erased.
pub(crate) trait AnyProps {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn AnyProps>;
}

impl<P: Clone + 'static> AnyProps for P {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyProps> {
        Box::new(self.clone())
    }
}

/// A nested component of a render.
pub(crate) struct ComponentEl {
    pub(crate) name: &'static str,
    pub(crate) props: Box<dyn AnyProps>,
    /// The rendering component's handler of its events.
    pub(crate) on_event: Option<Listener<dyn Any>>,
}

impl Clone for ComponentEl {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            props: self.props.clone_box(),
            on_event: self.on_event.clone(),
        }
    }
}

/// Places one piece of the app's content: a JSX child, or a list row.
pub(crate) type PlaceFn = Rc<dyn Fn(Child, Position) -> AnyElement>;

/// Where a JSX child or a list row is among its siblings (what a Solid row
/// knows from its index and the data's length).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Its index.
    pub index: usize,
    /// How many there are.
    pub count: usize,
}

impl Position {
    /// The first one.
    pub fn is_first(&self) -> bool {
        self.index == 0
    }

    /// The last one.
    pub fn is_last(&self) -> bool {
        self.index + 1 == self.count
    }
}

/// How a list slot is laid out (each set field is sent to the list).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct ListConfig {
    pub(crate) estimated: Option<f32>,
    pub(crate) columns: Option<u32>,
    pub(crate) horizontal: Option<bool>,
    pub(crate) inverted: Option<bool>,
    pub(crate) shows_indicator: Option<bool>,
    pub(crate) content: Option<ListContentStyle>,
}

impl ListConfig {
    /// The props of the fields that differ from `old`.
    pub(crate) fn changes(&self, old: &ListConfig, out: &mut Vec<Prop>) {
        if self.estimated != old.estimated {
            out.push(Prop::EstimatedItemSize(self.estimated.unwrap_or(50.0)));
        }
        if self.columns != old.columns {
            out.push(Prop::Columns(self.columns.unwrap_or(1)));
        }
        if self.horizontal != old.horizontal {
            out.push(Prop::Horizontal(self.horizontal.unwrap_or(false)));
        }
        if self.inverted != old.inverted {
            out.push(Prop::Inverted(self.inverted.unwrap_or(false)));
        }
        if self.shows_indicator != old.shows_indicator {
            out.push(Prop::ShowsIndicator(self.shows_indicator.unwrap_or(true)));
        }
        if self.content != old.content {
            out.push(Prop::ListContent(self.content.unwrap_or_default()));
        }
    }
}

/// A list slot as a render places it.
#[derive(Clone)]
pub(crate) struct ListEl {
    pub(crate) name: &'static str,
    pub(crate) config: ListConfig,
    pub(crate) row: PlaceFn,
}

/// What an element is beyond its tag.
#[derive(Clone)]
pub(crate) enum ElKind {
    Plain,
    /// The glue's node of slot `name`.
    Slot(&'static str),
    /// Where the JSX children go (a list row's content, in a row).
    Children,
    /// One JSX child (in [`ParentElement::children_each`]).
    JsChild(NodeId),
    /// Every JSX child, each placed by the function.
    EachChild(PlaceFn),
    /// The glue's list of an [`Items`] prop.
    List(Box<ListEl>),
    Component(ComponentEl),
}

/// An element as the core expands it (opaque).
#[doc(hidden)]
#[derive(Clone)]
pub struct ElNode {
    pub(crate) tag: Tag,
    pub(crate) key: Option<SharedString>,
    /// A text's content.
    pub(crate) text: Option<SharedString>,
    pub(crate) attrs: Attrs,
    pub(crate) style: StyleRefinement,
    pub(crate) handlers: Vec<(EventKind, Listener<EventPayload>)>,
    pub(crate) children: Vec<ElNode>,
    pub(crate) kind: ElKind,
}

impl ElNode {
    /// What a component that is not registered renders.
    pub(crate) fn empty_view() -> Self {
        Self::new(Tag::View)
    }

    fn new(tag: Tag) -> Self {
        Self {
            tag,
            key: None,
            text: None,
            attrs: Attrs::default(),
            style: StyleRefinement::default(),
            handlers: Vec::new(),
            children: Vec::new(),
            kind: ElKind::Plain,
        }
    }
}

/// Any element, its type erased.
pub struct AnyElement(pub(crate) ElNode);

/// What can be a child: elements, and strings (as texts).
pub trait IntoElement {
    /// As an element.
    fn into_element(self) -> AnyElement;
}

impl IntoElement for AnyElement {
    fn into_element(self) -> AnyElement {
        self
    }
}

impl IntoElement for SharedString {
    fn into_element(self) -> AnyElement {
        text(self).into_element()
    }
}

impl IntoElement for &'static str {
    fn into_element(self) -> AnyElement {
        text(self).into_element()
    }
}

impl IntoElement for String {
    fn into_element(self) -> AnyElement {
        text(self).into_element()
    }
}

/// Conditional building, as in GPUI.
pub trait FluentBuilder: Sized {
    /// `then(self)` when `condition` holds.
    fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        if condition { then(self) } else { self }
    }

    /// `then(self, value)` when `option` has one.
    fn when_some<T>(self, option: Option<T>, then: impl FnOnce(Self, T) -> Self) -> Self {
        match option {
            Some(v) => then(self, v),
            None => self,
        }
    }

    /// `f(self)`.
    fn map(self, f: impl FnOnce(Self) -> Self) -> Self {
        f(self)
    }
}

impl<T: Element> FluentBuilder for T {}

/// An element: what every element can do. Its other abilities are the
/// traits it implements ([`Styled`], [`Accessible`], [`Interactive`],
/// [`ParentElement`]).
pub trait Element: IntoElement + Sized {
    #[doc(hidden)]
    fn node(&mut self) -> &mut ElNode;

    /// Identifies the element among its siblings across renders (where
    /// their order changes).
    fn key(mut self, key: impl Into<SharedString>) -> Self {
        self.node().key = Some(key.into());
        self
    }
}

impl<T: Element> Styled for T {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.node().style
    }
}

/// Elements with children.
pub trait ParentElement: Element {
    /// Adds a child.
    fn child(mut self, child: impl IntoElement) -> Self {
        self.node().children.push(child.into_element().0);
        self
    }

    /// Adds children (an `Option` adds one or none).
    fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.node()
            .children
            .extend(children.into_iter().map(|c| c.into_element().0));
        self
    }

    /// Places slot `slot` here when the app passed it, as `place` styles it
    /// (the closure gets the slot's own node).
    fn slot(mut self, slot: Option<Slot>, place: impl FnOnce(SlotElement) -> SlotElement) -> Self {
        if let Some(slot) = slot {
            let mut node = ElNode::new(Tag::View);
            node.kind = ElKind::Slot(slot.name);
            self.node().children.push(place(SlotElement(node)).0);
        }
        self
    }

    /// Places the app's JSX children here one by one: `place(child, at)`
    /// wraps each (runs again when the children change).
    fn children_each<R: IntoElement>(
        mut self,
        children: Children,
        place: impl Fn(Child, Position) -> R + 'static,
    ) -> Self {
        let _ = children;
        let mut node = ElNode::new(Tag::View);
        node.kind = ElKind::EachChild(Rc::new(move |child, at| place(child, at).into_element()));
        self.node().children.push(node);
        self
    }

    /// Places the app's virtualized list of `items` here: `config` lays
    /// the list out (size, columns, padding), `row(content, at)` wraps
    /// each row the app renders (runs again when a cell shows another row
    /// or the number of rows changes). Only rows near the viewport exist;
    /// the core recycles them.
    fn list<R: IntoElement>(
        mut self,
        items: Items,
        config: impl FnOnce(ListElement) -> ListElement,
        row: impl Fn(Child, Position) -> R + 'static,
    ) -> Self {
        let mut node = ElNode::new(Tag::List);
        node.kind = ElKind::List(Box::new(ListEl {
            name: items.name,
            config: ListConfig::default(),
            row: Rc::new(move |content, at| row(content, at).into_element()),
        }));
        self.node().children.push(config(ListElement(node)).0);
        self
    }
}

/// Elements with an accessibility description.
pub trait Accessible: Element {
    /// Whether assistive technologies see it as one element.
    fn accessible(mut self, accessible: bool) -> Self {
        self.node().attrs.accessible = Some(accessible);
        self
    }

    /// What screen readers say.
    fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.node().attrs.label = Some(label.into());
        self
    }

    /// What happens when activated, for screen readers.
    fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.node().attrs.hint = Some(hint.into());
        self
    }

    /// What it is (a button, a header, ...).
    fn role(mut self, role: AccessibilityRole) -> Self {
        self.node().attrs.role = role;
        self
    }

    /// Its state for screen readers (`disabled` follows
    /// [`Interactive::disabled`]).
    fn accessibility_state(mut self, state: AccessibilityState) -> Self {
        let attrs = &mut self.node().attrs;
        attrs.state = AccessibilityState {
            disabled: attrs.state.disabled,
            ..state
        };
        self
    }
}

/// Elements that take touches.
pub trait Interactive: Element {
    /// Handles presses.
    fn on_press(self, listener: Listener<EventPayload>) -> Self {
        self.on(EventKind::Press, listener)
    }

    /// Handles native event `kind` (the element listens to it).
    fn on(mut self, kind: EventKind, listener: Listener<EventPayload>) -> Self {
        self.node().handlers.push((kind, listener));
        self
    }

    /// Styles while pressed (native: no render).
    fn active(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        let s = f(StyleRefinement::default());
        self.style().extend_as(Variant::Active, s);
        self
    }

    /// Styles while a pointer hovers it (iPad, Mac, mice).
    fn hover(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        let s = f(StyleRefinement::default());
        self.style().extend_as(Variant::Hover, s);
        self
    }

    /// Styles while focused.
    fn focus(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        let s = f(StyleRefinement::default());
        self.style().extend_as(Variant::Focus, s);
        self
    }

    /// No presses nor press feedback; screen readers say it is disabled.
    fn disabled(mut self, disabled: bool) -> Self {
        let attrs = &mut self.node().attrs;
        attrs.disabled = disabled;
        attrs.state.disabled = disabled;
        self
    }

    /// Which touches it and its children take.
    fn pointer_events(mut self, pointer_events: PointerEvents) -> Self {
        self.node().attrs.pointer_events = pointer_events;
        self
    }

    /// The opacity while pressed (a fade instead of `.active(…)` styles).
    fn active_opacity(mut self, opacity: f32) -> Self {
        self.node().attrs.active_opacity = Some(opacity);
        self
    }
}

macro_rules! element {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone)]
        pub struct $name(ElNode);

        impl Element for $name {
            fn node(&mut self) -> &mut ElNode {
                &mut self.0
            }
        }

        impl IntoElement for $name {
            fn into_element(self) -> AnyElement {
                AnyElement(self.0)
            }
        }

        impl Accessible for $name {}
    };
}

element! {
    /// A view (`<view>`), made by [`view`].
    View
}

element! {
    /// A text block (`<text>`), made by [`text`].
    Text
}

element! {
    /// An image (`<image>`), made by [`image`].
    Image
}

element! {
    /// A slot's node as a render places it ([`ParentElement::slot`]):
    /// style it like a view; its children are the app's.
    SlotElement
}

element! {
    /// A list as a render lays it out ([`ParentElement::list`]).
    ListElement
}

impl Interactive for View {}
impl ParentElement for View {}
impl Interactive for SlotElement {}

impl Text {
    /// At most `lines` lines (0: no limit), the last one truncated.
    pub fn number_of_lines(mut self, lines: u32) -> Self {
        self.0.attrs.number_of_lines = lines;
        self
    }
}

impl ListElement {
    fn config(&mut self) -> &mut ListConfig {
        match &mut self.0.kind {
            ElKind::List(list) => &mut list.config,
            _ => unreachable!("a ListElement is a list"),
        }
    }

    fn content(&mut self) -> &mut ListContentStyle {
        self.config().content.get_or_insert_default()
    }

    /// The size of rows not measured yet, in points (default 50).
    pub fn estimated_item_size(mut self, points: f32) -> Self {
        self.config().estimated = Some(points);
        self
    }

    /// Rows side by side (vertical lists).
    pub fn columns(mut self, columns: u32) -> Self {
        self.config().columns = Some(columns.max(1));
        self
    }

    /// Scrolls horizontally.
    pub fn horizontal(mut self) -> Self {
        self.config().horizontal = Some(true);
        self
    }

    /// Item 0 at the end (chats), starting scrolled to the end.
    pub fn inverted(mut self) -> Self {
        self.config().inverted = Some(true);
        self
    }

    /// Whether the scroll indicator shows (default yes).
    pub fn shows_indicator(mut self, shows: bool) -> Self {
        self.config().shows_indicator = Some(shows);
        self
    }

    /// Padding around the rows, in points.
    pub fn content_padding(mut self, points: f32) -> Self {
        self.content().padding = EdgeInsets::all(points);
        self
    }

    /// Horizontal padding around the rows.
    pub fn content_padding_x(mut self, points: f32) -> Self {
        let p = &mut self.content().padding;
        (p.left, p.right) = (points, points);
        self
    }

    /// Vertical padding around the rows.
    pub fn content_padding_y(mut self, points: f32) -> Self {
        let p = &mut self.content().padding;
        (p.top, p.bottom) = (points, points);
        self
    }

    /// Space between rows (columns in horizontal lists).
    pub fn row_gap(mut self, points: f32) -> Self {
        self.content().row_gap = points;
        self
    }

    /// Space between columns ([`Self::columns`]).
    pub fn column_gap(mut self, points: f32) -> Self {
        self.content().column_gap = points;
        self
    }
}

/// A named slot (an `Option<Slot>` prop): `Some` while the app passes
/// content for it. Placed with [`ParentElement::slot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    name: &'static str,
}

impl Slot {
    pub(crate) fn new(name: &'static str) -> Self {
        Self { name }
    }

    /// Its name (its prop's, in JS).
    pub fn name(&self) -> &'static str {
        self.name
    }
}

/// The app's virtualized rows (an `Items` prop): in JS the component takes
/// `each` and a row renderer like `List`. Placed with
/// [`ParentElement::list`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Items {
    name: &'static str,
}

impl Items {
    #[doc(hidden)]
    pub const fn __new(name: &'static str) -> Self {
        Self { name }
    }
}

/// A piece of the app's content to place: one JSX child
/// ([`ParentElement::children_each`]) or a list row's content
/// ([`ParentElement::list`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Child(ChildRef);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildRef {
    Js(NodeId),
    Row,
}

impl Child {
    pub(crate) fn js(id: NodeId) -> Self {
        Self(ChildRef::Js(id))
    }

    pub(crate) fn row() -> Self {
        Self(ChildRef::Row)
    }
}

impl IntoElement for Child {
    fn into_element(self) -> AnyElement {
        let mut node = ElNode::new(Tag::View);
        node.kind = match self.0 {
            ChildRef::Js(id) => ElKind::JsChild(id),
            ChildRef::Row => ElKind::Children,
        };
        AnyElement(node)
    }
}

/// A view.
pub fn view() -> View {
    View(ElNode::new(Tag::View))
}

/// A text block showing `content`.
pub fn text(content: impl Into<SharedString>) -> Text {
    let mut node = ElNode::new(Tag::Text);
    node.text = Some(content.into());
    Text(node)
}

/// An image loading `src` (a URL or an asset).
pub fn image(src: impl Into<SharedString>) -> Image {
    let mut node = ElNode::new(Tag::Image);
    node.attrs.src = Some(src.into());
    Image(node)
}

/// The JSX children of a component (a `Children` prop): rendered where the
/// render puts it, among the render's own children.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Children {
    present: bool,
}

impl Children {
    pub(crate) fn new(present: bool) -> Self {
        Self { present }
    }

    /// The app passed none.
    pub fn is_empty(&self) -> bool {
        !self.present
    }
}

impl IntoElement for Children {
    fn into_element(self) -> AnyElement {
        let mut node = ElNode::new(Tag::View);
        node.kind = ElKind::Children;
        AnyElement(node)
    }
}

/// Another component in a render ([`Component::element`]). Its styles go
/// after the component's own, like an app's classes.
pub struct ComponentElement<C: Component> {
    node: ElNode,
    _component: PhantomData<fn() -> C>,
}

impl<C: Component> ComponentElement<C> {
    pub(crate) fn new(props: C::Props) -> Self {
        let mut node = ElNode::new(Tag::View);
        node.kind = ElKind::Component(ComponentEl {
            name: C::NAME,
            props: Box::new(props),
            on_event: None,
        });
        Self {
            node,
            _component: PhantomData,
        }
    }

    /// Handles its events (they then do not reach the app).
    pub fn on_event(mut self, listener: Listener<C::Event>) -> Self {
        let inner = listener.0;
        if let ElKind::Component(c) = &mut self.node.kind {
            c.on_event = Some(Listener(Rc::new(move |this, event: &dyn Any, cx| {
                if let Some(event) = event.downcast_ref::<C::Event>() {
                    inner(this, event, cx);
                }
            })));
        }
        self
    }
}

impl<C: Component> Element for ComponentElement<C> {
    fn node(&mut self) -> &mut ElNode {
        &mut self.node
    }
}

impl<C: Component> IntoElement for ComponentElement<C> {
    fn into_element(self) -> AnyElement {
        AnyElement(self.node)
    }
}
