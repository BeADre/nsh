//! Components written in Rust: the UI sets' components (`nsh-ui-mobile`),
//! run by the core on the engine thread.
//!
//! A component renders a tree of primitive elements ([`El`]: views, texts,
//! images, other components) from its props and its own state. The core
//! expands that tree into nodes of its own, diffs it against the previous
//! render and lays it out in the same pass as everything else, so a
//! component sizes like any view, with no round trip. Events on its parts
//! go to the component first ([`El::on`]); it decides what reaches JS
//! ([`EventCx::emit`]), and calls [`EventCx::notify`] to render again.
//!
//! In JS a component is one host node (`<component>`, created by the glue
//! generated from its [`Spec`]): the root of its expansion. Its own props
//! arrive as `$name` values ([`Props`]); the app's `class` is appended to the
//! root's classes and its `style` stays the root's inline style, as for a
//! component written in TS. JS children go where the component's render
//! puts them ([`El::holds_children`]).
//!
//! Node ids: JS allocates even ids, the core odd ones for the nodes it
//! expands, so both stay dense.

use std::{any::Any, fmt, marker::PhantomData, rc::Rc, sync::Arc};

use rustc_hash::FxHashMap;

use crate::{
    AccessibilityRole, AccessibilityState, EventKind, EventPayload, ImageSource, JsonValue,
    PointerEvents, Prop, Tag,
};

/// The type of a component prop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropType {
    /// A string.
    String,
    /// A number.
    Number,
    /// A boolean.
    Bool,
    /// One of these strings.
    Enum(&'static [&'static str]),
    /// A CSS color string.
    Color,
    /// Any JSON value.
    Json,
}

/// One prop of a component.
#[derive(Debug, Clone, Copy)]
pub struct PropSpec {
    /// Its name in JS (`title`).
    pub name: &'static str,
    /// Its type.
    pub ty: PropType,
    /// Its default as JSON (`"primary"`, `false`); `None`: required.
    pub default: Option<&'static str>,
    /// Documentation, shown by editors.
    pub doc: &'static str,
}

/// One event of a component.
#[derive(Debug, Clone, Copy)]
pub struct EventSpec {
    /// Its name (`press`: the JS handler is `onPress`).
    pub name: &'static str,
    /// The TypeScript type of its data (`void`, `{ value: number }`).
    pub data: &'static str,
    /// Documentation.
    pub doc: &'static str,
}

/// A named slot of a component: JSX the app passes in a prop of that name
/// (`<Card header={…}>`), placed where the render puts [`El::slot`].
#[derive(Debug, Clone, Copy)]
pub struct SlotSpec {
    /// Its name and prop (`header`).
    pub name: &'static str,
    /// Documentation.
    pub doc: &'static str,
}

/// What a component is, for JS: its name, props, events and whether it takes
/// children. Its glue and declarations are generated from it
/// ([`typescript`]).
#[derive(Debug)]
pub struct Spec {
    /// Its registered name (`nsh.button`).
    pub name: &'static str,
    /// Its JS export (`Button`).
    pub export: &'static str,
    /// Documentation.
    pub doc: &'static str,
    /// Its props.
    pub props: &'static [PropSpec],
    /// Its events.
    pub events: &'static [EventSpec],
    /// It takes JSX children.
    pub children: bool,
    /// Its named slots.
    pub slots: &'static [SlotSpec],
}

/// A component written in Rust. `Default` makes a new instance.
pub trait Component: Any + Default {
    /// What it is.
    fn spec() -> &'static Spec
    where
        Self: Sized;

    /// Its element tree for `props`. Runs again when a prop changed or the
    /// component asked for it ([`EventCx::notify`]); keep it cheap and
    /// without side effects.
    fn render(&self, props: &Props, cx: &mut RenderCx) -> El<Self>
    where
        Self: Sized;

    /// JS called `ref.command(name, args)`.
    fn command(&mut self, name: &str, args: &JsonValue, cx: &mut EventCx) {
        let _ = (name, args, cx);
    }
}

