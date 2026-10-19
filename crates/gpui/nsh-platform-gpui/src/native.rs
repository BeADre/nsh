//! Native controls (videos, web views) and the native components the
//! embedding app registers (`nsh/native`'s `nativeComponent`).
//!
//! Each control is an item its node's view hosts ([`NshView::set_native`])
//! filling it: the view keeps decoration, frames, touches and
//! accessibility, as for any view, and [`resized`] keeps the item at the
//! view's bounds.
//!
//! - **Registered components** render *GPUI elements*:
//!   [`crate::AppView::register_component`] names a factory; the core
//!   creates a [`ViewKind::Native`] container, names the component
//!   ([`MountItem::Component`]) and sends its props as JSON as they change.
//!   Every frame, the surface element's prepaint asks each visible
//!   component for its element ([`NativeComponent::render`]: any GPUI or
//!   GPUI Kit element, an `Entity` view of the app's own included), lays it
//!   out at its container's box in the window and prepaints it there,
//!   clipped to the container's clipping ancestors ([`prepaint`]); the
//!   element is painted where the tree draws its container, so nsh content
//!   drawn above it covers it. A hitbox that blocks the mouse goes under
//!   the element: the component's own hitboxes take the pointer and the
//!   wheel over its box, nsh's input does not. Keys go to the component
//!   while focus is inside it ([`focused`]); a component with a focus
//!   handle ([`NativeComponent::focus_handle`]) is a Tab stop among the text
//!   fields, at its container's place ([`crate::input`], [`apply_focus`]).
//!   GPUI's content masks are rectangles: a rounded clipping ancestor cuts
//!   a component's element (and a web view) to its rectangle only. Components emit events and
//!   report their own size through their [`ComponentContext`], from any
//!   thread. Registries belong to one `AppView` (and the views made from
//!   the same [`crate::AppOptions`]): no globals.
//! - **Web views** ([`web`]) are wry web views in GPUI Fast native
//!   composition surfaces, placed at their view's box by every prepaint.
//! - **Videos** ([`video`]).
//!
//! Every control belongs to its surface ([`NativeState`]): stopping the app
//! drops them, so media stops playing and web views close.
//!
//! [`MountItem::Component`]: nsh_core::MountItem::Component

mod subtree;
mod typed;
pub use subtree::NativeSubtree;
pub use typed::{NativeRegistry, TypedComponentContext, TypedNativeComponent, typed_component};
mod video;
mod web;

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, ContentMask, FocusHandle, HitboxBehavior, IntoElement,
    ParentElement, Pixels, Styled, Window, prelude::FluentBuilder, px,
};
use nsh_core::{
    Command, EventKind, EventPayload, NativeProps, NodeId, PayloadValue, Rect, Size, ViewKind,
};
use nsh_engine::{Access, EngineControl};
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value as JsonValue;

use crate::{
    image::AssetBase,
    render::DrawCx,
    surface::Surface,
    view::{Hosted, NshView},
};

/// A native component made by a registered factory: GPUI content that the
/// app embedding nsh lays into an nsh layout.
///
/// ```no_run
/// use nsh_platform_gpui::{
///     ComponentContext, NativeComponent,
///     gpui::{
///         AnyElement, App, InteractiveElement, IntoElement, ParentElement,
///         StatefulInteractiveElement, Styled, Window, div, rgb,
///     },
/// };
///
/// /// A label the app sets with a `text` prop; clicks emit `tap`.
/// struct Badge {
///     cx: ComponentContext,
///     text: String,
/// }
///
/// impl NativeComponent for Badge {
///     fn render(&mut self, _window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
///         let context = self.cx.clone();
///         Some(
///             div()
///                 .id("badge")
///                 .size_full()
///                 .bg(rgb(0x8250df))
///                 .text_color(rgb(0xffffff))
///                 .child(self.text.clone())
///                 .on_click(move |_, _, _| context.emit("tap", serde_json::json!({})))
///                 .into_any_element(),
///         )
///     }
///
///     fn set_prop(
///         &mut self,
///         name: &str,
///         value: Option<&serde_json::Value>,
///         _window: &mut Window,
///         _cx: &mut App,
///     ) {
///         if name == "text" {
///             self.text = value.and_then(|v| v.as_str()).unwrap_or("").to_owned();
///         }
///     }
/// }
/// ```
pub trait NativeComponent {
    /// The GPUI element showing the component, laid out at its container's
    /// box (it is given exactly that size) and painted where nsh draws the
    /// container. Called on every frame of the surface while the container
    /// is visible; `None` draws nothing. Ask for a frame with
    /// [`ComponentContext::redraw`] when what it shows changes (an
    /// `Entity` the element shows redraws by itself when notified).
    fn render(&mut self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let _ = (window, cx);
        None
    }

    /// Where the last frame drew the container, in window coordinates, or
    /// `None` when it was not drawn: for components that move a native
    /// surface of their own with it.
    fn placed(&mut self, bounds: Option<Bounds<Pixels>>) {
        let _ = bounds;
    }

    /// The widget's primary focus handle, including programmatic-only focus.
    /// By default it is also a Tab stop; override `focus_handles` separately
    /// when pointer or command focus must remain outside the Tab order.
    fn focus_handle(&self) -> Option<FocusHandle> {
        None
    }

