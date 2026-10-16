//! System: Status bar, alerts, action sheets, environment, timers (as the TSX gallery's `demos/System.tsx`).

use nsh_core::ui::prelude::*;

use crate::ui::{colors, page, section};

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct System;

impl Component for System {
    const NAME: &'static str = "gallery.system";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([section(
            "System",
            [text("Not ported yet.")
                .text_color(colors::muted())
                .into_element()],
        )
        .into_element()]))
    }
}
