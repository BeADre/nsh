//! Rust components in the core ([`crate::component`]): creating instances,
//! expanding their renders into nodes, diffing renders, placing JS children
//! and routing events and commands.
//!
//! A component's host node is the root of its expansion: the root element's
//! props apply to it, its other elements become nodes with odd ids (JS uses
//! even ones) owned by the host. Ops from JS on a host are intercepted
//! ([`Core::apply`]): its `class` is appended to the root's, its children are
//! placed where the render holds them, `ref.command` reaches the component.
//! Renders run at the start of a commit, so props set in one JS turn render
//! once.

use std::sync::Arc;

use rustc_hash::FxHashMap;

use super::{Core, NO_KEY};
use crate::{
    ApplyError, Command, EventKind, EventMask, EventPayload, JsonValue, NodeId, Op, Prop, Tag,
    TextEngine,
    component::{
        AnyComponent, Attrs, ClassStr, ElNode, EventCx, Handler, Props, Registry, RenderCx,
    },
};

/// Renders of one component per commit, at most (a component whose render
/// keeps asking for another one is cut off).
const MAX_RENDERS: u32 = 16;

/// A component instance on its host node.
pub(super) struct Instance {
    /// `None` when nothing is registered under the name (renders nothing).
    component: Option<Box<dyn AnyComponent>>,
    props: Props,
    /// The app's `class`, appended to the root's.
    user_class: Option<Arc<str>>,
    /// Events JS listens to on the host.
    js_events: EventMask,
    /// The last render, expanded (its root is the host).
    root: Option<Expanded>,
    /// The host's JS children, in order.
    js_children: Vec<NodeId>,
    /// Where they are placed (the element holding them).
    children_parent: Option<NodeId>,
    /// The glue's node of each named slot (it holds the slot's JSX).
    slots: FxHashMap<Arc<str>, NodeId>,
    /// The last render's handlers, by node and event (a handful: a list
    /// beats a map).
    handlers: Vec<(NodeId, EventKind, Handler)>,
    dirty: bool,
    renders: u32,
}

/// An element of a render, as it was applied.
struct Expanded {
    id: NodeId,
    tag: Tag,
    key: Option<Arc<str>>,
    /// A text's content and its raw text node.
    text: Option<(Arc<str>, NodeId)>,
    attrs: Attrs,
    events: EventMask,
    /// A nested component's name.
    component: Option<Arc<str>>,
    /// A slot's node: JS's, never released by the core.
    slot: bool,
    children: Vec<Expanded>,
}

/// The core's component state.
pub(super) struct Components {
    registry: Registry,
    /// Boxed: a render takes its instance out of the map and back.
    pub(super) instances: FxHashMap<NodeId, Box<Instance>>,
    /// Expanded node -> the host of the component that rendered it.
    owners: FxHashMap<NodeId, NodeId>,
    /// Slot nodes (`componentSlot`) -> their slot's name.
    slot_names: FxHashMap<NodeId, Arc<str>>,
    /// Slot nodes in a host -> the host.
    slot_hosts: FxHashMap<NodeId, NodeId>,
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
            instances: FxHashMap::default(),
            owners: FxHashMap::default(),
            slot_names: FxHashMap::default(),
            slot_hosts: FxHashMap::default(),
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
}

fn joined(classes: &[Box<str>]) -> Option<Arc<str>> {
    let joined = classes.join(" ");
    (!joined.trim().is_empty()).then(|| joined.into())
}

impl<M: TextEngine> Core<M> {
    /// The components this core can render (from `nsh-ui-mobile` and the
    /// like); set before the first op.
    pub fn set_components(&mut self, registry: Registry) {
        self.components.registry = registry;
    }

    /// Whether `id` is a component's host.
    fn is_host(&self, id: NodeId) -> bool {
        self.components.instances.contains_key(&id)
    }