    /// The component's Tab stops in visual order. Compound components may
    /// expose more than one control; a roving group returns only its active
    /// entry. Disabled controls must be omitted. Defaults to `focus_handle`.
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focus_handle().into_iter().collect()
    }

    /// Sequential focus priority: positive values precede zero in ascending
    /// order; negative values exclude the component from Tab traversal.
    fn tab_index(&self) -> isize {
        0
    }

    /// Prop `name` changed (`None`: removed). Every prop the app set comes
    /// right after the component is made. Delivered with the next frame of
    /// the surface, before [`NativeComponent::render`], with its window:
    /// the component may update its entities here.
    fn set_prop(
        &mut self,
        name: &str,
        value: Option<&JsonValue>,
        window: &mut Window,
        cx: &mut App,
    );

    /// JS called `ref.command(name, args)`. Delivered as props are, in
    /// order with them.
    fn command(&mut self, name: &str, args: &JsonValue, window: &mut Window, cx: &mut App) {
        let _ = (name, args, window, cx);
    }
}

/// What a component can tell its app. Cheap to clone; usable from any
/// thread.
#[derive(Clone)]
pub struct ComponentContext {
    id: NodeId,
    control: EngineControl,
    /// The surface's key, to draw again from any thread.
    surface: u64,
    generation: u64,
}

impl std::fmt::Debug for ComponentContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentContext")
            .field("id", &self.id)
            .finish()
    }
}

impl ComponentContext {
    /// Applies the node's resolved NSH text inheritance to an unstyled native
    /// element. This opt-in leaves existing styled native components unchanged
    /// and adds no wrapper element or independent text-style parser.
    pub fn inherit_text_style<T: Styled>(&self, mut element: T) -> T {
        if let Some(surface) = Surface::by_key(self.surface)
            && surface.generation() == self.generation
            && let Some(style) = surface.native.text_styles.borrow().get(&self.id)
        {
            gpui::Refineable::refine(element.text_style(), style);
        }
        element
    }

    /// Uses NSH's already-resolved padding and border widths as the content
    /// insets of an unstyled native element. The NSH painter owns its visual
    /// border; do not add another native border around this content box.
    pub fn inherit_content_insets<T: Styled>(&self, element: T) -> T {
        let insets = Surface::by_key(self.surface)
            .filter(|surface| surface.generation() == self.generation)
            .and_then(|surface| {
                surface
                    .native
                    .content_insets
                    .borrow()
                    .get(&self.id)
                    .copied()
            });
        match insets {
            Some([top, right, bottom, left]) => element
                .pt(px(top))
                .pr(px(right))
                .pb(px(bottom))
                .pl(px(left)),
            None => element,
        }
    }

    /// Live NSH children as GPUI elements. Their reactive owners, node IDs,
    /// event handlers and native component entities stay in the NSH tree.
    /// Call on the GPUI thread while constructing the current frame.
    pub fn children(&self) -> Vec<NativeSubtree> {
        let Some(surface) = Surface::by_key(self.surface) else {
            return Vec::new();
        };
        if surface.generation() != self.generation {
            return Vec::new();
        }
        let Some(view) = surface.view(self.id) else {
            return Vec::new();
        };
        surface.native.composition_used.set(true);
        view.child_views()
            .into_iter()
            .filter(|child| {
                surface
                    .native
                    .slots
                    .borrow()
                    .get(&child.id())
                    .is_none_or(String::is_empty)
            })
            .map(|child| NativeSubtree::new(self.surface, child.id()))
            .collect()
    }

    /// A named live NSH slot. A slot is mounted by the JS bridge as a
    /// native node with the reserved `__nshSlot` prop.
    pub fn slot(&self, name: &str) -> Option<NativeSubtree> {
        let surface = Surface::by_key(self.surface)?;
        if surface.generation() != self.generation {
            return None;
        }
        let view = surface.view(self.id)?;
        surface.native.composition_used.set(true);
        if std::env::var_os("NSH_BRIDGE_TRACE").is_some() {
            eprintln!(
                "NSH bridge slot owner={} name={name:?} children={:?} named={:?}",
                self.id,
                view.child_views()
                    .iter()
                    .map(NshView::id)
                    .collect::<Vec<_>>(),
                surface.native.slots.borrow()
            );
        }
        view.child_views()
            .into_iter()
            .find(|child| {
                surface
                    .native
                    .slots
                    .borrow()
                    .get(&child.id())
                    .is_some_and(|slot| slot == name)
                    && !child.child_views().is_empty()
            })
            .map(|child| NativeSubtree::new(self.surface, child.id()))
    }

    /// Emits event `name` with `data`: the handler `on<Name>` gets it.
    pub fn emit(&self, name: &str, data: JsonValue) {
        self.control.dispatch_event(
            self.id,
            EventKind::NativeEvent,
            EventPayload::Native {
                name: name.into(),
                data,
            },
        );
    }

    /// The component's own size (points): used where its style leaves a
    /// dimension open.
    pub fn set_size(&self, width: f32, height: f32) {
        self.control
            .set_component_size(self.id, Size::new(width, height));
    }

    /// What the component shows changed: the surface draws again, asking
    /// for its element (from any thread).
    pub fn redraw(&self) {
        let key = self.surface;
        crate::rt::invoke(move || {
            if let Some(surface) = Surface::by_key(key) {
                surface.invalidate();
            }
        });
    }
}

// Components may emit from any thread (their own callbacks, workers).
const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<ComponentContext>();
};

/// Makes a component for an element, given its context.
pub type ComponentFactory = Rc<dyn Fn(ComponentContext) -> Box<dyn NativeComponent>>;

/// The components registered with a view (shared by its clones, so
/// registrations apply to elements created from then on).
#[derive(Clone, Default)]
pub struct Components {
    factories: Rc<RefCell<FxHashMap<Arc<str>, ComponentFactory>>>,
    /// Names already reported as unknown.
    missing: Rc<RefCell<FxHashSet<Arc<str>>>>,
}

impl std::fmt::Debug for Components {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set()
            .entries(self.factories.borrow().keys())
            .finish()
    }
}

