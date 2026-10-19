//! `TextField`: a text input with an optional label, for the pointer and
//! the keyboard.

use nsh_core::ui::prelude::*;

use crate::metrics::{ControlSize, FIELD_HOVER_OPACITY, RING_OFFSET, RING_WIDTH};

/// A text input with an optional label above it: its border darkens under
/// the pointer, and it is ringed while it has the keyboard focus (from Tab
/// or a click). With `value` the app controls the text (it updates `value`
/// from `onChange`); without, the field keeps its own, starting at
/// `defaultValue`.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct TextFieldProps {
    /// Above the input.
    pub label: Prop<Option<SharedString>>,
    /// Shown while it is empty.
    pub placeholder: Prop<Option<SharedString>>,
    /// The text, when the app controls it.
    pub value: Prop<Option<SharedString>>,
    /// The first text, when the app does not control it.
    #[prop(default)]
    pub default_value: Prop<SharedString>,
    /// Hides what is typed (passwords).
    #[prop(default)]
    pub secure: Prop<bool>,
    /// `sm`, `md` (default) or `lg`.
    #[prop(default)]
    pub size: Prop<ControlSize>,
    /// Not editable, dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
}

/// The events of a [`TextField`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum TextFieldEvent {
    /// The text changed (the new text).
    Change(String),
    /// Return was pressed (the text).
    Submit(String),
}

/// A text input with an optional label ([`TextFieldProps`]).
#[component]
pub fn TextField(props: TextFieldProps, cx: Scope<TextFieldEvent>) -> impl IntoElement {
    // The app's text or our own.
    let content = controllable(props.value, props.default_value.get_untracked());
    let size = props.size;
    let focused = signal(false);
    view()
        .gap_1()
        .style_when(props.disabled, |v| v.opacity(DISABLED_OPACITY))
        .child(dynamic(move || {
            props.label.get().map(|label| {
                text(label)
                    .text_size(move || px(size.get().font_size()))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(move || theme().muted_foreground)
            })
        }))
        .child(
            field_box(size, props.disabled, focused)
                .px(move || px(size.get().padding_x()))
                .child(
                    input()
                        .flex_1()
                        .text_size(move || px(size.get().font_size()))
                        .text_color(move || theme().foreground)
                        .value(move || content.with(|t| t.as_str().to_owned()))
                        .placeholder(move || {
                            props
                                .placeholder
                                .get()
                                .map_or_else(String::new, |p| p.as_str().to_owned())
                        })
                        .placeholder_color(move || color(theme().muted_foreground))
                        .selection_color(move || color(theme().primary))
                        .secure(props.secure)
                        .editable(move || !props.disabled.get())
                        // Return keeps the focus, as on desktop fields.
                        .submit_behavior(SubmitBehavior::Submit)
                        .on_change_text(move |text: &str| {
                            content.set(text.to_owned().into());
                            cx.emit(TextFieldEvent::Change(text.to_owned()));
                        })
                        .on_submit_editing(move |text: &str| {
                            cx.emit(TextFieldEvent::Submit(text.to_owned()));
                        })
                        .on_focus(move |_: &()| focused.set(true))
                        .on_blur(move |_: &()| focused.set(false)),
                ),
        )
}

/// The box a field's input goes in, at `size`'s height and corners: the
/// theme's `input` border, darker toward its `ring` under the pointer, and
/// the focus ring while `focused` (which the input sets on its focus and
/// blur). Inputs take focus from a click too, so the ring shows whenever
/// typing goes there, as browsers show `:focus-visible` on their inputs.
/// Shared by the set's fields.
pub(crate) fn field_box(
    size: Prop<ControlSize>,
    disabled: Prop<bool>,
    focused: Signal<bool>,
) -> View {
    view()
        .flex_row()
        .items_center()
        .h(move || px(size.get().height()))
        .rounded(move || px(size.get().radius()))
        .bg(move || theme().background)
        .border_1()
        .border_color(move || theme().input)
        .hover(|s| {
            s.border_color(move || {
                if disabled.get() {
                    theme().input
                } else {
                    theme().ring.opacity(FIELD_HOVER_OPACITY)
                }
            })
        })
        .style_when(
            move || focused.get() && !disabled.get(),
            |s| {
                s.outline(RING_WIDTH)
                    .outline_color(move || theme().ring)
                    .outline_offset(RING_OFFSET)
            },
        )
}