/// A prop value, stored for cheap reads: strings are shared, so a render
/// reading one only bumps a reference count.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PropValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(Arc<str>),
    Json(JsonValue),
}

impl From<JsonValue> for PropValue {
    fn from(v: JsonValue) -> Self {
        match v {
            JsonValue::Null => PropValue::Null,
            JsonValue::Bool(b) => PropValue::Bool(b),
            JsonValue::Number(n) => PropValue::Number(n.as_f64().unwrap_or(0.0)),
            JsonValue::String(s) => PropValue::Str(s.into()),
            v => PropValue::Json(v),
        }
    }
}

impl PropValue {
    fn to_json(&self) -> JsonValue {
        match self {
            PropValue::Null => JsonValue::Null,
            PropValue::Bool(b) => JsonValue::Bool(*b),
            PropValue::Number(n) => {
                serde_json::Number::from_f64(*n).map_or(JsonValue::Null, JsonValue::Number)
            }
            PropValue::Str(s) => JsonValue::String(s.to_string()),
            PropValue::Json(v) => v.clone(),
        }
    }
}

/// The empty string, shared.
fn empty() -> Arc<str> {
    static EMPTY: std::sync::LazyLock<Arc<str>> = std::sync::LazyLock::new(|| Arc::from(""));
    EMPTY.clone()
}

/// A spec's props' defaults, parsed once per registered component.
pub(crate) type Defaults = Arc<[Option<PropValue>]>;

fn defaults(spec: &Spec) -> Defaults {
    spec.props
        .iter()
        .map(|p| {
            p.default
                .and_then(|d| serde_json::from_str::<JsonValue>(d).ok())
                .map(PropValue::from)
        })
        .collect()
}

/// A component's props as JS set them, with the spec's defaults. Props are
/// stored by their index in the spec (components have a handful).
pub struct Props {
    values: Vec<Option<PropValue>>,
    spec: Option<&'static Spec>,
    defaults: Defaults,
}

impl fmt::Debug for Props {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = self.spec.map_or(&[][..], |s| s.props);
        f.debug_map()
            .entries(
                names
                    .iter()
                    .zip(&self.values)
                    .filter_map(|(p, v)| Some((p.name, v.as_ref()?))),
            )
            .finish()
    }
}

impl Props {
    pub(crate) fn new(spec: Option<&'static Spec>, defaults: Defaults) -> Self {
        Self {
            values: vec![None; spec.map_or(0, |s| s.props.len())],
            spec,
            defaults,
        }
    }

    fn index(&self, name: &str) -> Option<usize> {
        self.spec?.props.iter().position(|p| p.name == name)
    }

    /// Sets prop `name`; `None` removes it. Returns whether it changed (a
    /// name the spec does not have is ignored).
    pub(crate) fn set(&mut self, name: &str, value: Option<JsonValue>) -> bool {
        let Some(i) = self.index(name) else {
            return false;
        };
        let value = value.map(PropValue::from);
        if self.values[i] == value {
            return false;
        }
        self.values[i] = value;
        true
    }

    fn value(&self, name: &str) -> Option<&PropValue> {
        let i = self.index(name)?;
        self.values[i].as_ref().or(self.defaults.get(i)?.as_ref())
    }

    /// Prop `name` as JS set it, else its default.
    pub fn get(&self, name: &str) -> Option<JsonValue> {
        self.value(name).map(PropValue::to_json)
    }

    /// A string prop (`""` when unset).
    pub fn str(&self, name: &str) -> Arc<str> {
        match self.value(name) {
            Some(PropValue::Str(s)) => s.clone(),
            _ => empty(),
        }
    }

    /// A boolean prop (`false` when unset).
    pub fn bool(&self, name: &str) -> bool {
        matches!(self.value(name), Some(PropValue::Bool(true)))
    }