impl Components {
    /// Registers `factory` under `name` (replacing an earlier one).
    pub fn register(&self, name: &str, factory: ComponentFactory) {
        self.factories.borrow_mut().insert(name.into(), factory);
        self.missing.borrow_mut().remove(name);
    }

    /// The factory registered as `name`; `None` (logged once per name) if
    /// there is none.
    fn factory(&self, name: &Arc<str>) -> Option<ComponentFactory> {
        let factory = self.factories.borrow().get(name).cloned();
        if factory.is_none() && self.missing.borrow_mut().insert(name.clone()) {
            log::warn!(
                target: "js",
                "no native component is registered as `{name}`: it renders an empty view"
            );
        }
        factory
    }
}

type LayoutHint = (Option<f32>, Option<f32>);
type ComposedPlacement = (Rc<dyn Hosted>, Bounds<Pixels>);

/// The native controls of one surface.
#[derive(Default)]
pub(crate) struct NativeState {
    /// The components the app can use.
    components: RefCell<Components>,
    /// Where `asset:/` video sources are.
    asset_base: RefCell<Option<AssetBase>>,
    /// What the running app may reach (web view navigations).
    access: RefCell<Option<Arc<Access>>>,
    /// The controls, by node.
    hosts: RefCell<FxHashMap<NodeId, Host>>,
    pub(crate) slots: RefCell<FxHashMap<NodeId, String>>,
    pub(crate) composition_used: std::cell::Cell<bool>,
    pub(crate) pointer_down_dispatched: std::cell::Cell<bool>,
    pub(crate) pointer_move_dispatched: std::cell::Cell<bool>,
    slot_widths: RefCell<FxHashMap<NodeId, f32>>,
    layout_hints: RefCell<FxHashMap<NodeId, LayoutHint>>,
    text_styles: RefCell<FxHashMap<NodeId, gpui::TextStyleRefinement>>,
    content_insets: RefCell<FxHashMap<NodeId, [f32; 4]>>,
    pub(crate) composed_placed: RefCell<Vec<ComposedPlacement>>,
    pub(crate) subtree_events:
        RefCell<Option<(FocusHandle, gpui::WeakEntity<crate::app::AppView>)>>,
    /// The focus handles inside components that the last prepaint found
    /// focused, with their nodes: keys go to them, not to nsh.
    focused: RefCell<Vec<(NodeId, FocusHandle)>>,
    /// The component Tab moved to, focused by [`apply_focus`].
    tab_to: std::cell::Cell<Option<(NodeId, usize)>>,
}

impl NativeState {
    /// Uses `components` (an [`crate::AppOptions`]'s registry).
    pub(crate) fn set_components(&self, components: Components) {
        *self.components.borrow_mut() = components;
    }

    /// The registry, to register in.
    pub(crate) fn components(&self) -> Components {
        self.components.borrow().clone()
    }

    /// Where `asset:/` sources resolve from now on.
    pub(crate) fn set_asset_base(&self, base: Option<AssetBase>) {
        *self.asset_base.borrow_mut() = base;
    }

    /// What the app that starts may reach: web views keep their top-level
    /// navigations within its `webview` capability.
    pub(crate) fn set_access(&self, access: Arc<Access>) {
        *self.access.borrow_mut() = Some(access);
    }

    /// Drops every control (the app stopped): media stops playing.
    pub(crate) fn clear(&self) {
        let hosts = std::mem::take(&mut *self.hosts.borrow_mut());
        drop(hosts);
        self.focused.borrow_mut().clear();
        self.slots.borrow_mut().clear();
        self.slot_widths.borrow_mut().clear();
        self.layout_hints.borrow_mut().clear();
        self.text_styles.borrow_mut().clear();
        self.content_insets.borrow_mut().clear();
        self.composed_placed.borrow_mut().clear();
        self.composition_used.set(false);
        self.subtree_events.borrow_mut().take();
        self.tab_to.set(None);
    }
}

/// What a native view holds.
enum Host {
    Video(Rc<video::VideoItem>),
    Web(Rc<web::WebItem>),
    Component(Rc<ComponentItem>),
}

impl Host {
    fn item(&self) -> Rc<dyn Hosted> {
        match self {
            Host::Video(v) => v.clone(),
            Host::Web(w) => w.clone(),
            Host::Component(c) => c.clone(),
        }
    }

