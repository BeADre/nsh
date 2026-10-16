//! Mobile UI: nsh/ui/mobile from Rust: a ListView and a Group laying out rows (as the TSX gallery's `demos/MobileUi.tsx`).

use nsh_core::ui::prelude::*;

use crate::ui::{colors, page, section};

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct MobileUi;

impl Component for MobileUi {
    const NAME: &'static str = "gallery.mobile_ui";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([section(
            "Mobile UI",
            [text("Not ported yet.")
                .text_color(colors::muted())
                .into_element()],
        )
        .into_element()]))
    }
}
