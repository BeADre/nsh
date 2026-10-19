//! `Dialog`: a surface in the middle of the window over a dimmed backdrop.

use nsh_core::ui::{prelude::*, presence};

use crate::metrics::surface_radius;

/// How far the surface rises as it comes in (and sinks as it leaves), in
/// points.
const RISE: f32 = 8.0;

/// A surface in the middle of the window over a dimmed backdrop while
/// `open`: a title, a description, the content and the actions below it. A
/// click on the backdrop or Escape closes it (`onClose`: set `open` to
/// false); it fades and sinks a little as it goes.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct DialogProps {
    /// Shown.
    #[prop(default)]
    pub open: Prop<bool>,
    /// At the top, in bold.
    pub title: Prop<Option<SharedString>>,
    /// Below the title, muted.
    pub description: Prop<Option<SharedString>>,
    /// Below the content: the actions, at the end (the main one last).
    pub footer: Prop<Option<Slot>>,
    /// The surface's width, in points (narrower windows narrow it).
    #[prop(default = 440.0)]
    pub width: Prop<f32>,
    /// The content: made when the dialog opens and released when it has
    /// closed, so it starts fresh each time.
    #[prop(lazy)]
    pub children: Children,
}

/// The events of a [`Dialog`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum DialogEvent {
    /// The user closed it: a click on the backdrop, or Escape.
    Close,
}

/// A surface in the middle of the window ([`DialogProps`]).
#[component]
pub fn Dialog(props: DialogProps, cx: Scope<DialogEvent>) -> impl IntoElement {
    // 0 hidden, 1 shown: the backdrop's and the surface's opacity, and how
    // far the surface has risen.
    let shown = native_value(0.0);
    let present = presence(props.open, {
        let shown = shown.clone();
        move || shown.timing_with(0.0, motion::EXIT)
    });
    {
        let shown = shown.clone();
        effect(move || {
            if props.open.get() {
                shown.spring(1.0, motion::SETTLE);
            }
        });
    }
    // Once: a click or Escape while it leaves asks nothing more.
    let close = move || {
        if props.open.get_untracked() {
            cx.emit(DialogEvent::Close);
        }
    };
    let children = props.children;
    view().child(If::new(present, move || {
        let rise = shown.interpolate(&[0.0, 1.0], &[RISE, 0.0]);
        modal()
            .transparent(true)
            .animation(ModalAnimation::None)
            .on_request_close(move |_: &()| close())
            .child(
                // The dimmed window around it; a click on it closes it.
                view()
                    .absolute()
                    .inset_0()
                    .bg(move || theme().overlay)
                    .bind_opacity(&shown)
                    .label("Close")
                    .focusable(false)
                    .on_press(move |_: &()| close()),
            )
            .child(
                view()
                    .absolute()
                    .inset_0()
                    .items_center()
                    .justify_center()
                    .p_6()
                    .pointer_events(PointerEvents::BoxNone)
                    .child(
                        view()
                            .w(move || px(props.width.get()))
                            .max_w(relative(1.0))
                            .gap_4()
                            .px_6()
                            .py_5()
                            .rounded(move || px(surface_radius()))
                            .bg(move || theme().background)
                            .text_color(move || theme().foreground)
                            .border_1()
                            .border_color(move || theme().border)
                            .shadow_lg()
                            .bind_opacity(&shown)
                            .bind_translate_y(&rise)
                            .child(
                                view()
                                    .gap_1()
                                    .child(dynamic(move || {
                                        props.title.get().map(|title| {
                                            text(title)
                                                .text_size(move || px(theme().font_size))
                                                .font_weight(FontWeight::SEMIBOLD)
                                        })
                                    }))
                                    .child(dynamic(move || {
                                        props.description.get().map(|description| {
                                            text(description)
                                                .text_size(move || px(theme().font_size - 2.0))
                                                .text_color(move || theme().muted_foreground)
                                        })
                                    })),
                            )
                            .child(children)
                            .child(
                                props
                                    .footer
                                    .place(|footer| footer.flex_row().justify_end().gap_2().pt_2()),
                            ),
                    ),
            )
    }))
}