    /// The host's item, cloned out of the map so that no borrow of it is
    /// held while the control runs.
    fn item_ref(&self) -> HostRef {
        match self {
            Host::Video(v) => HostRef::Video(v.clone()),
            Host::Web(w) => HostRef::Web(w.clone()),
            Host::Component(c) => HostRef::Component(c.clone()),
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // The item may outlive its control in its view (a removal
        // animation keeps the view on screen a little longer): it shows the
        // last frame of a video, and nothing of a web view.
        match self {
            Host::Video(v) => v.close(),
            Host::Web(w) => w.close(),
            Host::Component(c) => c.close(),
        }
    }
}

/// A host's item ([`Host::item_ref`]).
enum HostRef {
    Video(Rc<video::VideoItem>),
    Web(Rc<web::WebItem>),
    Component(Rc<ComponentItem>),
}

/// Sets up a web view, video or component container.
pub(crate) fn create(surface: &Rc<Surface>, view: &NshView) {
    let id = view.id();
    let host = match view.kind() {
        ViewKind::Video => Host::Video(video::VideoItem::new(surface, id)),
        ViewKind::WebView => Host::Web(web::WebItem::new(surface, id)),
        _ => Host::Component(Rc::new(ComponentItem {
            surface: Rc::downgrade(surface),
            id,
            component: RefCell::new(None),
            element: RefCell::new(None),
            pending: RefCell::default(),
            reported_size: std::cell::Cell::new(None),
        })),
    };
    fill(view, &host.item());
    surface.native.hosts.borrow_mut().insert(id, host);
}

/// A view was deleted: its control stops (a removal animation may keep
/// the view on screen a little longer).
pub(crate) fn deleted(surface: &Rc<Surface>, view: &NshView) {
    surface.native.slots.borrow_mut().remove(&view.id());
    surface.native.slot_widths.borrow_mut().remove(&view.id());
    surface.native.layout_hints.borrow_mut().remove(&view.id());
    surface.native.text_styles.borrow_mut().remove(&view.id());
    surface
        .native
        .content_insets
        .borrow_mut()
        .remove(&view.id());
    let host = surface.native.hosts.borrow_mut().remove(&view.id());
    drop(host);
}

/// The view's frame changed: its item fills it again.
pub(crate) fn resized(view: &NshView) {
    let mut s = view.state_mut();
    let rect = bounds(s.frame.width, s.frame.height);
    for native in &mut s.natives {
        native.rect = rect;
    }
}

/// `NativeProps`.
pub(crate) fn props(surface: &Rc<Surface>, view: &NshView, props: &NativeProps) {
    let host = surface
        .native
        .hosts
        .borrow()
        .get(&view.id())
        .map(Host::item_ref);
    match (host, props) {
        (Some(HostRef::Video(video)), NativeProps::Video(p)) => {
            let base = surface.native.asset_base.borrow().clone();
            video.set(surface, p, base.as_ref());
        }
        (Some(HostRef::Web(web)), NativeProps::WebView(p)) => web.set(surface, view, p),
        _ => log::debug!("native: props for a view of another kind"),
    }
}

/// `Component`.
pub(crate) fn component(surface: &Rc<Surface>, view: &NshView, name: &Arc<str>) {
    let factory = if name.as_ref() == "__nsh.Slot" {
        Some(
            Rc::new(|context| Box::new(subtree::Slot { context }) as Box<dyn NativeComponent>)
                as ComponentFactory,
        )
    } else {
        surface.native.components.borrow().factory(name)
    };
    let Some(factory) = factory else { return };
    let mut control = None;
    surface.control(|c| control = Some(c.clone()));
    let Some(control) = control else { return };
    let Some(item) = component_item(surface, view.id()) else {
        return;
    };
    // Made with no borrow held: the app's code may reach the surface.
    let made = factory(ComponentContext {
        id: view.id(),
        control,
        surface: surface.key(),
        generation: surface.generation(),
    });
    *item.component.borrow_mut() = Some(made);
    view.changed();
}

/// `ComponentProps`.
pub(crate) fn component_props(
    surface: &Rc<Surface>,
    view: &NshView,
    props: &[(Arc<str>, Option<JsonValue>)],
) {
    for (name, value) in props {
        if name.as_ref() == "__nshText" {
            if let Some(value) = value {
                let style = gpui::TextStyleRefinement {
                    color: value
                        .get("color")
                        .and_then(JsonValue::as_u64)
                        .map(|color| crate::render::hsla(nsh_core::Color(color as u32))),
                    font_size: value
                        .get("fontSize")
                        .and_then(JsonValue::as_f64)
                        .map(|size| px(size as f32).into()),
                    font_family: Some(
                        value
                            .get("fontFamily")
                            .and_then(JsonValue::as_str)
                            .map(|family| gpui::SharedString::from(family.to_owned()))
                            .unwrap_or_else(|| {
                                gpui::SharedString::from(crate::text::ui_family().to_string())
                            }),
                    ),
                    font_weight: value
                        .get("fontWeight")
                        .and_then(JsonValue::as_f64)
                        .map(|weight| gpui::FontWeight(weight as f32)),
                    font_style: Some(
                        if value
                            .get("italic")
                            .and_then(JsonValue::as_bool)
                            .unwrap_or(false)
                        {
                            gpui::FontStyle::Italic
                        } else {
                            gpui::FontStyle::Normal
                        },
                    ),
                    line_height: value
                        .get("lineHeight")
                        .and_then(JsonValue::as_f64)
                        .filter(|height| *height > 0.0)
                        .map(|height| px(height as f32).into()),
                    ..Default::default()
                };
                surface
                    .native
                    .text_styles
                    .borrow_mut()
                    .insert(view.id(), style);
            } else {
                surface.native.text_styles.borrow_mut().remove(&view.id());
            }
        }
        if name.as_ref() == "__nshLayout" {
            if let Some(insets) = value
                .as_ref()
                .and_then(|value| value.get("insets"))
                .and_then(JsonValue::as_array)
                && let [top, right, bottom, left] = insets.as_slice()
                && let [Some(top), Some(right), Some(bottom), Some(left)] =
                    [top.as_f64(), right.as_f64(), bottom.as_f64(), left.as_f64()]
            {
                surface.native.content_insets.borrow_mut().insert(
                    view.id(),
                    [top as f32, right as f32, bottom as f32, left as f32],
                );
            }
            let dimension = |key| {
                value
                    .as_ref()
                    .and_then(|value| value.get(key))
                    .and_then(JsonValue::as_f64)
                    .filter(|value| value.is_finite())
                    .map(|value| value as f32)
            };
            surface
                .native
                .layout_hints
                .borrow_mut()
                .insert(view.id(), (dimension("width"), dimension("height")));
        }
        if name.as_ref() == "__nshSlot" {
            match value.as_ref().and_then(JsonValue::as_str) {
                Some(name) => {
                    surface
                        .native
                        .slots
                        .borrow_mut()
                        .insert(view.id(), name.to_owned());
                }
                None => {
                    surface.native.slots.borrow_mut().remove(&view.id());
                    surface.native.slot_widths.borrow_mut().remove(&view.id());
                    surface.native.layout_hints.borrow_mut().remove(&view.id());
                }
            }
        }
    }
    let Some(item) = component_item(surface, view.id()) else {
        return;
    };
    if item.component.borrow().is_none() {
        return;
    }
    let mut pending = item.pending.borrow_mut();
    for (name, value) in props {
        if !matches!(name.as_ref(), "__nshLayout" | "__nshText") {
            pending.push(Pending::Prop(name.clone(), value.clone()));
        }
    }
    drop(pending);
    view.changed();
}

/// A command for a web view, video or component.
pub(crate) fn command(surface: &Rc<Surface>, view: &NshView, command: &Command) {
    let host = surface
        .native
        .hosts
        .borrow()
        .get(&view.id())
        .map(Host::item_ref);
    match (host, command) {
        (Some(HostRef::Video(video)), Command::Seek(seconds)) => video.seek(*seconds),
        (Some(HostRef::Web(web)), _) => web.command(command),
        (Some(HostRef::Component(item)), Command::Native { name, args })
            if item.component.borrow().is_some() =>
        {
            item.pending
                .borrow_mut()
                .push(Pending::Command(name.clone(), args.clone()));
            view.changed();
        }
        _ => {}
    }
}

/// The surface element's prepaint, after it inserted its own hitbox (and
/// set its origin and size): lays out and prepaints the components'
/// elements at their containers' boxes and places the web views, each over
/// a hitbox that keeps nsh's input away from it.
pub(crate) fn prepaint(surface: &Rc<Surface>, window: &mut Window, cx: &mut App) {
    let hosts: Vec<(NodeId, HostRef)> = {
        let hosts = surface.native.hosts.borrow();
        let mut hosts: Vec<_> = hosts
            .iter()
            .filter(|(_, h)| !matches!(h, Host::Video(_)))
            .map(|(id, h)| (*id, h.item_ref()))
            .collect();
        // Hitboxes stack in insertion order: keep it the same from frame to
        // frame (nodes made later are usually drawn later, above).
        hosts.sort_unstable_by_key(|(id, _)| *id);
        hosts
    };
    // Video frames (and images) dropped where nothing draws images.
    crate::image::release_dropped(window, cx);
    surface.native.focused.borrow_mut().clear();
    for (id, host) in hosts {
        let Some(view) = surface.view(id) else {
            continue;
        };
        // A nested component is laid out and painted by its parent's
        // NativeSubtree element, never independently at its old NSH box.
        if has_component_ancestor(surface, &view) {
            continue;
        }
        let geometry = geometry(surface, &view);
        match host {
            HostRef::Component(item) => {
                item.deliver(window, cx);
                if let Some(focus) = item.prepaint(geometry, window, cx) {
                    surface.native.focused.borrow_mut().push((id, focus));
                }
            }
            HostRef::Web(web) => web.prepaint(surface, &view, geometry, window, cx),
            HostRef::Video(_) => {}
        }
    }
    // A parent native element may encompass another native component.
    // Attribute ordinary control focus to its actual owner, including when
    // an earlier element had already assigned the frame's focus path.
    let mut actual = surface
        .native
        .hosts
        .borrow()
        .iter()
        .filter_map(|(id, host)| {
            let Host::Component(item) = host else {
                return None;
            };
            let component = item.component.try_borrow().ok()?;
            focus_tracking_handles(component.as_ref()?.as_ref())
                .into_iter()
                .find(|handle| handle.contains_focused(window, cx))
                .map(|handle| (*id, handle))
        })
        .collect::<Vec<_>>();
    actual.sort_by_key(|(id, _)| {
        let mut depth = 0usize;
        let mut parent = surface.view(*id).and_then(|view| view.parent_view());
        while let Some(view) = parent {
            depth += 1;
            parent = view.parent_view();
        }
        std::cmp::Reverse(depth)
    });
    if !actual.is_empty() {
        *surface.native.focused.borrow_mut() = actual;
    }
}

/// Whether a component owns this view's GPUI placement.
fn has_component_ancestor(surface: &Surface, view: &NshView) -> bool {
    let mut parent = view.parent_view();
    while let Some(view) = parent {
        if component_item(surface, view.id()).is_some() {
            return true;
        }
        parent = view.parent_view();
    }
    false
}

/// NSH drawing delegates a registered component's children to its GPUI renderer.
pub(crate) fn owns_children(view: &NshView) -> bool {
    matches!(view.kind(), ViewKind::Native)
}

fn focus_tracking_handles(component: &dyn NativeComponent) -> Vec<FocusHandle> {
    let mut handles = component.focus_handles();
    if let Some(primary) = component.focus_handle()
        && !handles.contains(&primary)
    {
        handles.push(primary);
    }
    handles
}

/// True when focus is inside one of the surface's components: keys go
/// there, not to nsh.
pub(crate) fn focused(surface: &Surface, window: &Window, cx: &App) -> bool {
    let in_frame = surface
        .native
        .focused
        .borrow()
        .iter()
        .any(|(_, f)| f.contains_focused(window, cx));
    in_frame
        || surface.native.hosts.borrow().values().any(|h| match h {
            Host::Component(item) => item.component.try_borrow().is_ok_and(|c| {
                c.as_ref().is_some_and(|c| {
                    focus_tracking_handles(c.as_ref())
                        .iter()
                        .any(|f| f.contains_focused(window, cx))
                })
            }),
            _ => false,
        })
}

/// How many components the surface has.
pub(crate) fn component_count(surface: &Surface) -> usize {
    surface
        .native
        .hosts
        .borrow()
        .values()
        .filter(|h| matches!(h, Host::Component(_)))
        .count()
}

/// True when the component in node `id` takes the keyboard (a Tab stop).
pub(crate) fn focusable(surface: &Surface, id: NodeId) -> bool {
    focus_count(surface, id) > 0
}

/// Number of Tab stops this native component currently exposes.
pub(crate) fn focus_count(surface: &Surface, id: NodeId) -> usize {
    component_item(surface, id)
        .and_then(|item| {
            // Mounted nodes inside compound widgets can remain alive while their
            // slot is inactive. Only their actual GPUI placement is a Tab stop.
            if surface
                .view(id)
                .is_some_and(|view| has_component_ancestor(surface, &view))
            {
                let hosted: Rc<dyn Hosted> = item.clone();
                if !surface
                    .native
                    .composed_placed
                    .borrow()
                    .iter()
                    .any(|(placed, bounds)| Rc::ptr_eq(placed, &hosted) && !empty(bounds))
                {
                    return Some(0);
                }
            }
            let c = item.component.try_borrow().ok()?;
            Some(c.as_ref()?.focus_handles().len())
        })
        .unwrap_or(0)
}

/// The registered widget's sequential focus priority.
pub(crate) fn tab_index(surface: &Surface, id: NodeId) -> isize {
    let hosts = surface.native.hosts.borrow();
    match hosts.get(&id) {
        Some(Host::Component(item)) => item
            .component
            .try_borrow()
            .ok()
            .and_then(|component| component.as_ref().map(|component| component.tab_index()))
            .unwrap_or(0),
        _ => 0,
    }
}

/// Tab stop inside the component that owned focus at the last prepaint.
pub(crate) fn focused_index(surface: &Surface, id: NodeId) -> usize {
    let focused = surface
        .native
        .focused
        .borrow()
        .iter()
        .find(|(node, _)| *node == id)
        .map(|(_, handle)| handle.clone());
    focused
        .and_then(|focus| {
            component_item(surface, id).and_then(|item| {
                let c = item.component.try_borrow().ok()?;
                c.as_ref()?
                    .focus_handles()
                    .iter()
                    .position(|handle| handle == &focus)
            })
        })
        .unwrap_or(0)
}

/// The node of the component focus was inside at the last frame.
pub(crate) fn focused_component(surface: &Surface) -> Option<NodeId> {
    surface.native.focused.borrow().first().map(|(id, _)| *id)
}

/// Tab moved to the component in node `id`: [`apply_focus`] focuses it.
pub(crate) fn tab_to(surface: &Surface, id: NodeId, index: usize) {
    surface.native.tab_to.set(Some((id, index)));
}

/// After nsh handled Tab (with the window, which focusing needs): focuses
/// the component Tab moved to, if any; otherwise, when Tab moved from a
/// component to one of nsh's text fields, gives the window's focus back to
/// the surface (`own`).
pub(crate) fn apply_focus(
    surface: &Rc<Surface>,
    own: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let handle = surface.native.tab_to.take().and_then(|(id, index)| {
        let item = component_item(surface, id)?;
        let c = item.component.try_borrow().ok()?;
        c.as_ref()?.focus_handles().into_iter().nth(index)
    });
    match handle {
        Some(handle) => {
            crate::text_input::blur_all(surface);
            window.focus(&handle, cx);
            surface.invalidate();
        }
        None if focused(surface, window, cx) && crate::text_input::focused(surface).is_some() => {
            window.focus(own, cx);
            surface.invalidate();
        }
        None => {}
    }
}

/// Where a native control goes in the window: its view's box (window
/// coordinates) and the part of it its clipping ancestors leave visible.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Geometry {
    /// The whole box.
    pub(crate) document: Bounds<Pixels>,
    /// Its visible part (empty when clipped away, or when the box is).
    pub(crate) visible: Bounds<Pixels>,
}