    /// Ops from JS on a component's host take its place; others pass
    /// (`Some`).
    pub(super) fn intercept(&mut self, op: Op) -> Result<Option<Op>, ApplyError> {
        // Apps without components pay one check per op.
        if self.components.instances.is_empty() && self.components.slot_names.is_empty() {
            return Ok(Some(op));
        }
        match op {
            // A slot's node joins its host: the render places it.
            Op::Insert { parent, child, .. }
                if self.is_host(parent) && self.components.slot_names.contains_key(&child) =>
            {
                let name = self.components.slot_names[&child].clone();
                let inst = self.components.instances.get_mut(&parent).expect("host");
                inst.slots.insert(name, child);
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
                Ok(Some(Op::Release { id }))
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
                // A sibling that is not a child (a slot's node) is no anchor.
                let before = at.and(before);
                inst.js_children
                    .insert(at.unwrap_or(inst.js_children.len()), child);
                let place = inst.children_parent;
                if was_empty {
                    self.components.mark(parent);
                }
                match place {
                    Some(p) => self.apply_op(Op::Insert {
                        parent: p,
                        child,
                        before,
                    })?,
                    None => self.detach_id(child),
                }
                Ok(None)
            }
            Op::Remove { parent, child } if self.is_host(parent) => {
                let inst = self.components.instances.get_mut(&parent).expect("host");
                inst.js_children.retain(|&c| c != child);
                if inst.js_children.is_empty() {
                    self.components.mark(parent);
                }
                self.detach_id(child);
                Ok(None)
            }
            Op::SetProp {
                id,
                prop: Prop::Class(classes),
            } if self.is_host(id) => {
                self.set_user_class(id, joined(&[classes]));
                Ok(None)
            }
            Op::SetProp {
                id,
                prop: Prop::ClassList(classes),
            } if self.is_host(id) => {
                self.set_user_class(id, joined(&classes));
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
                command: Command::Native { name, args },
            } if self.is_host(id) => {
                let mut cx = EventCx::default();
                if let Some(c) = self
                    .components
                    .instances
                    .get_mut(&id)
                    .and_then(|i| i.component.as_mut())
                {
                    c.command(&name, &args, &mut cx);
                }
                self.finish_event(id, cx);
                Ok(None)
            }
            Op::Release { id } if self.is_host(id) => {
                self.release_instance(id);
                Ok(Some(Op::Release { id }))
            }
            op => Ok(Some(op)),
        }
    }

    /// Slot node `id` leaves its host.
    fn unslot(&mut self, id: NodeId) {
        if let Some(host) = self.components.slot_hosts.remove(&id) {
            if let Some(inst) = self.components.instances.get_mut(&host) {
                inst.slots.retain(|_, &mut n| n != id);
            }
            self.components.mark(host);
        }
    }

    /// `componentSlot` on a view: it holds a slot's JSX.
    pub(super) fn set_slot_name(&mut self, id: NodeId, name: Arc<str>) {
        self.components.slot_names.insert(id, name);
    }

    fn set_user_class(&mut self, host: NodeId, class: Option<Arc<str>>) {
        let inst = self.components.instances.get_mut(&host).expect("host");
        if inst.user_class != class {
            inst.user_class = class;
            self.components.mark(host);
        }
    }

