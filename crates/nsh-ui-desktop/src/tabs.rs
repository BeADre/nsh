//! `Tabs`: a row of tabs, one of them selected, with a mark that moves to
//! the one chosen.

use nsh_core::{
    Rect, TransformKind,
    ui::{Length, SharedString, prelude::*},
};

use crate::metrics::{ControlSize, RING_OFFSET, RING_WIDTH};

/// The gap between a segmented track's edge and its tabs, in points.
const TRACK_PADDING: f32 = 3.0;
/// The underline's thickness, in points.
const UNDERLINE: f32 = 2.0;

/// A row of tabs, one of them selected: an underline under it, or a raised
/// background in a segmented track, that moves to the tab chosen. Each tab
/// is pressed with a click, or with Return or Space after Tab focuses it.
/// With `value` the app controls the selection (it updates `value` from
/// `onChange`); without, the tabs keep their own, starting at
/// `defaultValue`.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct TabsProps {
    /// The tabs' labels.
    pub options: Prop<Vec<SharedString>>,
    /// The index of the selected tab, when the app controls it.
    pub value: Prop<Option<u32>>,
    /// The index of the first selected tab, when the app does not control
    /// it.
    #[prop(default)]
    pub default_value: Prop<u32>,
    /// `underline` (default): a line under the selected tab; `segmented`:
    /// tabs in a track, the selected one raised.
    #[prop(default)]
    pub variant: Prop<TabsVariant>,
    /// `sm`, `md` (default) or `lg`.
    #[prop(default)]
    pub size: Prop<ControlSize>,
    /// No presses, dimmed.
    #[prop(default)]
    pub disabled: Prop<bool>,
}

/// How [`Tabs`] look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PropEnum)]
pub enum TabsVariant {
    /// A line under the selected tab, over a divider under the row.
    #[default]
    Underline,
    /// Tabs in a track, the selected one on a raised background.
    Segmented,
}

/// The events of [`Tabs`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum TabsEvent {
    /// Another tab was chosen (its index).
    Change(u32),
}

/// A tab's place in the row: its left edge and width.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Span {
    x: f32,
    width: f32,
}

/// What every tab of a row reads and reports.
#[derive(Clone, Copy)]
struct Row {
    selected: Memo<u32>,
    segmented: Memo<bool>,
    size: Prop<ControlSize>,
    disabled: Prop<bool>,
    /// Where each tab is, from its layout.
    spans: Signal<Vec<Option<Span>>>,
    /// How wide each label is, from its layout.
    labels: Signal<Vec<f32>>,
    /// The widest label.
    widest: Memo<f32>,
}

/// Sets entry `i` of `list` to `value`, growing it as needed.
fn put<T: Clone + Default + 'static>(list: Signal<Vec<T>>, i: usize, value: T) {
    list.update(|list| {
        if list.len() <= i {
            list.resize(i + 1, T::default());
        }
        list[i] = value;
    });
}

