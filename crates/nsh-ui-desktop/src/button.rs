//! `Button`: a press target with a label, for the pointer and the keyboard.

use nsh_core::ui::prelude::*;

use crate::metrics::{ControlSize, HOVER_OPACITY, RING_OFFSET, RING_WIDTH};

/// A press target with a label: highlighted under the pointer, ringed when
/// the keyboard focuses it (Tab), pressed with a click, Return or Space.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct ButtonProps {
    /// The label.
    pub title: Prop<SharedString>,
    /// `primary` (default): filled; `secondary`: tinted; `danger`: a
    /// destructive action; `plain`: text only.
    #[prop(default)]
    pub variant: Prop<ButtonVariant>,
    /// `sm`, `md` (default) or `lg`.
    #[prop(default)]
    pub size: Prop<ControlSize>,
    /// No presses, no feedback, dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
    /// For screen readers, when `title` is not enough.
    pub accessibility_label: Prop<Option<SharedString>>,
}

/// How a [`Button`] looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PropEnum)]
pub enum ButtonVariant {
    /// Filled with the primary color.
    #[default]
    Primary,
    /// Filled with the secondary color.
    Secondary,
    /// Filled with the danger color.
    Danger,
    /// Text only; a background under the pointer and while pressed.
    Plain,
}

/// The events of a [`Button`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum ButtonEvent {
    /// The button was pressed.
    Press,
}

/// The colors of a variant.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Colors {
    background: AdaptiveColor,
    hovered: AdaptiveColor,
    pressed: AdaptiveColor,
    label: AdaptiveColor,
}

/// The colors of `variant` in the theme (reading subscribes to it).
fn colors(variant: ButtonVariant) -> Colors {
    let theme = theme();
    let filled = |background: AdaptiveColor, pressed, label| Colors {
        background,
        hovered: background.opacity(HOVER_OPACITY),
        pressed,
        label,
    };
    match variant {
        ButtonVariant::Primary => filled(
            theme.primary,
            theme.primary_active,
            theme.primary_foreground,
        ),
        ButtonVariant::Secondary => filled(
            theme.secondary,
            theme.secondary_active,
            theme.secondary_foreground,
        ),
        ButtonVariant::Danger => filled(theme.danger, theme.danger_active, theme.danger_foreground),
        ButtonVariant::Plain => Colors {
            background: AdaptiveColor::TRANSPARENT,
            hovered: theme.accent,
            pressed: theme.secondary_active,
            label: theme.foreground,
        },
    }
}

/// A press target with a label ([`ButtonProps`]).
#[component]
pub fn Button(props: ButtonProps, cx: Scope<ButtonEvent>) -> impl IntoElement {
    let colors = computed(move || colors(props.variant.get()));
    let size = props.size;
    view()
        .flex_row()
        .items_center()
        .justify_center()
        .h(move || px(size.get().height()))
        .px(move || px(size.get().padding_x()))
        .rounded(move || px(size.get().radius()))
        .bg(move || colors.get().background)
        .hover(|s| s.bg(move || colors.get().hovered))
        .active(|s| s.bg(move || colors.get().pressed))
        .focus(|s| {
            s.outline(RING_WIDTH)
                .outline_color(move || theme().ring)
                .outline_offset(RING_OFFSET)
        })
        .style_when(props.disabled, |v| v.opacity(DISABLED_OPACITY))
        .disabled(props.disabled)
        .role(AccessibilityRole::Button)
        .label(move || {
            props
                .accessibility_label
                .get()
                .unwrap_or_else(|| props.title.get())
        })
        .on_press(move |_: &()| cx.emit(ButtonEvent::Press))
        .child(
            text(props.title)
                .text_size(move || px(size.get().font_size()))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || colors.get().label),
        )
}
