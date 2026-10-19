//! `Popover`: a floating surface next to what opened it.

use nsh_core::{
    Rect, Size,
    ui::{Align, Side, place, prelude::*},
};

/// How far the surface stands off its trigger, and keeps from the window's
/// edges, in points.
const GAP: f32 = 6.0;
const MARGIN: f32 = 8.0;

/// A floating surface next to its `trigger` while `open`: below it by
/// default, flipped above (or to the other side) when the window has no
/// room there, and kept inside the window. A press outside it or Escape
/// closes it (`onClose`: set `open` to false).
#[derive(Debug, Clone, PartialEq, Props)]
pub struct PopoverProps {
    /// Shown.
    #[prop(default)]
    pub open: Prop<bool>,
    /// What opens it (a button): the surface goes next to it.
    pub trigger: Prop<Option<Slot>>,
    /// Which side of the trigger: `bottom` (default), `top`, `left` or
    /// `right`.
    #[prop(default)]
    pub side: Prop<Side>,
    /// How it lines up with the trigger: `start` (default), `center` or
    /// `end`.
    #[prop(default)]
    pub align: Prop<Align>,
    /// The content: made when the popover opens and released when it
    /// closes, so it starts fresh each time.
    #[prop(lazy)]
    pub children: Children,
}

/// The events of a [`Popover`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum PopoverEvent {
    /// The user closed it: a press outside it, or Escape.
    Close,
}

/// A floating surface next to its trigger ([`PopoverProps`]).
#[component]
pub fn Popover(props: PopoverProps, cx: Scope<PopoverEvent>) -> impl IntoElement {
    let trigger = element_ref();
    // Where the trigger is, measured when the popover opens (and again
    // when the window changes); the surface's size, from its layout.
    let anchor = signal(None::<Rect>);
    let size = signal(None::<Size>);
    {
        let trigger = trigger.clone();
        effect(move || {
            let _ = window();
            if !props.open.get() {
                anchor.set(None);
                size.set(None);
                return;
            }
            let trigger = trigger.clone();
            spawn(async move {
                if let Some(m) = trigger.measure().await {
                    anchor.set(Some(Rect {
                        x: m.page_x,
                        y: m.page_y,
                        width: m.frame.width,
                        height: m.frame.height,
                    }));
                }
            });
        });
    }
    // Where the surface goes, once both are known.
    let placed = computed(move || {
        let (anchor, size) = (anchor.get()?, size.get()?);
        let (window, insets) = (window(), safe_area());
        let bounds = Rect {
            x: insets.left + MARGIN,
            y: insets.top + MARGIN,
            width: window.width - insets.left - insets.right - 2.0 * MARGIN,
            height: window.height - insets.top - insets.bottom - 2.0 * MARGIN,
        };
        Some(place(
            anchor,
            size,
            props.side.get(),
            props.align.get(),
            GAP,
            bounds,
        ))
    });
    let close = move || cx.emit(PopoverEvent::Close);
    let children = props.children;
    view()
        .items_start()
        .child(props.trigger.place({
            let trigger = trigger.clone();
            move |slot| slot.element_ref(&trigger)
        }))
        .child(If::new(props.open, move || {
            // Fades in once it is placed: measured, it shows where it
            // belongs.
            let shown = native_value(0.0);
            {
                let shown = shown.clone();
                effect(move || {
                    if placed.with(Option::is_some) {
                        shown.spring(1.0, motion::SETTLE);
                    }
                });
            }
            modal()
                .transparent(true)
                .animation(ModalAnimation::None)
                .on_request_close(move |_: &()| close())
                .child(
                    // A press anywhere else closes it; the keyboard goes
                    // straight to the content (Escape closes it).
                    view()
                        .absolute()
                        .inset_0()
                        .label("Close")
                        .focusable(false)
                        .on_press(move |_: &()| close()),
                )
                .child(
                    view()
                        .absolute()
                        .left(move || px(placed.get().map_or(0.0, |p| p.x)))
                        .top(move || px(placed.get().map_or(0.0, |p| p.y)))
                        .bind_opacity(&shown)
                        .on_layout(move |frame: &Rect| {
                            size.set(Some(Size {
                                width: frame.width,
                                height: frame.height,
                            }));
                        })
                        .p_1()
                        .rounded(move || px(theme().radius))
                        .bg(move || theme().popover)
                        .text_color(move || theme().popover_foreground)
                        .border_1()
                        .border_color(move || theme().border)
                        .shadow_lg()
                        .child(children),
                )
        }))
}