impl Geometry {
    /// True when some of the box shows.
    pub(crate) fn shows(&self) -> bool {
        !empty(&self.visible)
    }
}

/// Where `view` is in the window, `None` when it is not attached or it or
/// an ancestor is hidden or transparent.
fn geometry(surface: &Surface, view: &NshView) -> Option<Geometry> {
    let (w, h) = {
        let s = view.try_state()?;
        (s.frame.width, s.frame.height)
    };
    let (ox, oy) = surface.origin.get();
    let to_window = |v: &NshView, w: f32, h: f32| {
        let (x, y, bw, bh) = v.to_root()?.bounds(0.0, 0.0, w, h)?;
        Some(crate::render::bounds(x + ox, y + oy, bw, bh))
    };
    let document = to_window(view, w, h)?;
    let (sw, sh) = surface.size();
    let mut visible = document.intersect(&crate::render::bounds(ox, oy, sw, sh));
    let mut node = Some(view.clone());
    while let Some(v) = node {
        let (shows, clips, size) = {
            let s = v.try_state()?;
            (
                !s.style.hidden && !s.concealed && s.opacity() > 0.0,
                s.clips(),
                (s.frame.width, s.frame.height),
            )
        };
        if !shows {
            return None;
        }
        if clips
            && v != *view
            && let Some(r) = to_window(&v, size.0, size.1)
        {
            visible = visible.intersect(&r);
        }
        node = v.parent_view();
    }
    Some(Geometry { document, visible })
}

