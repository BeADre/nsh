//! Live NSH subtrees composed inside GPUI component elements.
use gpui::{
    Element, ElementId, GlobalElementId, InspectorElementId, LayoutId, prelude::FluentBuilder,
};

use super::*;
use crate::geom::Mat4;

/// A live NSH child or named slot usable anywhere a GPUI element is accepted.
/// This handle retains only node identity; removing the NSH node immediately
/// makes future frames empty and drops its original native component state.
#[derive(Clone)]
pub struct NativeSubtree {
    surface: u64,
    id: NodeId,
    generation: u64,
}
impl NativeSubtree {
    pub(super) fn new(surface: u64, id: NodeId) -> Self {
        let generation = Surface::by_key(surface)
            .map(|s| s.generation())
            .unwrap_or(u64::MAX);
        Self {
            surface,
            id,
            generation,
        }
    }
    fn surface(&self) -> Option<Rc<Surface>> {
        Surface::by_key(self.surface).filter(|surface| surface.generation() == self.generation)
    }
    fn paint_decoration(
        &self,
        bounds: Bounds<Pixels>,
        background: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(surface) = self.surface() else {
            return;
        };
        if component_item(&surface, self.id).is_none() {
            return;
        }
        let Some(view) = surface.view(self.id) else {
            return;
        };
        // Native composition bypasses draw_view, but the native node's visual
        // box still belongs to NSH. Reuse its painter instead of translating
        // every style property into a second GPUI builder schema.
        let mask = window.content_mask().bounds;
        let mut draw = DrawCx::new(window, cx, mask, crate::system::dark(&surface));
        draw.set_transform(
            Mat4::new().translate(f32::from(bounds.origin.x), f32::from(bounds.origin.y)),
        );
        let state = view.state();
        if background {
            crate::decor::draw_background(&mut draw, &state);
        } else {
            crate::decor::draw_border(&mut draw, &state);
        }
    }
    /// Stable identity of the mounted NSH node.
    pub fn node_id(&self) -> NodeId {
        self.id
    }
}
impl IntoElement for NativeSubtree {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for NativeSubtree {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(("nsh-subtree", self.id as usize).into())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let mut element = (|| {
            let surface = self.surface()?;
            surface.view(self.id)?;
            if let Some(item) = component_item(&surface, self.id) {
                item.deliver(window, cx);
                let mut component = item.component.borrow_mut().take()?;
                let rendered = component.render(window, cx);
                item.put(component);
                let (width, height) = if surface.native.slots.borrow().contains_key(&self.id) {
                    // Slot width is feedback from GPUI to NSH, never a new
                    // constraint imposed back on the slot's GPUI parent.
                    (None, None)
                } else {
                    surface
                        .native
                        .layout_hints
                        .borrow()
                        .get(&self.id)
                        .copied()
                        .unwrap_or_default()
                };
                rendered.map(|element| {
                    if width.is_none() && height.is_none() {
                        element
                    } else {
                        gpui::div()
                            .flex()
                            .flex_col()
                            .when_some(width, |root, width| root.w(px(width)))
                            .when_some(height, |root, height| root.h(px(height)))
                            .child(element)
                            .into_any_element()
                    }
                })
            } else {
                Some(
                    NshContent {
                        surface: self.surface,
                        id: self.id,
                        generation: self.generation,
                    }
                    .into_any_element(),
                )
            }
        })()
        .unwrap_or_else(|| gpui::div().into_any_element());
        let layout = element.request_layout(window, cx);
        (layout, element)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        element: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(surface) = self.surface() {
            if surface.native.slots.borrow().contains_key(&self.id) {
                let width = f32::from(bounds.size.width);
                let old = surface
                    .native
                    .slot_widths
                    .borrow_mut()
                    .insert(self.id, width);
                if width > 0.0 && old != Some(width) {
                    surface.event(
                        self.id,
                        EventKind::NativeEvent,
                        EventPayload::Native {
                            name: "measure".into(),
                            data: serde_json::json!({ "width": width }),
                        },
                    );
                }
            }
            if let Some(item) = component_item(&surface, self.id) {
                item.report_size(bounds.size);
                surface
                    .native
                    .composed_placed
                    .borrow_mut()
                    .push((item, bounds));
            }
            if let Some(view) = surface.view(self.id) {
                // Preserve NSH's original tree and ownership while reflecting
                // the GPUI layout position for NSH hit testing and descendants.
                let (ox, oy) = surface.origin.get();
                let parent_origin = view
                    .parent_view()
                    .and_then(|p| p.to_root())
                    .and_then(|m| m.bounds(0.0, 0.0, 0.0, 0.0))
                    .map(|b| (b.0, b.1))
                    .unwrap_or_default();
                let mut state = view.state_mut();
                state.frame.x = f32::from(bounds.origin.x) - ox - parent_origin.0;
                state.frame.y = f32::from(bounds.origin.y) - oy - parent_origin.1;
                state.frame.width = f32::from(bounds.size.width);
                state.frame.height = f32::from(bounds.size.height);
                drop(state);
                resized(&view);
            }
        }
        if let Some(focus) = element.prepaint(window, cx)
            && let Some(surface) = self.surface()
        {
            surface.native.focused.borrow_mut().push((self.id, focus));
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        element: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.paint_decoration(bounds, true, window, cx);
        element.paint(window, cx);
        self.paint_decoration(bounds, false, window, cx);
    }
}

/// Paints a regular NSH View/Text/Image subtree inside GPUI composition.
struct NshContent {
    surface: u64,
    id: NodeId,
    generation: u64,
}
impl NshContent {
    fn surface(&self) -> Option<Rc<Surface>> {
        Surface::by_key(self.surface).filter(|surface| surface.generation() == self.generation)
    }
}
impl IntoElement for NshContent {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for NshContent {
    type RequestLayoutState = ();
    type PrepaintState = gpui::Hitbox;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let frame = self
            .surface()
            .and_then(|s| s.view(self.id))
            .map(|v| v.state().frame)
            .unwrap_or_default();
        let mut style = gpui::Style::default();
        style.size.width = px(frame.width).into();
        style.size.height = px(frame.height).into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::Hitbox {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        if let Some(surface) = self.surface()
            && let Some(view) = surface.view(self.id)
        {
            prepare_descendants(&surface, &view, window, cx);
        }
        hitbox
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _hitbox: &mut gpui::Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(surface) = self.surface() else {
            return;
        };
        let Some(view) = surface.view(self.id) else {
            return;
        };
        if let Some((focus, view)) = surface.native.subtree_events.borrow().as_ref() {
            crate::events::install_pointer(&surface, focus, _hitbox, Some(self.id), window, cx);
            let _ = view;
        }
        let mask = window.content_mask().bounds;
        let mut draw = DrawCx::new(window, cx, mask, crate::system::dark(&surface));
        crate::render::draw_view(
            &mut draw,
            &view,
            Mat4::new(),
            (f32::from(bounds.origin.x), f32::from(bounds.origin.y)),
        );
        // Hosted items are reported here too, including overlay placement.
        for (item, bounds) in draw.drawn {
            item.placed(Some(bounds));
        }
    }
}
fn prepare_descendants(surface: &Rc<Surface>, view: &NshView, window: &mut Window, cx: &mut App) {
    for child in view.child_views() {
        if let Some(item) = component_item(surface, child.id()) {
            item.deliver(window, cx);
            let geometry = geometry(surface, &child);
            item.prepaint(geometry, window, cx);
            if let Some(geometry) = geometry {
                surface
                    .native
                    .composed_placed
                    .borrow_mut()
                    .push((item, geometry.document));
            }
        } else {
            prepare_descendants(surface, &child, window, cx);
        }
    }
}

pub(super) struct Slot {
    pub(super) context: ComponentContext,
}
impl NativeComponent for Slot {
    fn render(&mut self, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        Some(
            gpui::div()
                .flex()
                .flex_col()
                .min_w_0()
                .max_w_full()
                .children(self.context.children())
                .into_any_element(),
        )
    }
    fn set_prop(&mut self, _: &str, _: Option<&JsonValue>, _: &mut Window, _: &mut App) {}
}
