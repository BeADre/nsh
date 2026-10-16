//! Photos: A long recycled list: images, pull to refresh, paging (as the TSX gallery's `demos/Photos.tsx`).

use nsh_core::ui::prelude::*;

use crate::ui::{colors, page, section};

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct Photos;

impl Component for Photos {
    const NAME: &'static str = "gallery.photos";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([section(
            "Photos",
            [text("Not ported yet.")
                .text_color(colors::muted())
                .into_element()],
        )
        .into_element()]))
    }
}
