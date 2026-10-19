//! `Badge`: a small label for a status or a count.

use nsh_core::ui::prelude::*;

use crate::metrics::{label_font_size, radius_for};

/// How tall a badge is, in points.
const HEIGHT: f32 = 20.0;

/// A small rounded label for a status, a category or a count, next to
/// what it describes.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct BadgeProps {
    /// The text.
    pub label: Prop<SharedString>,
    /// `secondary` (default): quiet; `primary`: prominent; `danger`: an
    /// error or a destructive state; `outline`: a border only.
    #[prop(default)]
    pub variant: Prop<BadgeVariant>,
}

/// How a [`Badge`] looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PropEnum)]
pub enum BadgeVariant {
    /// Filled with the secondary color.
    #[default]
    Secondary,
    /// Filled with the primary color.
    Primary,
    /// Filled with the danger color.
    Danger,
    /// A border, no fill.
    Outline,
}

/// The background, border and text colors of `variant` in the theme
/// (reading subscribes to it).
fn colors(variant: BadgeVariant) -> (AdaptiveColor, AdaptiveColor, AdaptiveColor) {
    let theme = theme();
    let filled = |background: AdaptiveColor, label| (background, background, label);
    match variant {
        BadgeVariant::Secondary => filled(theme.secondary, theme.secondary_foreground),
        BadgeVariant::Primary => filled(theme.primary, theme.primary_foreground),
        BadgeVariant::Danger => filled(theme.danger, theme.danger_foreground),
        BadgeVariant::Outline => (AdaptiveColor::TRANSPARENT, theme.border, theme.foreground),
    }
}

/// A small rounded label ([`BadgeProps`]).
#[component]
pub fn Badge(props: BadgeProps) -> impl IntoElement {
    let colors = computed(move || colors(props.variant.get()));
    view()
        .flex_row()
        .items_center()
        .justify_center()
        .h(px(HEIGHT))
        .px_2()
        .rounded(move || px(radius_for(HEIGHT)))
        .bg(move || colors.get().0)
        // Every variant has the border, so they are all the same size.
        .border_1()
        .border_color(move || colors.get().1)
        .child(
            text(props.label)
                .number_of_lines(1)
                .text_size(move || px(label_font_size()))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || colors.get().2),
        )
}