    /// A number prop (0 when unset).
    pub fn number(&self, name: &str) -> f64 {
        match self.value(name) {
            Some(PropValue::Number(n)) => *n,
            _ => 0.0,
        }
    }
}

/// What a render can ask about where it renders.
#[derive(Debug, Default)]
pub struct RenderCx {
    pub(crate) has_children: bool,
    /// Slots the app filled.
    pub(crate) filled: Vec<Arc<str>>,
}

impl RenderCx {
    /// JS gave the component children.
    pub fn has_children(&self) -> bool {
        self.has_children
    }

    /// The app passed something for slot `name`.
    pub fn has_slot(&self, name: &str) -> bool {
        self.filled.iter().any(|s| &**s == name)
    }
}

/// What a component does in response to an event or a command.
#[derive(Debug, Default)]
pub struct EventCx {
    pub(crate) emits: Vec<(Arc<str>, JsonValue)>,
    pub(crate) notify: bool,
}

impl EventCx {
    /// Sends event `name` to JS: the handler `on<Name>` gets `data`.
    pub fn emit(&mut self, name: &str, data: JsonValue) {
        self.emits.push((name.into(), data));
    }

    /// Renders the component again (its state changed).
    pub fn notify(&mut self) {
        self.notify = true;
    }
}

/// A nested component of a render: its name and props.
pub(crate) type Nested = (Arc<str>, Vec<(Arc<str>, JsonValue)>);

/// An event handler of a part, with the component erased.
pub(crate) type Handler = Rc<dyn Fn(&mut dyn Any, &EventPayload, &mut EventCx)>;

/// A run of Tailwind classes ([`El::class`]): static strings cost nothing,
/// dynamic ones are shared.
#[derive(Debug, Clone)]
pub enum ClassStr {
    /// Classes written in the component's code.
    Static(&'static str),
    /// Classes computed at render time.
    Shared(Arc<str>),
}

impl ClassStr {
    fn as_str(&self) -> &str {
        match self {
            ClassStr::Static(s) => s,
            ClassStr::Shared(s) => s,
        }
    }
}

impl PartialEq for ClassStr {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = (self.as_str(), other.as_str());
        std::ptr::eq(a, b) || a == b
    }
}

impl From<&'static str> for ClassStr {
    fn from(s: &'static str) -> Self {
        ClassStr::Static(s)
    }
}

impl From<String> for ClassStr {
    fn from(s: String) -> Self {
        ClassStr::Shared(s.into())
    }
}

impl From<Arc<str>> for ClassStr {
    fn from(s: Arc<str>) -> Self {
        ClassStr::Shared(s)
    }
}

/// An element's classes, in order; joined only when they changed.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Classes(pub(crate) Vec<ClassStr>);

impl Classes {
    pub(crate) fn push(&mut self, classes: ClassStr) {
        let empty = match &classes {
            ClassStr::Static(s) => s.is_empty(),
            ClassStr::Shared(s) => s.trim().is_empty(),
        };
        if !empty {
            self.0.push(classes);
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn joined(&self) -> Box<str> {
        match &self.0[..] {
            [] => "".into(),
            [one] => one.as_str().into(),
            parts => {
                let mut out =
                    String::with_capacity(parts.iter().map(|p| p.as_str().len() + 1).sum());
                for (i, p) in parts.iter().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    out.push_str(p.as_str());
                }
                out.into()
            }
        }
    }
}

/// The props of an element the core diffs field by field.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Attrs {
    pub(crate) class: Classes,
    pub(crate) disabled: bool,
    pub(crate) accessible: Option<bool>,
    pub(crate) label: Option<Arc<str>>,
    pub(crate) hint: Option<Arc<str>>,
    pub(crate) role: AccessibilityRole,
    pub(crate) state: AccessibilityState,
    pub(crate) pointer_events: PointerEvents,
    pub(crate) active_opacity: Option<f32>,
    pub(crate) number_of_lines: u32,
    pub(crate) src: Option<Arc<str>>,
    /// Props without a field: set when they change, never cleared.
    pub(crate) extra: Vec<Prop>,
}

