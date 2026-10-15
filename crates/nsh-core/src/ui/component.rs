//! Components: the [`Component`] trait, its [`Props`] and [`Event`]s
//! (derived), the [`Registry`] a surface renders from, and the JS glue and
//! TypeScript declarations generated from them ([`typescript`]).

use std::{any::Any, fmt, sync::Arc};

use rustc_hash::FxHashMap;

use super::{
    Children, IntoElement, PropError, Slot, Theme,
    element::{ComponentElement, ElNode, RenderCx},
};
use crate::JsonValue;

/// A component written in Rust. Its state is `Self` (`Default` makes a new
/// one); its props come from JS (or from the component that renders it);
/// its events go to JS.
///
/// ```ignore
/// #[derive(Default)]
/// pub struct Button;
///
/// impl Component for Button {
///     const NAME: &str = "nsh.button";
///     type Props = ButtonProps;
///     type Event = ButtonEvent;
///
///     fn render(&self, props: &ButtonProps, cx: &mut RenderCx<Self>) -> impl IntoElement {
///         view().px_4().py_3().bg(cx.theme().primary).child(props.title.clone())
///     }
/// }
/// ```
pub trait Component: Default + 'static {
    /// Its registered name (`nsh.button`); the JS export is the last part
    /// in Pascal case (`Button`).
    const NAME: &'static str;

    /// Its props (`#[derive(Props)]`).
    type Props: Props;

    /// Its events (`#[derive(Event)]`; [`NoEvent`] for none).
    type Event: Event;

    /// Its element tree. Runs again when a prop changes or a handler
    /// calls [`super::EventCx::notify`]; keep it cheap and free of side
    /// effects.
    fn render(&self, props: &Self::Props, cx: &mut RenderCx<'_, Self>) -> impl IntoElement;

    /// This component inside another one's render.
    fn element(props: Self::Props) -> ComponentElement<Self> {
        ComponentElement::new(props)
    }
}

/// What a prop is to the glue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    /// A value, sent as `$name`.
    Value,
    /// A named slot: JSX in a holder node.
    Slot,
    /// The JSX children.
    Children,
}

/// One prop, for the glue and its declarations.
#[derive(Debug, Clone)]
pub struct PropSpec {
    /// Its JS name (`accessibilityLabel`).
    pub name: &'static str,
    /// Its TypeScript type.
    pub ts: String,
    /// It may be left out.
    pub optional: bool,
    /// Documentation.
    pub doc: &'static str,
    /// What it is.
    pub kind: PropKind,
}

/// One event, for the glue and its declarations.
#[derive(Debug, Clone)]
pub struct EventSpec {
    /// Its name (`press`: the handler is `onPress`).
    pub name: &'static str,
    /// The TypeScript type of its data (`void` for none).
    pub data: String,
    /// Documentation.
    pub doc: &'static str,
}

/// A component's props. Derive it: `#[derive(Props)]`.
pub trait Props: Clone + PartialEq + 'static {
    /// The component's documentation (the struct's doc comment).
    fn doc() -> &'static str;

    /// The props.
    fn spec() -> Vec<PropSpec>;

    /// Every prop at its default.
    fn initial() -> Self;

    /// Prop `name` (its JS name) from JS; `None` restores its default.
    /// Returns whether it changed.
    fn set(&mut self, name: &str, value: Option<JsonValue>) -> Result<bool, PropError>;

    /// Slot `name` filled (`Some`) or not; returns whether it changed.
    fn set_slot(&mut self, name: &str, slot: Option<Slot>) -> bool;

    /// Whether the app passed children; returns whether it changed.
    fn set_children(&mut self, children: Children) -> bool;
}

/// A component's events. Derive it: `#[derive(Event)]`.
pub trait Event: 'static {
    /// The events.
    fn spec() -> Vec<EventSpec>;

    /// This event's name.
    fn name(&self) -> &'static str;

    /// Its data for JS.
    fn data(&self) -> JsonValue;
}

/// The events of a component that has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoEvent {}

impl Event for NoEvent {
    fn spec() -> Vec<EventSpec> {
        Vec::new()
    }

    fn name(&self) -> &'static str {
        match *self {}
    }

    fn data(&self) -> JsonValue {
        match *self {}
    }
}

