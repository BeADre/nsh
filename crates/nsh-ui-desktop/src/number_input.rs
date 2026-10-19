//! `NumberInput`: a number field between − and + buttons.

use nsh_core::ui::prelude::*;

use crate::{metrics::ControlSize, text_field::field_box};

/// How far the step buttons sit inside the field's border, in points.
const BUTTON_INSET: f32 = 2.0;

/// A number field between − and + buttons, within `min`–`max`: the
/// buttons step it (holding one keeps stepping), and what is typed is
/// read when Return is pressed or the field loses focus, rounded to the
/// step's decimals and kept within the bounds (text that is no number
/// gives the value back). With `value` the app controls it (it updates
/// `value` from `onChange`); without, the field keeps its own, starting at
/// `defaultValue`.
#[derive(Debug, Clone, Copy, PartialEq, Props)]
pub struct NumberInputProps {
    /// The value, when the app controls it.
    pub value: Prop<Option<f64>>,
    /// The first value, when the app does not control it.
    #[prop(default)]
    pub default_value: Prop<f64>,
    /// The smallest value.
    #[prop(default = 0.0)]
    pub min: Prop<f64>,
    /// The largest value.
    #[prop(default = 100.0)]
    pub max: Prop<f64>,
    /// How much a button press changes it.
    #[prop(default = 1.0)]
    pub step: Prop<f64>,
    /// `sm`, `md` (default) or `lg`.
    #[prop(default)]
    pub size: Prop<ControlSize>,
    /// Not editable, no presses, dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
}

/// The events of a [`NumberInput`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum NumberInputEvent {
    /// The value changed (the new one).
    Change(f64),
}

/// A number field between − and + buttons ([`NumberInputProps`]).
#[component]
pub fn NumberInput(props: NumberInputProps, cx: Scope<NumberInputEvent>) -> impl IntoElement {
    let range =
        computed(move || NumberRange::new(props.min.get(), props.max.get(), props.step.get()));
    // The app's value or our own, shown on the step's decimals and within
    // the bounds either way.
    let given = controllable(
        props.value,
        untracked(|| range.get().fit(props.default_value.get())),
    );
    let value = computed(move || range.get().fit(given.get()));
    // What the user is typing, until it is read: the field shows it
    // instead of the value meanwhile.
    let draft = signal(None::<String>);
    let repeat = press_repeat();
    // Shows `next` (unless the app controls the value) and reports it.
    let change = move |next: f64| {
        if next == value.get_untracked() {
            repeat.stop();
            return;
        }
        given.set(next);
        cx.emit(NumberInputEvent::Change(next));
    };
    // Reads what was typed (a number fitted to the range; anything else
    // gives the value back) and shows the value again.
    let commit = move || {
        if let Some(typed) = draft.get_untracked() {
            draft.set(None);
            if let Ok(number) = typed.trim().parse::<f64>()
                && number.is_finite()
            {
                change(range.get_untracked().fit(number));
            }
        }
    };
    // One step of `direction` (−1 or 1), from what was typed if anything
    // was; stops repeating at a bound.
    let step = move |direction: f64| {
        commit();
        let range = range.get_untracked();
        change(range.step_by(value.get_untracked(), direction));
        let now = value.get_untracked();
        if range.at_min(now) || range.at_max(now) {
            repeat.stop();
        }
    };
    effect(move || {
        if props.disabled.get() {
            repeat.stop();
        }
    });
    let size = props.size;
    let button =
        move |label: &'static str, name: &'static str, direction: f64, at_bound: Memo<bool>| {
            let off = computed(move || props.disabled.get() || at_bound.get());
            // A square inside the field's border, its corners following
            // the field's.
            let side = move || px(size.get().height() - 2.0 * (1.0 + BUTTON_INSET));
            view()
                .w(side)
                .h(side)
                .m(px(BUTTON_INSET))
                .items_center()
                .justify_center()
                .rounded(move || px((size.get().radius() - BUTTON_INSET).max(0.0)))
                .hover(|s| s.bg(move || theme().accent))
                .active(|s| s.bg(move || theme().secondary_active))
                .style_when(off, |v| v.opacity(DISABLED_OPACITY))
                .disabled(off)
                // The field is the Tab stop; the buttons are the pointer's.
                .focusable(false)
                .role(AccessibilityRole::Button)
                .label(name)
                // One step now, more while it is held.
                .on(EventKind::PressIn, move |_: &EventPayload| {
                    repeat.start(move || step(direction));
                })
                .on(EventKind::PressOut, move |_: &EventPayload| repeat.stop())
                .child(
                    text(label)
                        .text_size(move || px(size.get().font_size()))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || theme().foreground),
                )
        };
    let focused = signal(false);
    field_box(size, props.disabled, focused)
        .style_when(props.disabled, |v| v.opacity(DISABLED_OPACITY))
        .child(button(
            "−",
            "Decrease",
            -1.0,
            computed(move || range.get().at_min(value.get())),
        ))
        .child(
            input()
                .flex_1()
                .min_w(px(48.0))
                .text_center()
                .text_size(move || px(size.get().font_size()))
                .text_color(move || theme().foreground)
                .value(move || {
                    draft
                        .get()
                        .unwrap_or_else(|| range.get().format(value.get()))
                })
                .selection_color(move || color(theme().primary))
                .keyboard_type(KeyboardType::NumbersAndPunctuation)
                .editable(move || !props.disabled.get())
                // Return keeps the focus, as on desktop fields.
                .submit_behavior(SubmitBehavior::Submit)
                .on_change_text(move |text: &str| draft.set(Some(text.to_owned())))
                .on_submit_editing(move |_: &str| commit())
                .on_focus(move |_: &()| focused.set(true))
                .on_blur(move |_: &()| {
                    focused.set(false);
                    commit();
                }),
        )
        .child(button(
            "+",
            "Increase",
            1.0,
            computed(move || range.get().at_max(value.get())),
        ))
}
