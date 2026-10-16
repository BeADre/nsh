//! Form: Inputs, keyboard types, the keyboard over the content (as the TSX gallery's `demos/Form.tsx`).

use nsh_core::ui::prelude::*;

use crate::ui::{colors, page, section};

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct Form;

impl Component for Form {
    const NAME: &'static str = "gallery.form";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([section(
            "Form",
            [text("Not ported yet.")
                .text_color(colors::muted())
                .into_element()],
        )
        .into_element()]))
    }
}