/// What a component is to JS: its glue and declarations are generated
/// from it ([`typescript`]).
#[derive(Debug, Clone)]
pub struct ComponentSpec {
    /// Its registered name (`nsh.button`).
    pub name: &'static str,
    /// Its JS export (`Button`).
    pub export: String,
    /// Documentation.
    pub doc: &'static str,
    /// Its props, slots and children.
    pub props: Vec<PropSpec>,
    /// Its events.
    pub events: Vec<EventSpec>,
}

impl ComponentSpec {
    fn of<C: Component>() -> Self {
        Self {
            name: C::NAME,
            export: export_name(C::NAME),
            doc: C::Props::doc(),
            props: C::Props::spec(),
            events: C::Event::spec(),
        }
    }

    /// Its named slots.
    pub fn slots(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.props
            .iter()
            .filter(|p| p.kind == PropKind::Slot)
            .map(|p| p.name)
    }

    /// Whether it takes JSX children.
    pub fn takes_children(&self) -> bool {
        self.props.iter().any(|p| p.kind == PropKind::Children)
    }
}

/// `nsh.tab_bar` -> `TabBar`.
fn export_name(name: &str) -> String {
    let last = name.rsplit('.').next().unwrap_or(name);
    let mut out = String::with_capacity(last.len());
    let mut upper = true;
    for c in last.chars() {
        if c == '_' || c == '-' {
            upper = true;
        } else if upper {
            out.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// An instance: a component's state and props, their types erased.
pub(crate) trait AnyInstance {
    fn set_prop(&mut self, name: &str, value: Option<JsonValue>) -> Result<bool, PropError>;
    /// Props from the component that renders this one; true when they
    /// changed.
    fn set_props(&mut self, props: &dyn Any) -> bool;
    fn set_slot(&mut self, name: &'static str, filled: bool) -> bool;
    fn set_children(&mut self, present: bool) -> bool;
    fn render(&self, theme: &Theme) -> ElNode;
    fn state(&mut self) -> &mut dyn Any;
}

struct Instance<C: Component> {
    state: C,
    props: C::Props,
}

impl<C: Component> AnyInstance for Instance<C> {
    fn set_prop(&mut self, name: &str, value: Option<JsonValue>) -> Result<bool, PropError> {
        self.props.set(name, value)
    }

    fn set_props(&mut self, props: &dyn Any) -> bool {
        match props.downcast_ref::<C::Props>() {
            Some(p) if *p != self.props => {
                self.props = p.clone();
                true
            }
            _ => false,
        }
    }

    fn set_slot(&mut self, name: &'static str, filled: bool) -> bool {
        self.props.set_slot(name, filled.then(|| Slot::new(name)))
    }

    fn set_children(&mut self, present: bool) -> bool {
        self.props.set_children(Children::new(present))
    }

    fn render(&self, theme: &Theme) -> ElNode {
        let mut cx = RenderCx::new(theme);
        self.state.render(&self.props, &mut cx).into_element().0
    }

    fn state(&mut self) -> &mut dyn Any {
        &mut self.state
    }
}

#[derive(Clone)]
struct ComponentType {
    spec: Arc<ComponentSpec>,
    make: fn() -> Box<dyn AnyInstance>,
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

    /// Registers component `C` under [`Component::NAME`].
    pub fn register<C: Component>(&mut self) -> &mut Self {
        fn make<C: Component>() -> Box<dyn AnyInstance> {
            Box::new(Instance::<C> {
                state: C::default(),
                props: C::Props::initial(),
            })
        }
        Arc::make_mut(&mut self.types).insert(
            C::NAME,
            ComponentType {
                spec: Arc::new(ComponentSpec::of::<C>()),
                make: make::<C>,
            },
        );
        self
    }

    /// The registered components, by name.
    pub fn specs(&self) -> Vec<Arc<ComponentSpec>> {
        let mut specs: Vec<_> = self.types.values().map(|t| t.spec.clone()).collect();
        specs.sort_by_key(|s| s.name);
        specs
    }

    /// A new instance of component `name`.
    pub(crate) fn make(&self, name: &str) -> Option<(Arc<ComponentSpec>, Box<dyn AnyInstance>)> {
        self.types.get(name).map(|t| (t.spec.clone(), (t.make)()))
    }
}

/// The JS glue and TypeScript declarations of `specs`: one component per
/// spec, exported by its [`ComponentSpec::export`] name, built on
/// `nsh/_core`'s `component` helper.
pub fn typescript(specs: &[Arc<ComponentSpec>]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "// Generated from the Rust components (nsh_core::ui::typescript).\n\n\
         import type { JSX } from \"nsh\";\n\
         import { component, type ComponentHostProps } from \"nsh/_core\";\n",
    );
    let doc = |out: &mut String, indent: &str, doc: &str| {
        if !doc.is_empty() {
            let _ = writeln!(out, "{indent}/** {doc} */");
        }
    };
    let list = |names: Vec<&str>| {
        names
            .iter()
            .map(|n| format!("{n:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    for spec in specs {
        let _ = write!(
            out,
            "\nexport interface {}Props extends ComponentHostProps {{\n",
            spec.export
        );
        for p in &spec.props {
            doc(&mut out, "  ", p.doc);
            let optional = if p.optional { "?" } else { "" };
            let _ = writeln!(out, "  {}{optional}: {};", p.name, p.ts);
        }
        for e in &spec.events {
            doc(&mut out, "  ", e.doc);
            let handler = handler_name(e.name);
            let arg = if e.data == "void" {
                String::new()
            } else {
                format!("data: {}", e.data)
            };
            let _ = writeln!(out, "  {handler}?: ({arg}) => void;");
        }
        out.push_str("}\n\n");
        doc(&mut out, "", spec.doc);
        let _ = writeln!(
            out,
            "export const {}: (props: {}Props) => JSX.Element = component({:?}, [{}], [{}], {}, [{}]);",
            spec.export,
            spec.export,
            spec.name,
            list(
                spec.props
                    .iter()
                    .filter(|p| p.kind == PropKind::Value)
                    .map(|p| p.name)
                    .collect()
            ),
            list(spec.events.iter().map(|e| e.name).collect()),
            spec.takes_children(),
            list(spec.slots().collect()),
        );
    }
    out
}

/// `press` -> `onPress`.
fn handler_name(event: &str) -> String {
    let mut chars = event.chars();
    match chars.next() {
        Some(first) => format!("on{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => "on".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::prelude::*;

    #[derive(Default)]
    struct Stepper;

    /// Picks a number.
    #[derive(Clone, PartialEq, Props)]
    struct StepperProps {
        /// The value.
        value: f64,
        #[prop(default)]
        size: StepperSize,
        max_value: Option<f64>,
        /// Above the control.
        label: Option<Slot>,
        children: Children,
    }

    #[derive(Clone, Copy, PartialEq, Default, PropEnum)]
    enum StepperSize {
        #[default]
        Regular,
        ExtraLarge,
    }

    #[derive(Event)]
    enum StepperEvent {
        /// The value changed.
        Change {
            new_value: f64,
        },
        Limit(u32),
        Reset,
    }

    impl Component for Stepper {
        const NAME: &'static str = "test.number_stepper";
        type Props = StepperProps;
        type Event = StepperEvent;

        fn render(&self, _props: &StepperProps, _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
            view()
        }
    }

    #[test]
    fn the_glue_and_declarations_come_from_the_types() {
        let mut registry = Registry::new();
        registry.register::<Stepper>();
        let ts = typescript(&registry.specs());
        for line in [
            "export interface NumberStepperProps extends ComponentHostProps {",
            "  /** The value. */\n  value: number;",
            "  size?: \"regular\" | \"extraLarge\";",
            "  maxValue?: number | null;",
            "  /** Above the control. */\n  label?: JSX.Element;",
            "  children?: JSX.Element;",
            "  /** The value changed. */\n  onChange?: (data: { newValue: number }) => void;",
            "  onLimit?: (data: number) => void;",
            "  onReset?: () => void;",
            "/** Picks a number. */",
            "export const NumberStepper: (props: NumberStepperProps) => JSX.Element = \
             component(\"test.number_stepper\", [\"value\", \"size\", \"maxValue\"], \
             [\"change\", \"limit\", \"reset\"], true, [\"label\"]);",
        ] {
            assert!(ts.contains(line), "missing {line:?} in\n{ts}");
        }
        let mut props = StepperProps::initial();
        assert!(
            props
                .set("size", Some(serde_json::json!("extraLarge")))
                .unwrap()
        );
        assert!(props.size == StepperSize::ExtraLarge);
        assert!(props.set("size", Some(serde_json::json!("huge"))).is_err());
        assert!(props.set("size", None).unwrap());
        assert!(props.size == StepperSize::Regular);
        assert_eq!(
            StepperEvent::Change { new_value: 2.5 }.data(),
            serde_json::json!({ "newValue": 2.5 })
        );
        assert_eq!(StepperEvent::Limit(3).data(), serde_json::json!(3));
        assert_eq!(StepperEvent::Reset.name(), "reset");
    }
}