/// A row of tabs ([`TabsProps`]).
#[component]
pub fn Tabs(props: TabsProps, cx: Scope<TabsEvent>) -> impl IntoElement {
    let size = props.size;
    let segmented = computed(move || props.variant.get() == TabsVariant::Segmented);
    let count = computed(move || props.options.with(Vec::len));
    // The app's selection or our own, kept on a tab that exists.
    let given = controllable(props.value, props.default_value.get_untracked());
    let selected = computed(move || given.get().min(count.get().saturating_sub(1) as u32));
    let spans = signal(Vec::<Option<Span>>::new());
    let labels = signal(Vec::<f32>::new());
    let row = Row {
        selected,
        segmented,
        size,
        disabled: props.disabled,
        spans,
        labels,
        widest: computed(move || labels.with(|l| l.iter().copied().fold(0.0, f32::max))),
    };
    let span = computed(move || spans.with(|s| s.get(selected.get() as usize).copied().flatten()));
    // The mark's offset and, for the underline (one point wide), its
    // stretch: both spring to a newly selected tab, so it slides and takes
    // the tab's width as it goes. Where the tab itself moves (the first
    // layout, a resize), the mark jumps with it; hidden until then.
    let offset = native_value(0.0);
    let stretch = native_value(0.0);
    let marked = stored(None::<u32>);
    {
        let (offset, stretch) = (offset.clone(), stretch.clone());
        effect(move || {
            let Some(Span { x, width }) = span.get() else {
                return;
            };
            // The underline scales about its center; the segmented mark
            // has the tab's width already.
            let (to_x, to_stretch) = if segmented.get() {
                (x, 1.0)
            } else {
                (x + (width - 1.0) / 2.0, width)
            };
            let index = selected.get_untracked();
            if marked
                .replace(Some(index))
                .is_some_and(|before| before != index)
            {
                offset.spring(to_x, motion::SETTLE);
                stretch.spring(to_stretch, motion::SETTLE);
            } else {
                offset.set(to_x);
                stretch.set(to_stretch);
            }
        });
    }
    let choose = move |index: u32| {
        if index != selected.get_untracked() {
            given.set(index);
            cx.emit(TabsEvent::Change(index));
        }
    };
    let tabs = dynamic(move || {
        props
            .options
            .get()
            .into_iter()
            .enumerate()
            .map(|(i, label)| tab(row, i, label, choose))
            .collect::<Vec<_>>()
    });
    let pill = offset.clone();
    view()
        .flex_row()
        .items_center()
        // A segmented track is as wide as its tabs; an underlined row
        // spans its container, with the divider under it.
        .style_when(segmented, |v| v.self_start())
        .p(move || px(if segmented.get() { TRACK_PADDING } else { 0.0 }))
        .rounded(move || {
            px(if segmented.get() {
                size.get().radius()
            } else {
                0.0
            })
        })
        .bg(move || {
            if segmented.get() {
                theme().tab_bar_segmented
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .border_b(move || if segmented.get() { 0.0_f32 } else { 1.0 })
        .border_color(move || theme().border)
        .style_when(props.disabled, |v| v.opacity(DISABLED_OPACITY))
        .disabled(props.disabled)
        .role(AccessibilityRole::Tablist)
        .child(
            If::new(segmented, move || {
                view()
                    .absolute()
                    .left(px(0.0))
                    .top(px(TRACK_PADDING))
                    .bottom(px(TRACK_PADDING))
                    .w(move || px(span.get().map_or(0.0, |s| s.width)))
                    .rounded(move || px((size.get().radius() - TRACK_PADDING).max(0.0)))
                    .bg(move || theme().tab_active)
                    .shadow_sm()
                    .pointer_events(PointerEvents::None)
                    .bind_translate_x(&pill)
            })
            .otherwise(move || {
                view()
                    .absolute()
                    .left(px(0.0))
                    // Over the divider.
                    .bottom(px(-1.0))
                    .w(px(1.0))
                    .h(px(UNDERLINE))
                    .bg(move || theme().primary)
                    .pointer_events(PointerEvents::None)
                    .bind_translate_x(&offset)
                    .bind_transform(TransformKind::ScaleX, &stretch)
            }),
        )
        .child(tabs)
}

/// Tab `i` of `row`, labelled `label`.
fn tab(
    row: Row,
    i: usize,
    label: SharedString,
    choose: impl Fn(u32) + Copy + 'static,
) -> impl IntoElement {
    let Row {
        selected,
        segmented,
        size,
        ..
    } = row;
    let index = i as u32;
    let on = computed(move || selected.get() == index);
    let height = move || {
        let height = size.get().height();
        if segmented.get() {
            height - 2.0 * TRACK_PADDING
        } else {
            height
        }
    };
    // Segments are all as wide as the widest label needs, once the labels
    // are measured: the basis shares a stretched track evenly, the minimum
    // sizes a track that fits its tabs (which sums the tabs' minimums,
    // not their bases).
    let even = move || {
        let widest = row.widest.get();
        (segmented.get() && widest > 0.0).then(|| px(widest + 2.0 * size.get().padding_x()))
    };
    view()
        .group()
        .flex_grow(move || if segmented.get() { 1.0_f32 } else { 0.0 })
        .basis(move || even().unwrap_or(Length::Auto))
        .min_w(move || even().unwrap_or(Length::Auto))
        .flex_row()
        .items_center()
        .justify_center()
        .h(move || px(height()))
        .px(move || px(size.get().padding_x()))
        .rounded(move || {
            let radius = size.get().radius();
            px(if segmented.get() {
                (radius - TRACK_PADDING).max(0.0)
            } else {
                radius
            })
        })
        // The selected tab shows its mark; the others highlight under the
        // pointer.
        .hover(|s| {
            s.bg(move || {
                if on.get() {
                    AdaptiveColor::TRANSPARENT
                } else {
                    theme().accent
                }
            })
        })
        .focus(|s| {
            s.outline(RING_WIDTH)
                .outline_color(move || theme().ring)
                .outline_offset(RING_OFFSET)
        })
        .disabled(row.disabled)
        .role(AccessibilityRole::Tab)
        .label(label.clone())
        .accessibility_state(move || AccessibilityState {
            selected: on.get(),
            ..AccessibilityState::default()
        })
        .on_layout(move |frame: &Rect| {
            let span = Span {
                x: frame.x,
                width: frame.width,
            };
            put(row.spans, i, Some(span));
        })
        .on_press(move |_: &()| choose(index))
        .child(
            text(label)
                .number_of_lines(1)
                // Its own width, whatever the tab's: what segments size by.
                .flex_shrink_0()
                .on_layout(move |frame: &Rect| put(row.labels, i, frame.width))
                .text_size(move || px(size.get().font_size()))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || {
                    if on.get() {
                        theme().tab_active_foreground
                    } else {
                        theme().tab_foreground
                    }
                })
                .group_hover(|s| s.text_color(move || theme().tab_active_foreground)),
        )
}
