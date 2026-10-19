//! `Stepper`: a number with − and + buttons; holding one keeps stepping.

use std::rc::Rc;

use nsh_core::ui::prelude::*;

/// A number with − and + buttons, within `min`–`max`; holding a button
/// keeps stepping. With `value` the app controls it (it updates `value`
/// from `onChange`); without, the stepper keeps its own, starting at
/// `defaultValue`.
#[derive(Debug, Clone, Copy, PartialEq, Props)]
pub struct StepperProps {
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
    /// How much a press changes it.
    #[prop(default = 1.0)]
    pub step: Prop<f64>,
    /// No presses, dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
}

/// The events of a [`Stepper`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum StepperEvent {
    /// The value changed (the new one).
    Change(f64),
}

/// The bounds and the step of `props`.
fn range(props: &StepperProps) -> NumberRange {
    NumberRange::new(props.min.get(), props.max.get(), props.step.get())
}

/// What JS calls on a stepper's ref.
pub struct StepperApi {
    value: Memo<f64>,
    range: Memo<NumberRange>,
    change: Rc<dyn Fn(f64)>,
}

#[methods]
impl StepperApi {
    /// Sets the value (rounded to the step's decimals, within the bounds)
    /// and reports it as a press would.
    pub fn set_value(&self, value: f64) {
        (self.change)(self.range.get_untracked().fit(value));
    }

    /// The value shown.
    pub fn value(&self) -> f64 {
        self.value.get_untracked()
    }
}

/// A number with − and + buttons ([`StepperProps`]).
#[component(methods = StepperApi)]
pub fn Stepper(props: StepperProps, cx: Scope<StepperEvent>) -> impl IntoElement {
    let range = computed(move || range(&props));
    // The app's value or our own, shown on the step's decimals and within
    // the bounds either way.
    let given = controllable(
        props.value,
        untracked(|| range.get().fit(props.default_value.get())),
    );
    let value = computed(move || range.get().fit(given.get()));
    // Stepping while a button is held.
    let repeat = press_repeat();
    // Shows `next` (unless the app controls the value) and reports it.
    let change = move |next: f64| {
        if next == value.get_untracked() {
            repeat.stop();
            return;
        }
        given.set(next);
        cx.emit(StepperEvent::Change(next));
    };
    // One step of `direction` (−1 or 1); stops repeating at a bound.
    let step = move |direction: f64| {
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
    cx.methods(StepperApi {
        value,
        range,
        change: Rc::new(change),
    });
    let button =
        move |label: &'static str, name: &'static str, direction: f64, at_bound: Memo<bool>| {
            let off = computed(move || props.disabled.get() || at_bound.get());
            view()
                .size_10()
                .items_center()
                .justify_center()
                .rounded(move || px(theme().radius))
                .bg(move || theme().secondary)
                .active(|s| s.bg(move || theme().secondary_active))
                .style_when(off, |v| v.opacity(DISABLED_OPACITY))
                .disabled(off)
                .role(AccessibilityRole::Button)
                .label(name)
                // One step now, more while it is held.
                .on(EventKind::PressIn, move |_: &EventPayload| {
                    repeat.start(move || step(direction));
                })
                .on(EventKind::PressOut, move |_: &EventPayload| repeat.stop())
                .child(
                    text(label)
                        .text_size(move || px(theme().font_size + 4.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().secondary_foreground),
                )
        };
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .child(button(
            "−",
            "Decrease",
            -1.0,
            computed(move || range.get().at_min(value.get())),
        ))
        .child(
            text(move || range.get().format(value.get()))
                .min_w(px(48.0))
                .text_center()
                .text_size(move || px(theme().font_size + 1.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || theme().foreground),
        )
        .child(button(
            "+",
            "Increase",
            1.0,
            computed(move || range.get().at_max(value.get())),
        ))
}