/// True when `b` has no area.
fn empty(b: &Bounds<Pixels>) -> bool {
    b.size.width <= px(0.0) || b.size.height <= px(0.0)
}

fn component_item(surface: &Surface, id: NodeId) -> Option<Rc<ComponentItem>> {
    match surface.native.hosts.borrow().get(&id) {
        Some(Host::Component(item)) => Some(item.clone()),
        _ => None,
    }
}

/// Hosts `item` in `view`, filling it ([`resized`] follows the frame).
fn fill(view: &NshView, item: &Rc<dyn Hosted>) {
    let frame = view.state().frame;
    view.set_native(item, bounds(frame.width, frame.height));
}

fn bounds(width: f32, height: f32) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
    }
}

/// A registered component in its container.
struct ComponentItem {
    surface: Weak<Surface>,
    id: NodeId,
    /// `None` until named (or when no factory has the name), and while the
    /// app's code runs.
    component: RefCell<Option<Box<dyn NativeComponent>>>,
    /// The element the last prepaint laid out, painted when the tree draws
    /// the container.
    element: RefCell<Option<AnyElement>>,
    /// Props and commands for the next prepaint, which has the window.
    pending: RefCell<Vec<Pending>>,
    reported_size: std::cell::Cell<Option<Size>>,
}

