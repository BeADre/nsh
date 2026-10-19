//! `Switch`: a track and a thumb that slides across it, on or off.

use std::cell::Cell;

use nsh_core::{Checked, ui::prelude::*};

use crate::{
    checkbox::indicator,
    metrics::{ControlSize, HOVER_OPACITY, RING_OFFSET, RING_WIDTH},
};

/// A switch that is on or off, with an optional label beside it; a click on
/// the switch or the label, or Return or Space while it has the keyboard
/// focus, flips it, and the thumb springs across. With `value` the app
/// controls it (it updates `value` from `onChange`); without, the switch
/// keeps its own, starting at `defaultValue`.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct SwitchProps {
    /// On, when the app controls it.
    pub value: Prop<Option<bool>>,
    /// On at first, when the app does not control it.
    #[prop(default)]
    pub default_value: Prop<bool>,
    /// Beside the switch; a press on it flips the switch too.
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

/// The events of a [`Switch`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum SwitchEvent {
    /// The user turned it on or off (on now).
    Change(bool),
}

/// The gap between the thumb and the track's edge, in points.
const INSET: f32 = 2.0;

/// The track's height at `size`: a thumb as large as a check box, and its
/// inset on both sides.
fn track_height(size: ControlSize) -> f32 {
    indicator(size) + 2.0 * INSET
}

/// The track's width: room for the thumb to travel its own size.
fn track_width(size: ControlSize) -> f32 {
    track_height(size) + indicator(size)
}

/// How far the thumb moves from off to on, in points.
fn travel(size: ControlSize) -> f32 {
    track_width(size) - track_height(size)
}

/// A track and a thumb ([`SwitchProps`]).
#[component]
pub fn Switch(props: SwitchProps, cx: Scope<SwitchEvent>) -> impl IntoElement {
    let on = controllable(props.value, props.default_value.get_untracked());
    let size = props.size;
    // How far it is on, 0 to 1: the thumb's place and the on color's
    // strength, moved natively.
    let progress = native_value(if on.get_untracked() { 1.0 } else { 0.0 });
    {
        let progress = progress.clone();
        let first = Cell::new(true);
        effect(move || {
            let to = if on.get() { 1.0 } else { 0.0 };
            // It starts where it is; only changes spring.
            if !first.replace(false) {
                progress.spring(to, motion::SETTLE);
            }
        });
    }
    let fill = progress.clone();
    // The whole row is the press target, a group whose hover and press the
    // track shows.
    view()
        .group()
        .flex_row()
        .items_center()
        .self_start()
        .gap_2()
        .rounded(move || px(track_height(size.get()) / 2.0))
        .focus(|s| {
            s.outline(RING_WIDTH)
                .outline_color(move || theme().ring)
                .outline_offset(RING_OFFSET)
        })
        .style_when(props.disabled, |v| v.opacity(DISABLED_OPACITY))
        .disabled(props.disabled)
        .role(AccessibilityRole::Switch)
        .label(move || {
            props
                .accessibility_label
                .get()
                .or_else(|| props.label.get())
                .unwrap_or_default()
        })
        .accessibility_state(move || AccessibilityState {
            checked: Some(if on.get() {
                Checked::True
            } else {
                Checked::False
            }),
            disabled: props.disabled.get(),
            ..Default::default()
        })
        .on_press(move |_: &()| {
            let next = !on.get_untracked();
            on.set(next);
            cx.emit(SwitchEvent::Change(next));
        })
        .child(
            view()
                .w(move || px(track_width(size.get())))
                .h(move || px(track_height(size.get())))
                .rounded_full()
                .overflow_hidden()
                .bg(move || theme().switch)
                .group_hover(|s| s.bg(move || theme().switch.opacity(HOVER_OPACITY)))
                // The on color, over the off one, as strong as it is on.
                .child(
                    view()
                        .absolute()
                        .inset_0()
                        .bg(move || theme().primary)
                        .group_hover(|s| s.bg(move || theme().primary.opacity(HOVER_OPACITY)))
                        .group_active(|s| s.bg(move || theme().primary_active))
                        .bind_opacity(&fill),
                )
                // Made again for another size, whose travel differs, and
                // another theme, whose colors it blends.
                .child(dynamic(move || {
                    let size = size.get();
                    let side = indicator(size);
                    let theme = theme();
                    view()
                        .absolute()
                        .top(px(INSET))
                        .left(px(INSET))
                        .size(px(side))
                        .rounded_full()
                        // Toward the text color on the on color: a light
                        // thumb on the light track of a dark theme is lost.
                        .bind_background(&progress.interpolate_colors(
                            &[0.0, 1.0],
                            &[theme.switch_thumb, theme.primary_foreground],
                        ))
                        .shadow_sm()
                        .bind_translate_x(&progress.mul(travel(size)))
                })),
        )
        .child(dynamic(move || {
            props.label.get().map(|label| {
                text(label)
                    .text_size(move || px(size.get().font_size()))
                    .text_color(move || theme().foreground)
            })
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The thumb fits the track with the same inset all round, at every
    /// size, and travels its own size.
    #[test]
    fn the_thumb_fits_the_track() {
        for size in [ControlSize::Sm, ControlSize::Md, ControlSize::Lg] {
            let side = indicator(size);
            assert_eq!(track_height(size) - side, 2.0 * INSET);
            assert_eq!(INSET + travel(size) + side + INSET, track_width(size));
            assert_eq!(travel(size), side);
        }
        assert_eq!(indicator(ControlSize::Md), 16.0);
    }
}