/// An element without its component type.
pub(crate) struct ElNode {
    pub(crate) tag: Tag,
    pub(crate) key: Option<Arc<str>>,
    /// A text's content.
    pub(crate) text: Option<Arc<str>>,
    pub(crate) attrs: Attrs,
    pub(crate) handlers: Vec<(EventKind, Handler)>,
    pub(crate) children: Vec<ElNode>,
    /// JS children go here (and the element has none of its own).
    pub(crate) holds_children: bool,
    /// A nested component: its name and props.
    pub(crate) component: Option<Nested>,
    /// This element is slot `name`'s JSX (the glue's node for it).
    pub(crate) slot: Option<Arc<str>>,
}

/// An element of a component's render, built with [`view`], [`text`],
/// [`image`] or [`component`].
pub struct El<C> {
    pub(crate) node: ElNode,
    _component: PhantomData<fn(&mut C)>,
}

impl<C> fmt::Debug for El<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("El")
            .field("tag", &self.node.tag)
            .finish_non_exhaustive()
    }
}

fn el<C>(tag: Tag) -> El<C> {
    El {
        node: ElNode {
            tag,
            key: None,
            text: None,
            attrs: Attrs::default(),
            handlers: Vec::new(),
            children: Vec::new(),
            holds_children: false,
            component: None,
            slot: None,
        },
        _component: PhantomData,
    }
}

/// A view (`<view>`).
pub fn view<C>() -> El<C> {
    el(Tag::View)
}

/// A text block (`<text>`) showing `content`.
pub fn text<C>(content: impl Into<Arc<str>>) -> El<C> {
    let mut e = el(Tag::Text);
    e.node.text = Some(content.into());
    e
}

/// An image (`<image src>`).
pub fn image<C>(src: impl Into<Arc<str>>) -> El<C> {
    let mut e = el(Tag::Image);
    e.node.attrs.src = Some(src.into());
    e
}