/// A prop or command waiting for [`ComponentItem::deliver`].
enum Pending {
    Prop(Arc<str>, Option<JsonValue>),
    Command(Arc<str>, JsonValue),
}

impl ComponentItem {
    /// Reconciles natural GPUI layout with NSH's native leaf measurement.
    fn report_size(&self, measured: gpui::Size<Pixels>) {
        let size = Size::new(f32::from(measured.width), f32::from(measured.height));
        if !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.0
            || size.height <= 0.0
        {
            return;
        }
        if self.reported_size.replace(Some(size)) != Some(size)
            && let Some(surface) = self.surface.upgrade()
        {
            surface.control(|control| control.set_component_size(self.id, size));
        }
    }

    /// Puts `component` back, unless the view was deleted meanwhile (then
    /// it drops).
    fn put(&self, component: Box<dyn NativeComponent>) {
        let alive = self.surface.upgrade().is_some_and(|s| {
            matches!(
                s.native.hosts.borrow().get(&self.id),
                Some(Host::Component(_))
            )
        });
        if alive {
            *self.component.borrow_mut() = Some(component);
        }
    }

    /// Hands the props and commands that came since the last frame to the
    /// component, in order.
    fn deliver(&self, window: &mut Window, cx: &mut App) {
        let pending = std::mem::take(&mut *self.pending.borrow_mut());
        if pending.is_empty() {
            return;
        }
        // Taken out while the app's code runs, so it may reach the surface.
        let Some(mut component) = self.component.borrow_mut().take() else {
            return;
        };
        for p in &pending {
            match p {
                Pending::Prop(name, value) => component.set_prop(name, value.as_ref(), window, cx),
                Pending::Command(name, args) if name.as_ref() == "__nsh.controlled_ack" => {
                    if let Some(props) = args.as_object() {
                        for (name, value) in props {
                            component.set_prop(name, Some(value), window, cx);
                        }
                    }
                }
                Pending::Command(name, args) => component.command(name, args, window, cx),
            }
        }
        self.put(component);
    }

    fn close(&self) {
        self.pending.borrow_mut().clear();
        self.element.borrow_mut().take();
        // Dropped here, with whatever the component holds.
        let component = self.component.borrow_mut().take();
        drop(component);
    }

    /// Asks the component for its element and prepaints it at `geometry`
    /// (nothing when `None`). Returns the focus handle focused inside it.
    fn prepaint(
        &self,
        geometry: Option<Geometry>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<FocusHandle> {
        self.element.borrow_mut().take();
        // Asked while its box is empty too: it may size itself
        // ([`ComponentContext::set_size`]).
        let geometry = geometry.filter(|g| g.shows() || empty(&g.document))?;
        // Taken out while the app's code runs, so it may reach the surface.
        let mut component = self.component.borrow_mut().take()?;
        let element = component.render(window, cx);
        self.put(component);
        let size = geometry.document.size;
        let mask = ContentMask {
            bounds: geometry.visible,
        };
        let (focus, element) = window.with_content_mask(Some(mask), |window| {
            // Under the component's own hitboxes: nsh's input (and the
            // embedding app's, under the surface) stays away from its box.
            if geometry.shows() {
                window.insert_hitbox(geometry.visible, HitboxBehavior::BlockMouse);
            }
            let mut root = element?;
            let hints = self
                .surface
                .upgrade()
                .and_then(|surface| surface.native.layout_hints.borrow().get(&self.id).copied())
                .unwrap_or_default();
            if hints.0.is_some() || hints.1.is_some() {
                root = gpui::div()
                    .flex()
                    .flex_col()
                    .when_some(hints.0, |root, width| root.w(px(width)))
                    .when_some(hints.1, |root, height| root.h(px(height)))
                    .child(root)
                    .into_any_element();
            }
            let id = ("nsh-native", self.id as usize);
            let focus = window.with_element_namespace(id, |window| {
                // Definite space honors the NSH allocation. An unmeasured
                // dimension gets intrinsic GPUI content size, then reports
                // it to NSH rather than remaining an invisible zero-size leaf.
                let measured = root.layout_as_root(
                    gpui::size(
                        if size.width > px(0.0) {
                            AvailableSpace::Definite(size.width)
                        } else {
                            AvailableSpace::MaxContent
                        },
                        if size.height > px(0.0) {
                            AvailableSpace::Definite(size.height)
                        } else {
                            AvailableSpace::MaxContent
                        },
                    ),
                    window,
                    cx,
                );
                self.report_size(measured);
                root.prepaint_at(geometry.document.origin, window, cx)
            });
            Some((focus, root))
        })?;
        *self.element.borrow_mut() = Some(element);
        focus
    }
}

impl Hosted for ComponentItem {
    fn draw(&self, cx: &mut DrawCx<'_>, _size: (f32, f32)) {
        let Some(mut element) = self.element.borrow_mut().take() else {
            return;
        };
        if cx.culled() {
            return;
        }
        // In the namespace it was prepainted in, as GPUI requires.
        let id = ("nsh-native", self.id as usize);
        cx.paint(|window, app| {
            window.with_element_namespace(id, |window| element.paint(window, app))
        });
    }