    /// `Prop::Component` on a view: it becomes the host of component `name`.
    pub(super) fn create_instance(&mut self, host: NodeId, name: Arc<str>) {
        if self.is_host(host) {
            return;
        }
        let (props, component) = match self.components.registry.make(&name) {
            Some((props, c)) => (props, Some(c)),
            None => {
                self.deny(format!(
                    "no component is registered as `{name}`: it renders nothing"
                ));
                (Props::new(None, Default::default()), None)
            }
        };
        self.components.instances.insert(
            host,
            Box::new(Instance {
                component,
                props,
                user_class: None,
                js_events: EventMask::default(),
                root: None,
                js_children: Vec::new(),
                children_parent: None,
                slots: FxHashMap::default(),
                handlers: Vec::new(),
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
        if inst.props.set(&name, value) {
            self.components.mark(host);
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
        let filled = inst
            .slots
            .iter()
            .filter(|&(_, &node)| {
                self.key(node)
                    .is_ok_and(|k| !self.nodes[k as usize].children.is_empty())
            })
            .map(|(name, _)| name.clone())
            .collect();
        let mut cx = RenderCx {
            has_children: !inst.js_children.is_empty(),
            filled,
        };
        let mut el = match &inst.component {
            Some(c) => c.render(&inst.props, &mut cx),
            None => crate::component::view::<()>().node,
        };
        if el.tag != Tag::View || el.component.is_some() {
            log::error!(target: "js", "a component's root must be a view; it is rendered as one");
            el.tag = Tag::View;
            el.component = None;
            el.text = None;
        }
        if let Some(user) = &inst.user_class {
            el.attrs.class.push(ClassStr::Shared(user.clone()));
        }
        inst.handlers.clear();
        let old = inst.root.take();
        let old_parent = inst.children_parent.take();
        // The root is the host itself.
        let (old_attrs, old_events, old_children) = match old {
            Some(e) => (e.attrs, e.events, e.children),
            None => (Attrs::default(), EventMask::default(), Vec::new()),
        };
        self.apply_attrs(host, &el.attrs, &old_attrs);
        // The host listens to its component's events and to JS's.
        let js = inst.js_events;
        self.apply_handlers(host, &el, old_events | js, js, inst);
        let mut events = EventMask::default();
        for (kind, _) in &el.handlers {
            events.set(*kind, true);
        }
        let children = if el.holds_children {
            for e in old_children {
                self.release_expanded(e);
            }
            inst.children_parent = Some(host);
            Vec::new()
        } else {
            self.reconcile_children(
                host,
                old_children,
                std::mem::take(&mut el.children),
                host,
                inst,
            )
        };
        inst.root = Some(Expanded {
            id: host,
            tag: Tag::View,
            key: None,
            text: None,
            attrs: el.attrs,
            events,
            component: None,
            slot: false,
            children,
        });
        if inst.children_parent != old_parent {
            self.place_children(inst);
        }
    }

    /// Moves the JS children where the render holds them (or out of the
    /// tree when it does not).
    fn place_children(&mut self, inst: &Instance) {
        for &child in &inst.js_children {
            match inst.children_parent {
                Some(parent) => {
                    let _ = self.apply_op(Op::Insert {
                        parent,
                        child,
                        before: None,
                    });
                }
                None => self.detach_id(child),
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
        let mut old: Vec<Option<Expanded>> = old.into_iter().map(Some).collect();
        let mut out = Vec::with_capacity(new.len());
        for (i, el) in new.into_iter().enumerate() {
            if let Some(name) = &el.slot {
                // The slot's own node, while the app passes one.
                let Some(&node) = inst.slots.get(name) else {
                    continue;
                };
                let at = old
                    .iter()
                    .position(|o| o.as_ref().is_some_and(|o| o.slot && o.id == node));
                let prev = at.and_then(|j| old[j].take());
                out.push(self.reconcile_slot(prev, node, el, host, inst));
                continue;
            }
            let same = |o: &Expanded| {
                o.tag == el.tag && o.component.as_deref() == el.component.as_ref().map(|c| &*c.0)
            };
            let at = match &el.key {
                Some(k) => old.iter().position(|o| {
                    o.as_ref()
                        .is_some_and(|o| !o.slot && o.key.as_ref() == Some(k) && same(o))
                }),
                None => old
                    .get(i)
                    .and_then(|o| o.as_ref())
                    .is_some_and(|o| !o.slot && o.key.is_none() && same(o))
                    .then_some(i),
            };
            let prev = at.and_then(|j| old[j].take());
            out.push(self.reconcile_node(prev, el, host, inst));
        }
        for o in old.into_iter().flatten() {
            self.release_expanded(o);
        }
        // Order the native children as rendered (moves only when it changed).
        let in_order = self.key(parent).is_ok_and(|k| {
            let have = &self.nodes[k as usize].children;
            have.len() == out.len()
                && have
                    .iter()
                    .zip(&out)
                    .all(|(&c, e)| self.nodes[c as usize].id == e.id)
        });
        if !in_order {
            for e in &out {
                let _ = self.apply_op(Op::Insert {
                    parent,
                    child: e.id,
                    before: None,
                });
            }
        }
        out
    }

    /// Slot node `node` as element `el`: its classes, props and handlers
    /// (its children are the app's JSX).
    fn reconcile_slot(
        &mut self,
        prev: Option<Expanded>,
        node: NodeId,
        el: ElNode,
        host: NodeId,
        inst: &mut Instance,
    ) -> Expanded {
        let (old_attrs, old_events) = match prev {
            Some(p) => (p.attrs, p.events),
            None => (Attrs::default(), EventMask::default()),
        };
        self.apply_attrs(node, &el.attrs, &old_attrs);
        let events = self.apply_handlers(node, &el, old_events, EventMask::default(), inst);
        // Its events are the component's.
        self.components.owners.insert(node, host);
        Expanded {
            id: node,
            tag: Tag::View,
            key: None,
            text: None,
            attrs: el.attrs,
            events,
            component: None,
            slot: true,
            children: Vec::new(),
        }
    }

    fn reconcile_node(
        &mut self,
        prev: Option<Expanded>,
        mut el: ElNode,
        host: NodeId,
        inst: &mut Instance,
    ) -> Expanded {
        let (id, old_attrs, old_events, old_children, old_text) = match prev {
            Some(p) => (p.id, p.attrs, p.events, p.children, p.text),
            None => {
                let id = self.components.alloc();
                let _ = self.apply_op(Op::Create { id, tag: el.tag });
                self.components.owners.insert(id, host);
                if let Some((name, _)) = &el.component {
                    let _ = self.apply_op(Op::SetProp {
                        id,
                        prop: Prop::Component(name.clone()),
                    });
                }
                (id, Attrs::default(), EventMask::default(), Vec::new(), None)
            }
        };
        let component = el.component.take();
        let nested = component.as_ref().map(|c| c.0.clone());
        if let Some((_, props)) = component {
            // A nested component: its props, and `class` as the app's class.
            for (name, value) in props {
                self.set_component_prop(id, name, Some(value));
            }
            let class = (!el.attrs.class.is_empty()).then(|| el.attrs.class.joined().into());
            self.set_user_class(id, class);
        } else {
            self.apply_attrs(id, &el.attrs, &old_attrs);
        }
        let text = match (el.text.take(), old_text) {
            (Some(content), Some((old, raw))) => {
                if content != old {
                    let _ = self.apply_op(Op::SetText {
                        id: raw,
                        text: content.to_string(),
                    });
                }
                Some((content, raw))
            }
            (Some(content), None) => {
                let raw = self.components.alloc();
                let _ = self.apply_op(Op::CreateText {
                    id: raw,
                    text: content.to_string(),
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
        let children = if el.holds_children {
            for e in old_children {
                self.release_expanded(e);
            }
            inst.children_parent = Some(id);
            Vec::new()
        } else if el.children.is_empty() && old_children.is_empty() {
            Vec::new()
        } else {
            self.reconcile_children(
                id,
                old_children,
                std::mem::take(&mut el.children),
                host,
                inst,
            )
        };
        Expanded {
            id,
            tag: el.tag,
            key: el.key,
            text,
            attrs: el.attrs,
            events,
            component: nested,
            slot: false,
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
        if e.slot {
            self.components.owners.remove(&e.id);
            self.detach_id(e.id);
            return;
        }
        for c in e.children {
            self.release_expanded(c);
        }
        if let Some((_, raw)) = e.text {
            self.release_id(raw);
        }
        if e.component.is_some() {
            self.release_instance(e.id);
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
        let Some(handler) = inst
            .handlers
            .iter()
            .find(|(n, k, _)| *n == id && *k == kind)
            .map(|(_, _, h)| h.clone())
        else {
            return false;
        };
        let mut cx = EventCx::default();
        if let Some(c) = inst.component.as_mut() {
            handler(c.as_any(), payload, &mut cx);
        }
        self.finish_event(host, cx);
        true
    }

    /// Applies what a handler or command did: a new render, events for JS
    /// (or for the component that rendered this one).
    fn finish_event(&mut self, host: NodeId, cx: EventCx) {
        if cx.notify {
            self.components.mark(host);
        }
        for (name, data) in cx.emits {
            let payload = EventPayload::Native { name, data };
            match self.components.owners.get(&host).copied() {
                // A nested component: the component that rendered it gets
                // the event, on the nested host.
                Some(parent) => {
                    self.handle(parent, host, EventKind::NativeEvent, &payload);
                }
                None => self.events.push((host, EventKind::NativeEvent, payload)),
            }
        }
    }
}