/// Another component, by its registered name, with these props.
pub fn component<C>(
    name: &str,
    props: impl IntoIterator<Item = (&'static str, JsonValue)>,
) -> El<C> {
    let mut e = el(Tag::View);
    e.node.component = Some((
        name.into(),
        props.into_iter().map(|(k, v)| (Arc::from(k), v)).collect(),
    ));
    e
}

impl<C: 'static> El<C> {
    /// Tailwind classes; several calls append. Pass static strings where
    /// possible (`"px-4 py-3"`): they are neither copied nor joined until
    /// they change.
    pub fn class(mut self, classes: impl Into<ClassStr>) -> Self {
        self.node.attrs.class.push(classes.into());
        self
    }

    /// Identifies the element among its siblings across renders (in lists
    /// of elements whose order changes).
    pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
        self.node.key = Some(key.into());
        self
    }

    /// No presses, `disabled:` classes.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.node.attrs.disabled = disabled;
        self
    }

    /// `accessible`.
    pub fn accessible(mut self, accessible: bool) -> Self {
        self.node.attrs.accessible = Some(accessible);
        self
    }

    /// `accessibilityLabel`.
    pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
        self.node.attrs.label = Some(label.into());
        self
    }

    /// `accessibilityHint`.
    pub fn hint(mut self, hint: impl Into<Arc<str>>) -> Self {
        self.node.attrs.hint = Some(hint.into());
        self
    }

    /// `accessibilityRole`.
    pub fn role(mut self, role: AccessibilityRole) -> Self {
        self.node.attrs.role = role;
        self
    }

    /// `accessibilityState`.
    pub fn state(mut self, state: AccessibilityState) -> Self {
        self.node.attrs.state = state;
        self
    }

    /// `pointerEvents`.
    pub fn pointer_events(mut self, pointer_events: PointerEvents) -> Self {
        self.node.attrs.pointer_events = pointer_events;
        self
    }

    /// `activeOpacity`.
    pub fn active_opacity(mut self, opacity: f32) -> Self {
        self.node.attrs.active_opacity = Some(opacity);
        self
    }

    /// A text's `numberOfLines` (0: no limit).
    pub fn number_of_lines(mut self, lines: u32) -> Self {
        self.node.attrs.number_of_lines = lines;
        self
    }

    /// Any other prop: set when it changes, never cleared (pass the
    /// default to undo it).
    pub fn prop(mut self, prop: Prop) -> Self {
        self.node.attrs.extra.push(prop);
        self
    }

    /// Handles native event `kind` on this element (the element listens to
    /// it).
    pub fn on(
        mut self,
        kind: EventKind,
        handler: impl Fn(&mut C, &EventPayload, &mut EventCx) + 'static,
    ) -> Self {
        let handler: Handler = Rc::new(move |this, payload, cx| {
            if let Some(this) = this.downcast_mut::<C>() {
                handler(this, payload, cx);
            }
        });
        self.node.handlers.push((kind, handler));
        self
    }

    /// Handles presses (`onPress`).
    pub fn on_press(self, handler: impl Fn(&mut C, &mut EventCx) + 'static) -> Self {
        self.on(EventKind::Press, move |this, _, cx| handler(this, cx))
    }

    /// Adds a child.
    pub fn child(mut self, child: El<C>) -> Self {
        self.node.children.push(child.node);
        self
    }

    /// Adds children.
    pub fn children(mut self, children: impl IntoIterator<Item = El<C>>) -> Self {
        self.node
            .children
            .extend(children.into_iter().map(|c| c.node));
        self
    }

    /// The component's JSX children go here; the element has none of its
    /// own.
    pub fn holds_children(mut self) -> Self {
        self.node.holds_children = true;
        self
    }

    /// This element is slot `name`: the JSX the app passed for it (its own
    /// children). Built on [`view`]; rendered only while the slot exists.
    pub fn slot(mut self, name: &str) -> Self {
        self.node.slot = Some(name.into());
        self
    }
}

impl Attrs {
    /// The props of the fields that differ from `old`, in a fixed order.
    pub(crate) fn changes(&self, old: &Attrs, out: &mut Vec<Prop>) {
        use crate::{AccessibilityProp as A, NativeProp};
        if self.class != old.class {
            out.push(Prop::Class(self.class.joined()));
        }
        if self.disabled != old.disabled {
            out.push(Prop::Native(NativeProp::Disabled(self.disabled)));
        }
        if self.accessible != old.accessible {
            out.push(Prop::Accessibility(A::Accessible(self.accessible)));
        }
        if self.label != old.label {
            out.push(Prop::Accessibility(A::Label(self.label.clone())));
        }
        if self.hint != old.hint {
            out.push(Prop::Accessibility(A::Hint(self.hint.clone())));
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
                uri: uri.clone(),
                size: None,
            })));
        }
        for prop in &self.extra {
            if !old.extra.contains(prop) {
                out.push(prop.clone());
            }
        }
    }
}

/// A component with its type erased.
pub(crate) trait AnyComponent {
    fn render(&self, props: &Props, cx: &mut RenderCx) -> ElNode;
    fn command(&mut self, name: &str, args: &JsonValue, cx: &mut EventCx);
    fn as_any(&mut self) -> &mut dyn Any;
}

impl<C: Component> AnyComponent for C {
    fn render(&self, props: &Props, cx: &mut RenderCx) -> ElNode {
        Component::render(self, props, cx).node
    }

    fn command(&mut self, name: &str, args: &JsonValue, cx: &mut EventCx) {
        Component::command(self, name, args, cx)
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone)]
struct ComponentType {
    spec: &'static Spec,
    defaults: Defaults,
    make: fn() -> Box<dyn AnyComponent>,
}

