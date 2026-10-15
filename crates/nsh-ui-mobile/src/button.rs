//! `Button`: a press target with a label.

use nsh_core::{
    AccessibilityRole, AccessibilityState, JsonValue,
    component::{
        Component, El, EventCx, EventSpec, PropSpec, PropType, Props, RenderCx, Spec, text, view,
    },
};

/// A button: `primary` (filled), `secondary` (tinted) or `plain` (text
/// only); native press feedback, dimmed when disabled.
#[derive(Debug, Default)]
pub struct Button;

static SPEC: Spec = Spec {
    name: "nsh.button",
    export: "Button",
    doc: "A press target with a label, the same on iOS and Android.",
    props: &[
        PropSpec {
            name: "title",
            ty: PropType::String,
            default: None,
            doc: "The label.",
        },
        PropSpec {
            name: "variant",
            ty: PropType::Enum(&["primary", "secondary", "plain"]),
            default: Some("\"primary\""),
            doc: "`primary` (default): filled; `secondary`: tinted; `plain`: text only.",
        },
        PropSpec {
            name: "disabled",
            ty: PropType::Bool,
            default: Some("false"),
            doc: "No presses, no feedback, dimmed.",
        },
        PropSpec {
            name: "accessibilityLabel",
            ty: PropType::String,
            default: Some("\"\""),
            doc: "For screen readers, when `title` is not enough.",
        },
    ],
    events: &[EventSpec {
        name: "press",
        data: "void",
        doc: "The button was pressed.",
    }],
    children: false,
};

const BASE: &str = "flex-row items-center justify-center rounded-xl px-4 py-3 disabled:opacity-40";

fn variant(name: &str) -> &'static str {
    match name {
        "secondary" => {
            "bg-blue-100 active:bg-blue-200 text-blue-700 dark:bg-blue-950 dark:active:bg-blue-900 dark:text-blue-300"
        }
        "plain" => "active:opacity-60 text-blue-600 dark:text-blue-400",
        _ => "bg-blue-600 active:bg-blue-700 text-white",
    }
}

impl Component for Button {
    fn spec() -> &'static Spec {
        &SPEC
    }

    fn render(&self, p: &Props, _cx: &mut RenderCx) -> El<Self> {
        let title = p.str("title");
        let label = p.str("accessibilityLabel");
        let disabled = p.bool("disabled");
        view()
            .class(BASE)
            .class(variant(&p.str("variant")))
            .disabled(disabled)
            .role(AccessibilityRole::Button)
            .label(if label.is_empty() {
                title.clone()
            } else {
                label
            })
            .state(AccessibilityState {
                disabled,
                ..AccessibilityState::default()
            })
            .on_press(|_: &mut Self, cx: &mut EventCx| cx.emit("press", JsonValue::Null))
            // The label inherits the button's text classes.
            .child(text(title).class("text-[17px] font-semibold"))
    }
}