    fn hit(&self, _x: f32, _y: f32, _size: (f32, f32)) -> bool {
        // GPUI's hitboxes give the component its input.
        false
    }

    fn placed(&self, rect: Option<Bounds<Pixels>>) {
        let taken = self.component.borrow_mut().take();
        if let Some(mut component) = taken {
            component.placed(rect);
            self.put(component);
        }
    }
}

/// Where a video's source is.
#[derive(Debug, PartialEq)]
#[cfg_attr(not(feature = "video"), allow(dead_code))]
enum Location {
    /// A URL (`https://`, `file://`, ...).
    Url(String),
    /// A file path.
    Path(std::path::PathBuf),
}

/// Resolves `source` (`asset:/` URIs against `base`).
#[cfg_attr(not(feature = "video"), allow(dead_code))]
fn locate(source: &str, base: Option<&AssetBase>) -> Option<Location> {
    if let Some(path) = source.strip_prefix("asset:/") {
        let path = path.trim_start_matches('/');
        return match base {
            Some(AssetBase::Directory(dir)) => Some(Location::Path(dir.join(path))),
            Some(AssetBase::Http(url)) => Some(Location::Url(format!(
                "{}/{path}",
                url.trim_end_matches('/')
            ))),
            None => None,
        };
    }
    if source.contains("://") {
        Some(Location::Url(source.to_owned()))
    } else if source.is_empty() {
        None
    } else {
        Some(Location::Path(source.into()))
    }
}

/// Sends a control's events to JS, when JS listens to them.
#[derive(Clone)]
struct Emitter {
    surface: Weak<Surface>,
    id: NodeId,
}

impl Emitter {
    fn new(surface: &Rc<Surface>, id: NodeId) -> Self {
        Emitter {
            surface: Rc::downgrade(surface),
            id,
        }
    }

    fn emit(&self, kind: EventKind, payload: impl FnOnce() -> EventPayload) {
        let Some(surface) = self.surface.upgrade() else {
            return;
        };
        // A view whose state is being changed right now cannot be read:
        // send the event, JS ignores events it has no handler for.
        let listens = surface
            .view(self.id)
            .is_some_and(|v| v.try_state().is_none_or(|s| s.events.contains(kind)));
        if listens {
            surface.event(self.id, kind, payload());
        }
    }
}

fn error(message: String) -> EventPayload {
    EventPayload::Map(vec![("message", PayloadValue::Str(message))])
}

/// Paints `text` centered in the current item's `size`, in a neutral gray:
/// what a control that cannot show draws.
fn placeholder(cx: &mut DrawCx<'_>, size: (f32, f32), text: &'static str) {
    if cx.culled() {
        return;
    }
    let font_size = 13.0 * cx.transform_scale();
    let line_height = px(font_size * 1.3);
    let color = cx.color(nsh_core::Color::rgba(140, 140, 140, 255));
    let center = cx.map(size.0 / 2.0, size.1 / 2.0);
    cx.paint(|window, app| {
        let run = gpui::TextRun {
            len: text.len(),
            font: gpui::font(crate::text::ui_family()),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = window
            .text_system()
            .shape_line(text.into(), px(font_size), &[run], None);
        let origin = gpui::point(center.x - line.width / 2.0, center.y - line_height / 2.0);
        let _ = line.paint(
            origin,
            line_height,
            gpui::TextAlign::Left,
            None,
            window,
            app,
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locates_sources() {
        let dir = AssetBase::Directory("/app/assets".into());
        let http = AssetBase::Http("http://10.0.0.2:8790/assets/".into());
        assert_eq!(
            locate("asset:/clips/a.mp4", Some(&dir)),
            Some(Location::Path(
                std::path::Path::new("/app/assets").join("clips/a.mp4")
            ))
        );
        assert_eq!(
            locate("asset:/a.mp4", Some(&http)),
            Some(Location::Url("http://10.0.0.2:8790/assets/a.mp4".into()))
        );
        assert_eq!(locate("asset:/a.mp4", None), None);
        assert_eq!(
            locate("https://example.com/a.mp4", None),
            Some(Location::Url("https://example.com/a.mp4".into()))
        );
        assert_eq!(
            locate("/videos/a.mp4", None),
            Some(Location::Path("/videos/a.mp4".into()))
        );
        assert_eq!(locate("", None), None);
    }

    #[test]
    fn empty_bounds() {
        let b = crate::render::bounds;
        assert!(!empty(&b(0.0, 0.0, 10.0, 10.0)));
        assert!(empty(
            &b(0.0, 0.0, 10.0, 10.0).intersect(&b(10.0, 0.0, 10.0, 10.0))
        ));
        assert!(empty(&b(5.0, 5.0, 0.0, 10.0)));
    }

    #[test]
    fn registers_components() {
        let components = Components::default();
        let factory: ComponentFactory = Rc::new(|_| unreachable!("not made in this test"));
        // Clones share the registry.
        components.clone().register("com.example.map", factory);
        assert!(components.factory(&"com.example.map".into()).is_some());
        assert!(components.factory(&"com.example.none".into()).is_none());
        assert_eq!(format!("{components:?}"), r#"{"com.example.map"}"#);
    }
}