/// The components a surface can render, by name. Cheap to clone.
#[derive(Clone, Default)]
pub struct Registry {
    types: Arc<FxHashMap<&'static str, ComponentType>>,
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.types.keys()).finish()
    }
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers component `C` under its spec's name.
    pub fn register<C: Component>(&mut self) -> &mut Self {
        fn make<C: Component>() -> Box<dyn AnyComponent> {
            Box::new(C::default())
        }
        Arc::make_mut(&mut self.types).insert(
            C::spec().name,
            ComponentType {
                spec: C::spec(),
                defaults: defaults(C::spec()),
                make: make::<C>,
            },
        );
        self
    }

    /// The registered components' specs, by name.
    pub fn specs(&self) -> Vec<&'static Spec> {
        let mut specs: Vec<_> = self.types.values().map(|t| t.spec).collect();
        specs.sort_by_key(|s| s.name);
        specs
    }

    /// A new instance of component `name`, with its props.
    pub(crate) fn make(&self, name: &str) -> Option<(Props, Box<dyn AnyComponent>)> {
        self.types
            .get(name)
            .map(|t| (Props::new(Some(t.spec), t.defaults.clone()), (t.make)()))
    }
}

/// The JS glue and TypeScript declarations of `specs`: one component per
/// spec, exported by its [`Spec::export`] name, built on
/// `nsh/_core`'s `component` helper.
pub fn typescript(specs: &[&Spec]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "// Generated from the specs of the Rust components (nsh_core::component::typescript).\n\n\
         import type { JSX } from \"nsh\";\n\
         import { component, type ComponentHostProps } from \"nsh/_core\";\n",
    );
    for spec in specs {
        let _ = write!(
            out,
            "\nexport interface {}Props extends ComponentHostProps {{\n",
            spec.export
        );
        for p in spec.props {
            let ty = match p.ty {
                PropType::String | PropType::Color => "string".to_owned(),
                PropType::Number => "number".to_owned(),
                PropType::Bool => "boolean".to_owned(),
                PropType::Json => "unknown".to_owned(),
                PropType::Enum(values) => values
                    .iter()
                    .map(|v| format!("{v:?}"))
                    .collect::<Vec<_>>()
                    .join(" | "),
            };
            let optional = if p.default.is_some() { "?" } else { "" };
            if !p.doc.is_empty() {
                let _ = writeln!(out, "  /** {} */", p.doc);
            }
            let _ = writeln!(out, "  {}{optional}: {ty};", p.name);
        }
        for e in spec.events {
            if !e.doc.is_empty() {
                let _ = writeln!(out, "  /** {} */", e.doc);
            }
            let handler = handler_name(e.name);
            let arg = if e.data == "void" {
                String::new()
            } else {
                format!("data: {}", e.data)
            };
            let _ = writeln!(out, "  {handler}?: ({arg}) => void;");
        }
        if spec.children {
            out.push_str("  children?: JSX.Element;\n");
        }
        for s in spec.slots {
            if !s.doc.is_empty() {
                let _ = writeln!(out, "  /** {} */", s.doc);
            }
            let _ = writeln!(out, "  {}?: JSX.Element;", s.name);
        }
        out.push_str("}\n");
        if !spec.doc.is_empty() {
            let _ = writeln!(out, "\n/** {} */", spec.doc);
        }
        let list = |names: Vec<&str>| {
            names
                .iter()
                .map(|n| format!("{n:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(
            out,
            "export const {}: (props: {}Props) => JSX.Element = component({:?}, [{}], [{}], {}, [{}]);",
            spec.export,
            spec.export,
            spec.name,
            list(spec.props.iter().map(|p| p.name).collect()),
            list(spec.events.iter().map(|e| e.name).collect()),
            spec.children,
            list(spec.slots.iter().map(|s| s.name).collect()),
        );
    }
    out
}

/// `press` -> `onPress`.
pub(crate) fn handler_name(event: &str) -> String {
    let mut chars = event.chars();
    match chars.next() {
        Some(first) => format!("on{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => "on".to_owned(),
    }
}
