//! Rust components in the core ([`crate::ui`]): creating instances,
//! expanding their renders into nodes, diffing renders, placing JS children
//! and slots, and routing events.
//!
//! A component's host node is the root of its expansion: the root element's
//! props and styles apply to it, its other elements become nodes with odd
//! ids (JS uses even ones) owned by the host. Ops from JS on a host are
//! intercepted ([`Core::apply`]): its `class` goes after the root's styles,
//! its children are placed where the render's `Children` is. Renders run at
//! the start of a commit, so props set in one JS turn render once.

use std::{any::Any, sync::Arc};

use rustc_hash::FxHashMap;

use super::{Core, NO_KEY};
use crate::{
    ApplyError, Command, EventKind, EventMask, EventPayload, JsonValue, NodeId, Op, Prop, Tag,
    TextEngine,
    tailwind::ClassList,
    ui::{
        AnyInstance, Attrs, Child, ComponentSpec, ElKind, ElNode, ErasedEventCx, IntoElement,
        ListConfig, Listener, ParentElement, PlaceFn, Position, Registry, SharedString,
        StyleRefinement, Theme, view,
    },
};

/// Renders of one component per commit, at most (a component whose render
/// keeps asking for another one is cut off).
const MAX_RENDERS: u32 = 16;

/// Lowered styles kept for reuse (components render the same styles over
/// and over); cleared when full.
const STYLE_CACHE: usize = 4096;

/// What renders on a host node.
enum Inner {
    /// Nothing is registered under the name: renders nothing.
    Missing,
    Component(Box<dyn AnyInstance>),
    /// A list cell of a component's list: the component's row function
    /// wraps the row the app rendered.
    Row {
        place: PlaceFn,
        index: usize,
    },
}

/// Where a host's JS children are.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Placement {
    /// Out of the tree (the render has no place for them).
    None,
    /// In this node, before the rendered node after the render's
    /// `Children` (their anchor).
    At(NodeId, Option<NodeId>),
    /// Each in its own wrapper ([`ParentElement::children_each`]): placed
    /// by the render.
    Each,
}

/// A component instance on its host node.
pub(super) struct Instance {
    inner: Inner,
    spec: Option<Arc<ComponentSpec>>,
    /// The app's `class` and `classList` on the host.
    class: Box<str>,
    class_list: Vec<Box<str>>,
    /// The app's classes, or the styles of the component rendering this
    /// one: applied after the root's own.
    user: Option<Arc<ClassList>>,
    /// Events JS listens to on the host.
    js_events: EventMask,
    /// The last render, expanded (its root is the host).
    root: Option<Expanded>,
    /// The host's JS children, in order.
    js_children: Vec<NodeId>,
    children_at: Placement,
    /// The glue's node of each slot the app passes.
    slots: Vec<(&'static str, NodeId)>,
    /// The last render's handlers: native events by node, and the events of
    /// nested components by their host (a handful: lists beat maps).
    handlers: Vec<(NodeId, EventKind, Listener<EventPayload>)>,
    event_handlers: Vec<(NodeId, Listener<dyn Any>)>,
    dirty: bool,
    renders: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Node,
    /// A slot's node: JS's, never released by the core.
    Slot,
    /// Where the JS children are (no node of its own).
    Children,
    /// A nested component.
    Component(&'static str),
    /// One of the JS children (no node of its own).
    JsChild,
}

/// The node's classes: its styles lowered, and what was merged after them.
type Classes = (Option<Arc<ClassList>>, Option<Arc<ClassList>>);

/// An element of a render, as it was applied.
struct Expanded {
    id: NodeId,
    tag: Tag,
    key: Option<SharedString>,
    /// A text's content and its raw text node.
    text: Option<(SharedString, NodeId)>,
    attrs: Attrs,
    style: StyleRefinement,
    classes: Classes,
    events: EventMask,
    /// A list's layout.
    list: ListConfig,
    part: Part,
    children: Vec<Expanded>,
}

impl Expanded {
    /// A node of JS's in the render (`id` 0: the place of the children).
    fn js(id: NodeId, part: Part) -> Self {
        Self {
            id,
            tag: Tag::View,
            key: None,
            text: None,
            attrs: Attrs::default(),
            style: StyleRefinement::default(),
            classes: (None, None),
            events: EventMask::default(),
            list: ListConfig::default(),
            part,
            children: Vec::new(),
        }
    }
}

/// The core's component state.
pub(super) struct Components {
    registry: Registry,
    theme: Arc<Theme>,
    /// Boxed: a render takes its instance out of the map and back.
    pub(super) instances: FxHashMap<NodeId, Box<Instance>>,
    /// Expanded node -> the host of the component that rendered it.
    owners: FxHashMap<NodeId, NodeId>,
    /// Slot nodes (`componentSlot`) -> their slot's name.
    slot_names: FxHashMap<NodeId, Arc<str>>,
    /// Slot nodes in a host -> the host.
    slot_hosts: FxHashMap<NodeId, NodeId>,
    styles: FxHashMap<StyleRefinement, Arc<ClassList>>,
    /// The lists components place (the glue's, of `Items` props) -> the
    /// row function of the component's last render, and the number of rows
    /// its wrappers last saw.
    lists: FxHashMap<NodeId, (PlaceFn, u32)>,
    next_id: NodeId,
    dirty: Vec<NodeId>,
    /// Reused buffers.
    rendered: Vec<NodeId>,
    props: Vec<Prop>,
}

impl Default for Components {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
            theme: Arc::new(Theme::default()),
            instances: FxHashMap::default(),
            owners: FxHashMap::default(),
            slot_names: FxHashMap::default(),
            slot_hosts: FxHashMap::default(),
            styles: FxHashMap::default(),
            lists: FxHashMap::default(),
            // Odd: JS allocates the even ids (1 is the root).
            next_id: 3,
            dirty: Vec::new(),
            rendered: Vec::new(),
            props: Vec::new(),
        }
    }
}

