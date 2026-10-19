//! `Select`: one of a list of options, picked from a popover.

use nsh_core::ui::prelude::*;

use crate::{
    metrics::{ControlSize, FIELD_HOVER_OPACITY, RING_OFFSET, RING_WIDTH},
    popover::{Popover, PopoverEvent, PopoverProps},
};

/// One of `options`, shown in a field that opens the list below it (above,
/// when the window has no room) to pick another. With `value` the app
/// controls the choice (it updates `value` from `onChange`); without, the
/// select keeps its own, starting at `defaultValue`.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct SelectProps {
    /// The options.
    pub options: Prop<Vec<SharedString>>,
    /// The index of the chosen option, when the app controls it.
    pub value: Prop<Option<u32>>,
    /// The first choice, when the app does not control it (none by
    /// default).
    pub default_value: Prop<Option<u32>>,
    /// Shown while nothing is chosen.
    pub placeholder: Prop<Option<SharedString>>,
    /// `sm`, `md` (default) or `lg`.
    #[prop(default)]
    pub size: Prop<ControlSize>,
    /// Not opened, dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
}

/// The events of a [`Select`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum SelectEvent {
    /// An option was picked (its index).
    Change(u32),
}

/// The field's chevron: a corner (two borders) turned onto its point, in
/// proportion to the text, as the check box's mark is drawn.
fn chevron(size: Prop<ControlSize>) -> View {
    let side = move || size.get().font_size() * 0.4;
    view()
        .w(move || px(side()))
        .h(move || px(side()))
        .mb(move || px(side() * 0.5))
        .border_r(move || side() / 4.0)
        .border_b(move || side() / 4.0)
        .border_color(move || theme().muted_foreground)
        .rotate(45.0)
}

/// One of a list of options ([`SelectProps`]).
#[component]
pub fn Select(props: SelectProps, cx: Scope<SelectEvent>) -> impl IntoElement {
    let chosen = controllable(
        move || props.value.get().map(Some),
        props.default_value.get_untracked(),
    );
    let open = signal(false);
    // The field's width: the list is at least as wide.
    let width = signal(0.0_f32);
    let size = props.size;
    let options = props.options;
    let label = computed(move || {
        chosen
            .get()
            .and_then(|i| options.with(|o| o.get(i as usize).cloned()))
    });
    let field = view()
        .flex_row()
        .items_center()
        .gap_2()
        .h(move || px(size.get().height()))
        .px(move || px(size.get().padding_x()))
        .rounded(move || px(size.get().radius()))
        .bg(move || theme().background)
        .border_1()
        .border_color(move || theme().input)
        .hover(|s| s.border_color(move || theme().ring.opacity(FIELD_HOVER_OPACITY)))
        .active(|s| s.bg(move || theme().accent))
        .focus(|s| {
            s.outline(RING_WIDTH)
                .outline_color(move || theme().ring)
                .outline_offset(RING_OFFSET)
        })
        .style_when(props.disabled, |v| v.opacity(DISABLED_OPACITY))
        .disabled(props.disabled)
        .role(AccessibilityRole::Button)
        .accessibility_state(move || AccessibilityState {
            expanded: Some(open.get()),
            ..Default::default()
        })
        .label(move || {
            label
                .get()
                .or_else(|| props.placeholder.get())
                .unwrap_or_default()
        })
        .on_layout(move |frame: &nsh_core::Rect| width.set(frame.width))
        .on_press(move |_: &()| open.set(true))
        .child(
            text(move || {
                label
                    .get()
                    .or_else(|| props.placeholder.get())
                    .map_or_else(String::new, |t| t.as_str().to_owned())
            })
            // Its own width at least: a field sized by its content keeps
            // the label.
            .flex_grow(1.0)
            .number_of_lines(1)
            .text_size(move || px(size.get().font_size()))
            .text_color(move || {
                if label.with(Option::is_some) {
                    theme().foreground
                } else {
                    theme().muted_foreground
                }
            }),
        )
        .child(chevron(size));
    let list = view()
        .min_w(move || px(width.get()))
        .child(dynamic(move || {
            let count = options.with(Vec::len);
            (0..count)
                .map(|i| {
                    let index = i as u32;
                    let on = computed(move || chosen.get() == Some(index));
                    view()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .h(move || px(size.get().height()))
                        .px(move || px(size.get().padding_x()))
                        .rounded(move || px(size.get().radius()))
                        .hover(|s| s.bg(move || theme().accent))
                        .active(|s| s.bg(move || theme().secondary_active))
                        .focus(|s| s.bg(move || theme().accent))
                        .role(AccessibilityRole::Button)
                        .accessibility_state(move || AccessibilityState {
                            selected: on.get(),
                            ..Default::default()
                        })
                        .on_press(move |_: &()| {
                            chosen.set(Some(index));
                            open.set(false);
                            cx.emit(SelectEvent::Change(index));
                        })
                        .child(
                            text(move || {
                                options.with(|o| {
                                    o.get(i).map_or_else(String::new, |t| t.as_str().to_owned())
                                })
                            })
                            .flex_grow(1.0)
                            .text_size(move || px(size.get().font_size()))
                            .text_color(move || theme().popover_foreground),
                        )
                        .child(If::new(on, move || {
                            text("✓")
                                .text_size(move || px(size.get().font_size()))
                                .text_color(move || theme().popover_foreground)
                        }))
                        .into_element()
                })
                .collect::<Vec<_>>()
        }));
    Popover::element(PopoverProps {
        open: open.into(),
        ..PopoverProps::default()
    })
    .fill("trigger", field)
    .child(list)
    .on_event(move |_: &PopoverEvent| open.set(false))
}
