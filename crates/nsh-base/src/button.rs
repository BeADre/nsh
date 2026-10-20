//! `Button`: a press target with no look of its own.

use nsh_core::{AccessibilityState, ui::prelude::*};

/// A press target that owns its behavior (presses, the disabled and
/// selected states, its role for screen readers) and leaves its look to the
/// app: no colors, sizes or label of its own. The app's `class` and `style`
/// style it, its states with `active:`, `hover:`, `focus:` and `disabled:`,
/// and its children are its content.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct ButtonProps {
    /// No presses, no press feedback; `disabled:` styles apply and screen
    /// readers say it is dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
    /// Selected (a toggled trigger, the current tab): screen readers say so.
    /// The app owns the value and styles it.
    #[prop(default)]
    pub selected: Prop<bool>,
    /// What screen readers say, when its content is not enough (an icon).
    pub accessibility_label: Prop<Option<SharedString>>,
    /// The content.
    pub children: Prop<Children>,
}

/// The events of a [`Button`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum ButtonEvent {
    /// The button was pressed.
    Press,
    /// A touch went down on it (for the app's own press feedback).
    PressIn,
    /// The touch that went down on it ended or left.
    PressOut,
}

/// A press target ([`ButtonProps`]).
#[component]
pub fn Button(props: ButtonProps, cx: Scope<ButtonEvent>) -> impl IntoElement {
    view()
        .disabled(props.disabled)
        .role(AccessibilityRole::Button)
        .optional_label(props.accessibility_label)
        .accessibility_state(move || AccessibilityState {
            selected: props.selected.get(),
            ..AccessibilityState::default()
        })
        .on_press(move |_: &()| cx.emit(ButtonEvent::Press))
        .on_press_in(move |_: &()| cx.emit(ButtonEvent::PressIn))
        .on_press_out(move |_: &()| cx.emit(ButtonEvent::PressOut))
        .child(props.children)
}