impl Components {
    fn alloc(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 2;
        id
    }

    fn mark(&mut self, host: NodeId) {
        if let Some(inst) = self.instances.get_mut(&host)
            && !inst.dirty
        {
            inst.dirty = true;
            self.dirty.push(host);
        }
    }

    /// `style` as rules, shared between the elements that have it.
    fn lowered(&mut self, style: &StyleRefinement) -> Arc<ClassList> {
        if let Some(list) = self.styles.get(style) {
            return list.clone();
        }
        if self.styles.len() >= STYLE_CACHE {
            self.styles.clear();
        }
        let list = Arc::new(style.lower());
        self.styles.insert(style.clone(), list.clone());
        list
    }
}

fn same(a: &Option<Arc<ClassList>>, b: &Option<Arc<ClassList>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

impl<M: TextEngine> Core<M> {
    /// The components this core can render (from `nsh-ui-mobile` and the
    /// like); set before the first op.
    pub fn set_components(&mut self, registry: Registry) {
        self.components.registry = registry;
    }

    /// `uiTheme` on the root: the app's theme (`nsh.json`); every component
    /// renders again.
    pub(super) fn set_ui_theme(&mut self, json: &str) {
        match Theme::from_json(json) {
            Ok(theme) => {
                self.components.theme = Arc::new(theme);
                let hosts: Vec<NodeId> = self.components.instances.keys().copied().collect();
                for host in hosts {
                    self.components.mark(host);
                }
            }
            Err(e) => log::warn!("theme: {e}"),
        }
    }

    /// Whether `id` is a component's host.
    fn is_host(&self, id: NodeId) -> bool {
        self.components.instances.contains_key(&id)
    }

    /// Ops from JS on a component's host take its place; others pass
    /// (`Some`).
    pub(super) fn intercept(&mut self, op: Op) -> Result<Option<Op>, ApplyError> {
        match op {
            // A slot's node joins its host: the render places it.
            Op::Insert { parent, child, .. }
                if self.is_host(parent) && self.components.slot_names.contains_key(&child) =>
            {
                let name = self.components.slot_names[&child].clone();
                let inst = self.components.instances.get_mut(&parent).expect("host");
                let Some(slot) = inst.spec.as_ref().and_then(|s| s.holder(&name)) else {
                    log::warn!(target: "js", "the component has no slot `{name}`");
                    self.detach_id(child);
                    return Ok(None);
                };
                inst.slots.retain(|(_, n)| *n != child);
                inst.slots.push((slot, child));
                self.components.slot_hosts.insert(child, parent);
                self.components.mark(parent);
                Ok(None)
            }
            Op::Remove { parent, child }
                if self.is_host(parent) && self.components.slot_names.contains_key(&child) =>
            {
                self.unslot(child);
                self.detach_id(child);
                Ok(None)
            }
            // A component's list gets a cell: the component wraps its rows.
            Op::Insert { parent, child, .. }
                if self.components.slot_hosts.contains_key(&parent) && self.is_list(parent) =>
            {
                self.apply_op(op)?;
                if let Some((place, _)) = self.components.lists.get(&parent).cloned() {
                    self.wrap_row(child, place);
                }
                Ok(None)
            }
            Op::Remove { parent, .. }
                if self.components.slot_hosts.contains_key(&parent) && self.is_list(parent) =>
            {
                Ok(Some(op))
            }
            // A slot filled or emptied renders its component again.
            Op::Insert { parent, .. } | Op::Remove { parent, .. }
                if self.components.slot_hosts.contains_key(&parent) =>
            {
                let empty = |core: &Self| {
                    core.key(parent)
                        .map_or(true, |k| core.nodes[k as usize].children.is_empty())
                };
                let before = empty(self);
                self.apply_op(op)?;
                if empty(self) != before {
                    let host = self.components.slot_hosts[&parent];
                    self.components.mark(host);
                }
                Ok(None)
            }
            Op::Release { id } if self.components.slot_names.contains_key(&id) => {
                self.unslot(id);
                self.components.slot_names.remove(&id);
                self.components.lists.remove(&id);
                Ok(Some(Op::Release { id }))
            }
            // The number of rows changed: the rows' wrappers render again
            // (they may depend on it: the last row).
            Op::SetProp {
                id,
                prop: Prop::Count(count),
            } if self.components.lists.contains_key(&id) => {
                let key = self.key(id)?;
                self.apply_op(Op::SetProp {
                    id,
                    prop: Prop::Count(count),
                })?;
                // A splice before may have changed the list's count already.
                let seen = &mut self.components.lists.get_mut(&id).expect("list").1;
                if std::mem::replace(seen, count) != count {
                    let cells: Vec<NodeId> = self.nodes[key as usize]
                        .children
                        .iter()
                        .map(|&c| self.nodes[c as usize].id)
                        .collect();
                    for cell in cells {
                        self.components.mark(cell);
                    }
                }
                Ok(None)
            }
            // A row cell shows another row: its wrapper renders again.
            Op::SetProp {
                id,
                prop: Prop::CellIndex(index),
            } if self.is_host(id) => {
                let inst = self.components.instances.get_mut(&id).expect("host");
                if let Inner::Row { index: current, .. } = &mut inst.inner
                    && *current != index as usize
                {
                    *current = index as usize;
                    self.components.mark(id);
                }
                Ok(Some(op))
            }
            Op::Insert {
                parent,
                child,
                before,
            } if self.is_host(parent) => {
                self.key(child)?;
                let inst = self.components.instances.get_mut(&parent).expect("host");
                let was_empty = inst.js_children.is_empty();
                inst.js_children.retain(|&c| c != child);
                let at = before.and_then(|b| inst.js_children.iter().position(|&c| c == b));
                inst.js_children
                    .insert(at.unwrap_or(inst.js_children.len()), child);
                let place = inst.children_at;
                if was_empty || place == Placement::Each {
                    self.components.mark(parent);
                }
                match place {
                    // Before its next JS sibling, else before what the
                    // render puts after the children.
                    Placement::At(p, anchor) => self.apply_op(Op::Insert {
                        parent: p,
                        child,
                        before: at.and(before).or(anchor),
                    })?,
                    // Its wrapper is the render's.
                    Placement::Each => self.detach_id(child),
                    Placement::None => self.detach_id(child),
                }
                Ok(None)
            }
            Op::Remove { parent, child } if self.is_host(parent) => {
                let inst = self.components.instances.get_mut(&parent).expect("host");
                inst.js_children.retain(|&c| c != child);
                if inst.js_children.is_empty() || inst.children_at == Placement::Each {
                    self.components.mark(parent);
                }
                self.detach_id(child);
                Ok(None)
            }
            Op::SetProp {
                id,
                prop: Prop::Class(class),
            } if self.is_host(id) => {
                let inst = self.components.instances.get_mut(&id).expect("host");
                inst.class = class;
                self.update_user_class(id);
                Ok(None)
            }
            Op::SetProp {
                id,
                prop: Prop::ClassList(list),
            } if self.is_host(id) => {
                let inst = self.components.instances.get_mut(&id).expect("host");
                inst.class_list = list;
                self.update_user_class(id);
                Ok(None)
            }
            // The component's events come from the core, not from the view.
            Op::Listen {
                id,
                event: EventKind::NativeEvent,
                ..
            } if self.is_host(id) => Ok(None),
            Op::Listen { id, event, enabled } if self.is_host(id) => {
                let inst = self.components.instances.get_mut(&id).expect("host");
                inst.js_events.set(event, enabled);
                let own = inst.root.as_ref().is_some_and(|r| r.events.contains(event));
                Ok(Some(Op::Listen {
                    id,
                    event,
                    enabled: enabled || own,
                }))
            }
            Op::Command {
                id,
                command: Command::Native { name, .. },
            } if self.is_host(id) => {
                log::warn!(target: "js", "Rust components take no commands (`{name}`)");
                Ok(None)
            }
            Op::Release { id } if self.is_host(id) => {
                self.release_instance(id);
                Ok(Some(Op::Release { id }))
            }
            op => Ok(Some(op)),
        }
    }

    /// The number of rows of the list cell `cell` is in.
    fn rows_of_cell(&self, cell: NodeId) -> usize {
        let Ok(key) = self.key(cell) else { return 0 };
        self.nodes
            .get(self.nodes[key as usize].parent as usize)
            .and_then(|list| list.list.as_ref())
            .map_or(0, |list| list.count as usize)
    }

    /// The number of rows of list `list`.
    fn rows_of_list(&self, list: NodeId) -> u32 {
        self.key(list)
            .ok()
            .and_then(|k| self.nodes[k as usize].list.as_ref())
            .map_or(0, |l| l.count)
    }

    /// Whether `id` is a list.
    fn is_list(&self, id: NodeId) -> bool {
        self.key(id)
            .is_ok_and(|k| self.nodes[k as usize].list.is_some())
    }

    /// Item cell `cell` of a component's list: its rows are wrapped by
    /// `place`, the row function of the component's last render.
    fn wrap_row(&mut self, cell: NodeId, place: PlaceFn) {
        let Ok(key) = self.key(cell) else { return };
        let node = &self.nodes[key as usize];
        if !matches!(node.slot, Some(crate::CellSlot::Item(_))) {
            return;
        }
        if let Some(inst) = self.components.instances.get_mut(&cell) {
            if let Inner::Row { place: current, .. } = &mut inst.inner {
                *current = place;
            }
            self.components.mark(cell);
            return;
        }
        // The row's content is already in the cell: it moves into the
        // wrapper.
        let js_children = node
            .children
            .iter()
            .map(|&c| self.nodes[c as usize].id)
            .collect();
        self.components.instances.insert(
            cell,
            Box::new(Instance {
                inner: Inner::Row { place, index: 0 },
                spec: None,
                class: "".into(),
                class_list: Vec::new(),
                user: None,
                js_events: EventMask::default(),
                root: None,
                js_children,
                children_at: Placement::None,
                slots: Vec::new(),
                handlers: Vec::new(),
                event_handlers: Vec::new(),
                dirty: false,
                renders: 0,
            }),
        );
        self.components.mark(cell);
    }

    /// The app's classes on `host` changed.
    fn update_user_class(&mut self, host: NodeId) {
        let inst = &self.components.instances[&host];
        let mut combined = String::with_capacity(inst.class.len() + 16 * inst.class_list.len());
        combined.push_str(&inst.class);
        for name in &inst.class_list {
            combined.push(' ');
            combined.push_str(name);
        }
        let user = (!combined.trim().is_empty()).then(|| self.tailwind.get(&combined).0);
        self.set_user(host, user);
    }

    /// What goes after `host`'s own styles.
    fn set_user(&mut self, host: NodeId, user: Option<Arc<ClassList>>) {
        let inst = self.components.instances.get_mut(&host).expect("host");
        if !same(&inst.user, &user) {
            inst.user = user;
            self.components.mark(host);
        }
    }

    /// Slot node `id` leaves its host.
    fn unslot(&mut self, id: NodeId) {
        if let Some(host) = self.components.slot_hosts.remove(&id) {
            if let Some(inst) = self.components.instances.get_mut(&host) {
                inst.slots.retain(|(_, n)| *n != id);
            }
            self.components.mark(host);
        }
    }

    /// `componentSlot` on a view: it holds a slot's JSX.
    pub(super) fn set_slot_name(&mut self, id: NodeId, name: Arc<str>) {
        self.components.slot_names.insert(id, name);
    }

    /// `Prop::Component` on a view: it becomes the host of component `name`.
    pub(super) fn create_instance(&mut self, host: NodeId, name: Arc<str>) {
        if self.is_host(host) {
            return;
        }
        let (spec, inner) = match self.components.registry.make(&name) {
            Some((spec, inner)) => (Some(spec), Inner::Component(inner)),
            None => {
                self.deny(format!(
                    "no component is registered as `{name}`: it renders nothing"
                ));
                (None, Inner::Missing)
            }
        };
        self.components.instances.insert(
            host,
            Box::new(Instance {
                inner,
                spec,
                class: "".into(),
                class_list: Vec::new(),
                user: None,
                js_events: EventMask::default(),
                root: None,
                js_children: Vec::new(),
                children_at: Placement::None,
                slots: Vec::new(),
                handlers: Vec::new(),
                event_handlers: Vec::new(),
                dirty: false,
                renders: 0,
            }),
        );
        self.components.mark(host);
    }

    /// `$name` on a host: a prop of its component.
    pub(super) fn set_component_prop(
        &mut self,
        host: NodeId,
        name: Arc<str>,
        value: Option<JsonValue>,
    ) -> bool {
        let Some(inst) = self.components.instances.get_mut(&host) else {
            return false;
        };
        let Inner::Component(inner) = &mut inst.inner else {
            return true;
        };
        match inner.set_prop(&name, value) {
            Ok(true) => self.components.mark(host),
            Ok(false) => {}
            Err(e) => {
                let component = inst.spec.as_ref().map_or("", |s| s.name);
                log::warn!(target: "js", "{component}: {e}");
            }
        }
        true
    }

    /// Renders the components whose props or state changed.
    pub(super) fn render_components(&mut self) {
        if self.components.dirty.is_empty() {
            return;
        }
        let mut rendered = std::mem::take(&mut self.components.rendered);
        while let Some(host) = self.components.dirty.pop() {
            let Some(mut inst) = self.components.instances.remove(&host) else {
                continue;
            };
            inst.dirty = false;
            inst.renders += 1;
            if inst.renders > MAX_RENDERS {
                log::error!(target: "js", "component on node {host} keeps rendering again; stopped");
                self.components.instances.insert(host, inst);
                continue;
            }
            self.render(host, &mut inst);
            self.components.instances.insert(host, inst);
            rendered.push(host);
        }
        for host in rendered.drain(..) {
            if let Some(inst) = self.components.instances.get_mut(&host) {
                inst.renders = 0;
            }
        }
        self.components.rendered = rendered;
    }

    fn render(&mut self, host: NodeId, inst: &mut Instance) {
        let mut el = match &mut inst.inner {
            Inner::Component(inner) => {
                // What the app passed, as props.
                if let Some(spec) = &inst.spec {
                    for name in spec.slots() {
                        let filled = inst.slots.iter().any(|(s, node)| {
                            *s == name
                                && self
                                    .key(*node)
                                    .is_ok_and(|k| !self.nodes[k as usize].children.is_empty())
                        });
                        inner.set_slot(name, filled);
                    }
                }
                inner.set_children(!inst.js_children.is_empty());
                inner.render(&self.components.theme)
            }
            // The cell itself stays the list's: the wrapper goes inside.
            Inner::Row { place, index } => {
                let at = Position {
                    index: *index,
                    count: self.rows_of_cell(host),
                };
                view().child(place(Child::row(), at)).into_element().0
            }
            Inner::Missing => ElNode::empty_view(),
        };
        if el.tag != Tag::View || !matches!(el.kind, ElKind::Plain) {
            log::error!(target: "js", "a component's root must be a view; it is rendered as one");
            el.tag = Tag::View;
            el.kind = ElKind::Plain;
            el.text = None;
        }
        inst.handlers.clear();
        inst.event_handlers.clear();
        let old = inst.root.take();
        let old_place = std::mem::replace(&mut inst.children_at, Placement::None);
        // The root is the host itself.
        let (old_attrs, old_style, old_classes, old_events, old_children) = match old {
            Some(e) => (e.attrs, e.style, e.classes, e.events, e.children),
            None => (
                Attrs::default(),
                StyleRefinement::default(),
                (None, None),
                EventMask::default(),
                Vec::new(),
            ),
        };
        self.apply_attrs(host, &el.attrs, &old_attrs);
        let user = inst.user.clone();
        let classes = self.apply_styles(host, &el.style, &old_style, old_classes, user);
        // The host listens to its component's events and to JS's.
        let js = inst.js_events;
        self.apply_handlers(host, &el, old_events | js, js, inst);
        let mut events = EventMask::default();
        for (kind, _) in &el.handlers {
            events.set(*kind, true);
        }
        let new_children = std::mem::take(&mut el.children);
        let children = self.reconcile_children(host, old_children, new_children, host, inst);
        inst.root = Some(Expanded {
            id: host,
            tag: Tag::View,
            key: None,
            text: None,
            attrs: el.attrs,
            style: el.style,
            classes,
            events,
            list: ListConfig::default(),
            part: Part::Node,
            children,
        });
        // No `Children` any more: the JS children leave the tree.
        if inst.children_at == Placement::None && old_place != Placement::None {
            for &child in &inst.js_children {
                self.detach_id(child);
            }
        }
    }

    fn detach_id(&mut self, id: NodeId) {
        if let Ok(key) = self.key(id)
            && self.nodes[key as usize].parent != NO_KEY
        {
            self.detach(key);
        }
    }

    fn apply_attrs(&mut self, id: NodeId, new: &Attrs, old: &Attrs) {
        let mut props = std::mem::take(&mut self.components.props);
        new.changes(old, &mut props);
        for prop in props.drain(..) {
            let _ = self.apply_op(Op::SetProp { id, prop });
        }
        self.components.props = props;
    }

    /// Sets node `id`'s classes: `style` lowered, then `user` (a host's).
    fn apply_styles(
        &mut self,
        id: NodeId,
        style: &StyleRefinement,
        old_style: &StyleRefinement,
        old: Classes,
        user: Option<Arc<ClassList>>,
    ) -> Classes {
        let own = if style == old_style {
            old.0.clone()
        } else if style.is_empty() {
            None
        } else {
            Some(self.components.lowered(style))
        };
        if !same(&own, &old.0) || !same(&user, &old.1) {
            let list = match (&own, &user) {
                (Some(a), Some(b)) => Some(Arc::new(a.merged(b))),
                (Some(a), None) => Some(a.clone()),
                (None, Some(b)) => Some(b.clone()),
                (None, None) => None,
            };
            if let Ok(key) = self.key(id) {
                self.set_class_list(key, list);
            }
        }
        (own, user)
    }

    /// Registers `el`'s handlers for node `id` and listens to their events
    /// (plus `extra`, what JS listens to on a host); returns the mask.
    fn apply_handlers(
        &mut self,
        id: NodeId,
        el: &ElNode,
        old: EventMask,
        extra: EventMask,
        inst: &mut Instance,
    ) -> EventMask {
        let mut mask = extra;
        for (kind, handler) in &el.handlers {
            mask.set(*kind, true);
            inst.handlers.push((id, *kind, handler.clone()));
        }
        // Only the events that changed.
        let mut changed = mask.bits() ^ old.bits();
        while changed != 0 {
            let kind = EventKind::ALL[changed.trailing_zeros() as usize];
            changed &= changed - 1;
            let _ = self.apply_op(Op::Listen {
                id,
                event: kind,
                enabled: mask.contains(kind),
            });
        }
        mask
    }

    fn reconcile_children(
        &mut self,
        parent: NodeId,
        old: Vec<Expanded>,
        new: Vec<ElNode>,
        host: NodeId,
        inst: &mut Instance,
    ) -> Vec<Expanded> {
        if new.is_empty() && old.is_empty() {
            return Vec::new();
        }
        // `children_each`: one wrapper per JS child, keyed by it.
        let new = if new.iter().any(|e| matches!(e.kind, ElKind::EachChild(_))) {
            let mut each = Vec::with_capacity(new.len() + inst.js_children.len());
            for el in new {
                let ElKind::EachChild(place) = &el.kind else {
                    each.push(el);
                    continue;
                };
                if inst.children_at != Placement::None {
                    log::error!(target: "js", "a render places its children once");
                    continue;
                }
                inst.children_at = Placement::Each;
                let count = inst.js_children.len();
                for (index, &child) in inst.js_children.iter().enumerate() {
                    let mut wrapper = place(Child::js(child), Position { index, count }).0;
                    wrapper.key = Some(format!("\u{0}{child}").into());
                    each.push(wrapper);
                }
            }
            each
        } else {
            new
        };
        let mut old: Vec<Option<Expanded>> = old.into_iter().map(Some).collect();
        let mut out = Vec::with_capacity(new.len());
        let mut placeholder = None;
        // Unkeyed elements take the next unused old element of their kind,
        // in order: an element that comes or goes (`.children(option)`)
        // does not shift its siblings onto new nodes.
        let mut cursor = 0;
        for mut el in new {
            match std::mem::replace(&mut el.kind, ElKind::Plain) {
                ElKind::Slot(name) => {
                    // The slot's own node, while the app passes one.
                    let Some(&(_, node)) = inst.slots.iter().find(|(s, _)| *s == name) else {
                        continue;
                    };
                    let at = old.iter().position(|o| {
                        o.as_ref()
                            .is_some_and(|o| o.part == Part::Slot && o.id == node)
                    });
                    let prev = at.and_then(|j| old[j].take());
                    out.push(self.reconcile_slot(prev, node, el, host, inst));
                }
                ElKind::Children => {
                    if placeholder.is_some() || inst.children_at != Placement::None {
                        log::error!(target: "js", "a render places its children once");
                        continue;
                    }
                    if let Some(j) = old
                        .iter()
                        .position(|o| o.as_ref().is_some_and(|o| o.part == Part::Children))
                    {
                        old[j] = None;
                    }
                    placeholder = Some(out.len());
                    out.push(Expanded::js(0, Part::Children));
                }
                ElKind::JsChild(id) => {
                    // Taken over, not released (that would detach it).
                    if let Some(j) = old.iter().position(|o| {
                        o.as_ref()
                            .is_some_and(|o| o.part == Part::JsChild && o.id == id)
                    }) {
                        old[j] = None;
                    }
                    if inst.js_children.contains(&id) {
                        out.push(Expanded::js(id, Part::JsChild));
                    }
                }
                // Expanded above.
                ElKind::EachChild(_) => {}
                ElKind::List(list) => {
                    // The glue's list: placed and laid out like a slot,
                    // its rows wrapped by this render's row function.
                    let Some(&(_, node)) = inst.slots.iter().find(|(s, _)| *s == list.name) else {
                        continue;
                    };
                    let at = old.iter().position(|o| {
                        o.as_ref()
                            .is_some_and(|o| o.part == Part::Slot && o.id == node)
                    });
                    let prev = at.and_then(|j| old[j].take());
                    let old_config = prev.as_ref().map(|p| p.list).unwrap_or_default();
                    let mut props = std::mem::take(&mut self.components.props);
                    list.config.changes(&old_config, &mut props);
                    for prop in props.drain(..) {
                        let _ = self.apply_op(Op::SetProp { id: node, prop });
                    }
                    self.components.props = props;
                    let mut e = self.reconcile_slot(prev, node, el, host, inst);
                    e.list = list.config;
                    let rows = self.rows_of_list(node);
                    self.components.lists.insert(node, (list.row.clone(), rows));
                    let cells: Vec<NodeId> = match self.key(node) {
                        Ok(k) => self.nodes[k as usize]
                            .children
                            .iter()
                            .map(|&c| self.nodes[c as usize].id)
                            .collect(),
                        Err(_) => Vec::new(),
                    };
                    for cell in cells {
                        self.wrap_row(cell, list.row.clone());
                    }
                    out.push(e);
                }
                kind => {
                    el.kind = kind;
                    let part = match &el.kind {
                        ElKind::Component(c) => Part::Component(c.name),
                        _ => Part::Node,
                    };
                    let matches = |o: &Expanded| o.tag == el.tag && o.part == part;
                    let at = match &el.key {
                        Some(k) => old.iter().position(|o| {
                            o.as_ref()
                                .is_some_and(|o| o.key.as_ref() == Some(k) && matches(o))
                        }),
                        None => {
                            let at = old[cursor.min(old.len())..]
                                .iter()
                                .position(|o| {
                                    o.as_ref().is_some_and(|o| o.key.is_none() && matches(o))
                                })
                                .map(|j| j + cursor);
                            if let Some(j) = at {
                                cursor = j + 1;
                            }
                            at
                        }
                    };
                    let prev = at.and_then(|j| old[j].take());
                    out.push(self.reconcile_node(prev, el, part, host, inst));
                }
            }
        }
        for o in old.into_iter().flatten() {
            self.release_expanded(o);
        }
        if let Some(at) = placeholder {
            inst.children_at = Placement::At(parent, out.get(at + 1).map(|e| e.id));
        }
        // Order the native children as rendered, the JS children in the
        // placeholder's place (moves only when it changed).
        let js = &inst.js_children;
        let wanted = || {
            out.iter().flat_map(move |e| {
                if e.part == Part::Children {
                    js.as_slice()
                } else {
                    std::slice::from_ref(&e.id)
                }
            })
        };
        let in_order = self.key(parent).is_ok_and(|k| {
            let have = &self.nodes[k as usize].children;
            have.len() == wanted().count()
                && have
                    .iter()
                    .zip(wanted())
                    .all(|(&c, &id)| self.nodes[c as usize].id == id)
        });
        if !in_order {
            let ids: Vec<NodeId> = wanted().copied().collect();
            for child in ids {
                let _ = self.apply_op(Op::Insert {
                    parent,
                    child,
                    before: None,
                });
            }
        }
        out
    }

    /// Slot node `node` as element `el`: its styles, props and handlers
    /// (its children are the app's JSX).
    fn reconcile_slot(
        &mut self,
        prev: Option<Expanded>,
        node: NodeId,
        el: ElNode,
        host: NodeId,
        inst: &mut Instance,
    ) -> Expanded {
        let (old_attrs, old_style, old_classes, old_events) = match prev {
            Some(p) => (p.attrs, p.style, p.classes, p.events),
            None => (
                Attrs::default(),
                StyleRefinement::default(),
                (None, None),
                EventMask::default(),
            ),
        };
        self.apply_attrs(node, &el.attrs, &old_attrs);
        let classes = self.apply_styles(node, &el.style, &old_style, old_classes, None);
        let events = self.apply_handlers(node, &el, old_events, EventMask::default(), inst);
        // Its events are the component's.
        self.components.owners.insert(node, host);
        Expanded {
            id: node,
            tag: Tag::View,
            key: None,
            text: None,
            attrs: el.attrs,
            style: el.style,
            classes,
            events,
            list: ListConfig::default(),
            part: Part::Slot,
            children: Vec::new(),
        }
    }

    fn reconcile_node(
        &mut self,
        prev: Option<Expanded>,
        mut el: ElNode,
        part: Part,
        host: NodeId,
        inst: &mut Instance,
    ) -> Expanded {
        let (id, old_attrs, old_style, old_classes, old_events, old_children, old_text) = match prev
        {
            Some(p) => (
                p.id, p.attrs, p.style, p.classes, p.events, p.children, p.text,
            ),
            None => {
                let id = self.components.alloc();
                let _ = self.apply_op(Op::Create { id, tag: el.tag });
                self.components.owners.insert(id, host);
                if let Part::Component(name) = part {
                    let _ = self.apply_op(Op::SetProp {
                        id,
                        prop: Prop::Component(name.into()),
                    });
                }
                (
                    id,
                    Attrs::default(),
                    StyleRefinement::default(),
                    (None, None),
                    EventMask::default(),
                    Vec::new(),
                    None,
                )
            }
        };
        if let ElKind::Component(c) = std::mem::replace(&mut el.kind, ElKind::Plain) {
            // A nested component: its props, and the styles as the app's
            // classes of its host.
            if let Some(nested) = self.components.instances.get_mut(&id)
                && let Inner::Component(inner) = &mut nested.inner
                && inner.set_props(c.props.as_any())
            {
                self.components.mark(id);
            }
            if self.is_host(id) {
                let user = (!el.style.is_empty()).then(|| self.components.lowered(&el.style));
                self.set_user(id, user);
            }
            if let Some(listener) = c.on_event {
                inst.event_handlers.push((id, listener));
            }
            return Expanded {
                id,
                tag: el.tag,
                key: el.key,
                text: None,
                attrs: Attrs::default(),
                style: el.style,
                classes: (None, None),
                events: EventMask::default(),
                list: ListConfig::default(),
                part,
                children: Vec::new(),
            };
        }
        self.apply_attrs(id, &el.attrs, &old_attrs);
        let classes = self.apply_styles(id, &el.style, &old_style, old_classes, None);
        let text = match (el.text.take(), old_text) {
            (Some(content), Some((old, raw))) => {
                if content != old {
                    let _ = self.apply_op(Op::SetText {
                        id: raw,
                        text: content.as_str().to_owned(),
                    });
                }
                Some((content, raw))
            }
            (Some(content), None) => {
                let raw = self.components.alloc();
                let _ = self.apply_op(Op::CreateText {
                    id: raw,
                    text: content.as_str().to_owned(),
                });
                self.components.owners.insert(raw, host);
                let _ = self.apply_op(Op::Insert {
                    parent: id,
                    child: raw,
                    before: None,
                });
                Some((content, raw))
            }
            (None, Some((_, raw))) => {
                self.release_id(raw);
                None
            }
            (None, None) => None,
        };
        let events = self.apply_handlers(id, &el, old_events, EventMask::default(), inst);
        let new_children = std::mem::take(&mut el.children);
        let children = self.reconcile_children(id, old_children, new_children, host, inst);
        Expanded {
            id,
            tag: el.tag,
            key: el.key,
            text,
            attrs: el.attrs,
            style: el.style,
            classes,
            events,
            list: ListConfig::default(),
            part,
            children,
        }
    }

    fn release_id(&mut self, id: NodeId) {
        self.components.owners.remove(&id);
        let _ = self.apply_op(Op::Release { id });
    }

    /// Frees an expanded element and what it rendered (a slot's node is
    /// JS's: it only leaves the tree).
    fn release_expanded(&mut self, e: Expanded) {
        match e.part {
            Part::Slot => {
                self.components.owners.remove(&e.id);
                self.detach_id(e.id);
                return;
            }
            // The JS children are placed (or detached) by the render.
            Part::Children => return,
            // A JS child leaves the wrapper being released.
            Part::JsChild => {
                self.detach_id(e.id);
                return;
            }
            Part::Component(_) => self.release_instance(e.id),
            Part::Node => {}
        }
        for c in e.children {
            self.release_expanded(c);
        }
        if let Some((_, raw)) = e.text {
            self.release_id(raw);
        }
        self.release_id(e.id);
    }

    /// Frees what a component rendered (not its host nor its JS children).
    fn release_instance(&mut self, host: NodeId) {
        let Some(inst) = self.components.instances.remove(&host) else {
            return;
        };
        for &child in &inst.js_children {
            self.detach_id(child);
        }
        for (_, node) in inst.slots {
            self.components.slot_hosts.remove(&node);
            self.components.owners.remove(&node);
            self.detach_id(node);
        }
        if let Some(root) = inst.root {
            for c in root.children {
                self.release_expanded(c);
            }
        }
    }

    /// A native event on node `id`: true when a component handled it (its
    /// own parts' events never reach JS, which does not know them).
    pub fn component_event(&mut self, id: NodeId, kind: EventKind, payload: &EventPayload) -> bool {
        let host = if self.is_host(id) {
            id
        } else if let Some(&h) = self.components.owners.get(&id) {
            h
        } else {
            return false;
        };
        self.handle(host, id, kind, payload) || id != host
    }

    /// Runs `host`'s component's handler of `kind` on its node `id`; false
    /// when it has none.
    fn handle(
        &mut self,
        host: NodeId,
        id: NodeId,
        kind: EventKind,
        payload: &EventPayload,
    ) -> bool {
        let Some(inst) = self.components.instances.get_mut(&host) else {
            return false;
        };
        let Some(listener) = inst
            .handlers
            .iter()
            .find(|(n, k, _)| *n == id && *k == kind)
            .map(|(_, _, l)| l.clone())
        else {
            return false;
        };
        let mut cx = ErasedEventCx::default();
        if let Inner::Component(inner) = &mut inst.inner {
            (listener.0)(inner.state(), payload, &mut cx);
        }
        self.finish_event(host, cx);
        true
    }

    /// Applies what a handler did: a new render, events for JS (or for the
    /// component that rendered this one).
    fn finish_event(&mut self, host: NodeId, cx: ErasedEventCx) {
        if cx.notify {
            self.components.mark(host);
        }
        for event in cx.emits {
            match self.components.owners.get(&host).copied() {
                // A nested component: the component that rendered it gets
                // the event, if it handles it.
                Some(parent) => {
                    let Some(inst) = self.components.instances.get_mut(&parent) else {
                        continue;
                    };
                    let Some(listener) = inst
                        .event_handlers
                        .iter()
                        .find(|(n, _)| *n == host)
                        .map(|(_, l)| l.clone())
                    else {
                        continue;
                    };
                    let mut cx = ErasedEventCx::default();
                    if let Inner::Component(inner) = &mut inst.inner {
                        (listener.0)(inner.state(), event.as_any(), &mut cx);
                    }
                    self.finish_event(parent, cx);
                }
                None => self.events.push((
                    host,
                    EventKind::NativeEvent,
                    EventPayload::Native {
                        name: event.name().into(),
                        data: event.data(),
                    },
                )),
            }
        }
    }
}
