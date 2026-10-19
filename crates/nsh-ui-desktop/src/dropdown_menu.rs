//! `DropdownMenu`: a list of actions in a popover under its trigger.

use nsh_core::ui::{Align, Side, prelude::*};

use crate::{
    kbd::shortcut_label,
    metrics::{ControlSize, label_font_size},
    popover::floating,
};

/// What an entry of a [`DropdownMenu`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PropEnum)]
pub enum MenuEntryKind {
    /// A line between groups of items.
    Separator,
}

/// An entry of a [`DropdownMenu`]: an item, or `{ type: "separator" }`
/// (the items of `nsh/desktop`'s `Menu.popup`, picked by index).
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct MenuEntry {
    /// `"separator"`: a line instead of an item.
    #[prop(rename = "type")]
    pub kind: Option<MenuEntryKind>,
    /// What the item shows.
    pub label: Option<SharedString>,
    /// Its shortcut, shown at its end as menus write it here (`"mod+r"`
    /// reads `⌘R` on macOS): the keystrokes of `nsh/keymap`, which binds
    /// them.
    pub shortcut: Option<SharedString>,
    /// It can be chosen (default true).
    pub enabled: Option<bool>,
    /// A destructive action, in the theme's danger color.
    pub danger: Option<bool>,
    /// A check mark: set (true or false) for a checkable item.
    pub checked: Option<bool>,
}

/// A list of actions in a popover next to its `trigger` while `open`:
/// choosing one (a click, or Return or Space with the keyboard focus)
/// reports `onSelect` with its index and asks to close; a press outside it
/// or Escape asks to close (`onClose`: set `open` to false).
#[derive(Debug, Clone, PartialEq, Props)]
pub struct DropdownMenuProps {
    /// Shown.
    #[prop(default)]
    pub open: Prop<bool>,
    /// What opens it (a button): the menu goes under it.
    pub trigger: Prop<Option<Slot>>,
    /// The entries.
    pub items: Prop<Vec<MenuEntry>>,
    /// `sm`, `md` (default) or `lg`: the items' height and text size.
    #[prop(default)]
    pub size: Prop<ControlSize>,
}

/// The events of a [`DropdownMenu`].
#[derive(Debug, Clone, PartialEq, Event)]
pub enum DropdownMenuEvent {
    /// An item was chosen (its index in `items`); `onClose` follows.
    Select(u32),
    /// The menu asks to close: an item was chosen, or a press outside it or
    /// Escape.
    Close,
}

/// A list of actions under a trigger ([`DropdownMenuProps`]).
#[component]
pub fn DropdownMenu(props: DropdownMenuProps, cx: Scope<DropdownMenuEvent>) -> impl IntoElement {
    let size = props.size;
    let items = props.items;
    // A check column for all items once one is checkable, so labels line
    // up.
    let checks = computed(move || items.with(|i| i.iter().any(|e| e.checked.is_some())));
    let entries = move || {
        view()
            .role(AccessibilityRole::Menu)
            .min_w(px(180.0))
            .child(dynamic(move || {
                items.with(|all| {
                    all.iter()
                        .enumerate()
                        .map(|(i, entry)| entry_row(entry, i as u32, size, checks, cx))
                        .collect::<Vec<_>>()
                })
            }))
            .into_element()
    };
    floating(
        props.open,
        Side::Bottom.into(),
        Align::Start.into(),
        |trigger| props.trigger.place(move |slot| slot.element_ref(&trigger)),
        entries,
        move || cx.emit(DropdownMenuEvent::Close),
    )
}

/// The row of `entry`, the `index`th.
fn entry_row(
    entry: &MenuEntry,
    index: u32,
    size: Prop<ControlSize>,
    checks: Memo<bool>,
    cx: Scope<DropdownMenuEvent>,
) -> AnyElement {
    if entry.kind == Some(MenuEntryKind::Separator) {
        return view()
            .h(px(1.0))
            .my_1()
            .mx_1()
            .bg(move || theme().border)
            .into_element();
    }
    let enabled = entry.enabled.unwrap_or(true);
    let danger = entry.danger.unwrap_or(false);
    let label = entry
        .label
        .as_ref()
        .map_or_else(String::new, |l| l.as_str().to_owned());
    let shortcut = entry.shortcut.as_ref().map(|s| shortcut_label(s));
    let checked = entry.checked;
    let color = move || {
        if danger {
            theme().danger
        } else {
            theme().popover_foreground
        }
    };
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .h(move || px(size.get().height()))
        .px(move || px(size.get().padding_x()))
        .rounded(move || px(size.get().radius()))
        .hover(|s| s.bg(move || theme().accent))
        .active(|s| s.bg(move || theme().secondary_active))
        .focus(|s| s.bg(move || theme().accent))
        .style_when(!enabled, |v| v.opacity(DISABLED_OPACITY))
        .disabled(!enabled)
        .role(AccessibilityRole::Menuitem)
        .label(label.clone())
        .accessibility_state(AccessibilityState {
            disabled: !enabled,
            checked: checked.map(|c| {
                if c {
                    nsh_core::Checked::True
                } else {
                    nsh_core::Checked::False
                }
            }),
            ..Default::default()
        })
        .on_press(move |_: &()| {
            cx.emit(DropdownMenuEvent::Select(index));
            cx.emit(DropdownMenuEvent::Close);
        })
        .child(If::new(checks, move || {
            text(if checked == Some(true) { "✓" } else { "" })
                .w(move || px(size.get().font_size()))
                .text_size(move || px(size.get().font_size()))
                .text_color(color)
        }))
        .child(
            text(label)
                .flex_grow(1.0)
                .number_of_lines(1)
                .text_size(move || px(size.get().font_size()))
                .text_color(color),
        )
        .when_some(shortcut, |row, shortcut| {
            row.child(
                text(shortcut)
                    .pl_4()
                    .text_size(move || px(label_font_size() + 1.0))
                    .text_color(move || theme().muted_foreground),
            )
        })
        .into_element()
}
