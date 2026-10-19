//! `Checkbox`: a box with a check mark, and a label beside it.

use nsh_core::{Checked, ui::prelude::*};

use crate::metrics::{ControlSize, HOVER_OPACITY, RING_OFFSET, RING_WIDTH, radius_for};

/// A box that is checked or not, with an optional label beside it; a click
/// on the box or the label, or Return or Space while it has the keyboard
/// focus, toggles it. With `value` the app controls it (it updates `value`
/// from `onChange`); without, the checkbox keeps its own, starting at
/// `defaultValue`.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct CheckboxProps {
    /// Checked, when the app controls it.
    pub value: Prop<Option<bool>>,
    /// Checked at first, when the app does not control it.
    #[prop(default)]
    pub default_value: Prop<bool>,
    /// Beside the box; a press on it toggles the box too.
    pub label: Prop<Option<SharedString>>,
    /// `sm`, `md` (default) or `lg`.
    #[prop(default)]
    pub size: Prop<ControlSize>,
    /// No presses, no feedback, dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
    /// For screen readers, when `label` is not enough (or there is none).
    pub accessibility_label: Prop<Option<SharedString>>,
}

/// The events of a [`Checkbox`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum CheckboxEvent {
    /// The user checked or unchecked it (checked now).
    Change(bool),
}

/// The side of the box, in points: small next to the control's text, as a
/// check box is in a form, and square at every size.
pub(crate) fn indicator(size: ControlSize) -> f32 {
    match size {
        ControlSize::Sm => 14.0,
        ControlSize::Md => 16.0,
        ControlSize::Lg => 20.0,
    }
}

/// The colors of the box.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Colors {
    background: AdaptiveColor,
    hovered: AdaptiveColor,
    pressed: AdaptiveColor,
    border: AdaptiveColor,
}

/// The box's colors, checked or not, in the theme (reading subscribes to
/// it).
fn colors(checked: bool) -> Colors {
    let theme = theme();
    if checked {
        Colors {
            background: theme.primary,
            hovered: theme.primary.opacity(HOVER_OPACITY),
            pressed: theme.primary_active,
            border: theme.primary,
        }
    } else {
        Colors {
            background: theme.background,
            hovered: theme.accent,
            pressed: theme.secondary_active,
            border: theme.input,
        }
    }
}

/// A box with a check mark ([`CheckboxProps`]).
#[component]
pub fn Checkbox(props: CheckboxProps, cx: Scope<CheckboxEvent>) -> impl IntoElement {
    let checked = controllable(props.value, props.default_value.get_untracked());
    let colors = computed(move || colors(checked.get()));
    let size = props.size;
    let side = move || indicator(size.get());
    // The check mark: a corner (two borders) turned onto its point, in
    // proportion to the box.
    let mark = move || {
        view()
            .w(move || px(side() * 0.55))
            .h(move || px(side() * 0.3))
            .mb(move || px(side() * 0.12))
            .border_l(move || side() / 8.0)
            .border_b(move || side() / 8.0)
            .border_color(move || theme().primary_foreground)
            .rotate(-45.0)
    };
    // The whole row is the press target, a group whose hover and press the
    // box shows.
    view()
        .group()
        .flex_row()
        .items_center()
        .self_start()
        .gap_2()
        .rounded(move || px(radius_for(side())))
        .focus(|s| {
            s.outline(RING_WIDTH)
                .outline_color(move || theme().ring)
                .outline_offset(RING_OFFSET)
        })
        .style_when(props.disabled, |v| v.opacity(DISABLED_OPACITY))
        .disabled(props.disabled)
        .role(AccessibilityRole::Checkbox)
        .label(move || {
            props
                .accessibility_label
                .get()
                .or_else(|| props.label.get())
                .unwrap_or_default()
        })
        .accessibility_state(move || AccessibilityState {
            checked: Some(if checked.get() {
                Checked::True
            } else {
                Checked::False
            }),
            disabled: props.disabled.get(),
            ..Default::default()
        })
        .on_press(move |_: &()| {
            let next = !checked.get_untracked();
            checked.set(next);
            cx.emit(CheckboxEvent::Change(next));
        })
        .child(
            view()
                .size(move || px(side()))
                .items_center()
                .justify_center()
                .rounded(move || px(radius_for(side())))
                .border_1()
                .border_color(move || colors.get().border)
                .bg(move || colors.get().background)
                .group_hover(|s| s.bg(move || colors.get().hovered))
                .group_active(|s| s.bg(move || colors.get().pressed))
                .child(If::new(move || checked.get(), mark)),
        )
        .child(dynamic(move || {
            props.label.get().map(|label| {
                text(label)
                    .text_size(move || px(size.get().font_size()))
                    .text_color(move || theme().foreground)
            })
        }))
}
