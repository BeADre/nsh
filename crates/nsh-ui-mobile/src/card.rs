//! `Card`: a surface with an optional header and footer around its content.

use nsh_core::component::{
    Component, El, PropSpec, PropType, Props, RenderCx, SlotSpec, Spec, view,
};

/// A rounded surface: `header` (titles: its texts are semibold), the
/// children, `footer` (actions), separated only when both are there.
#[derive(Debug, Default)]
pub struct Card;

static SPEC: Spec = Spec {
    name: "nsh.card",
    export: "Card",
    doc: "A rounded surface with an optional header and footer around its content.",
    props: &[PropSpec {
        name: "variant",
        ty: PropType::Enum(&["elevated", "outlined"]),
        default: Some("\"elevated\""),
        doc: "`elevated` (default): a soft shadow; `outlined`: a border.",
    }],
    events: &[],
    children: true,
    slots: &[
        SlotSpec {
            name: "header",
            doc: "Above the content: a title, an avatar row.",
        },
        SlotSpec {
            name: "footer",
            doc: "Below the content, after a separator: actions.",
        },
    ],
};

impl Component for Card {
    fn spec() -> &'static Spec {
        &SPEC
    }

    fn render(&self, p: &Props, cx: &mut RenderCx) -> El<Self> {
        let surface = match &*p.str("variant") {
            "outlined" => "border border-zinc-200 dark:border-zinc-800",
            _ => "shadow-sm",
        };
        let header = cx.has_slot("header");
        view()
            .class("rounded-2xl bg-white dark:bg-zinc-900 text-zinc-900 dark:text-zinc-100")
            .class(surface)
            .children(header.then(|| {
                view()
                    .class("px-4 pt-4 text-[17px] font-semibold")
                    .slot("header")
            }))
            .child(
                view()
                    .class(if header { "px-4 pt-2 pb-4 gap-2" } else { "p-4 gap-2" })
                    .holds_children(),
            )
            .children(cx.has_slot("footer").then(|| {
                view()
                    .class("flex-row justify-end gap-2 px-4 py-3 border-t border-zinc-100 dark:border-zinc-800")
                    .slot("footer")
            }))
    }
}
