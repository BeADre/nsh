//! `ListView`: a scrolling list of the app's rows, recycled natively.

use nsh_core::ui::prelude::*;

/// A scrolling list of rows ([`ListViewProps`]).
#[derive(Debug, Default)]
pub struct ListView;

/// A scrolling list of the app's rows, padded and divided: only the rows
/// near the screen exist, cells are reused. The app passes the data and
/// renders a row (`each` and children, like `List`); the look is the
/// list's.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct ListViewProps {
    /// The rows.
    pub items: Items,
    /// Hairlines between rows.
    #[prop(default = true)]
    pub dividers: bool,
}

impl Component for ListView {
    const NAME: &'static str = "nsh.list_view";
    type Props = ListViewProps;
    type Event = NoEvent;

    fn render(&self, props: &ListViewProps, cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (border, dividers) = (theme.border, props.dividers);
        view()
            .flex_1()
            .bg(theme.background)
            .text_color(theme.foreground)
            .list(
                props.items,
                |list| list.flex_1().estimated_item_size(48.0),
                move |row, index| {
                    view()
                        .px_4()
                        .py_3()
                        .min_h(px(48.0))
                        .justify_center()
                        .when(dividers && index > 0, |v| {
                            v.border_t_1().border_color(border)
                        })
                        .child(row)
                },
            )
    }
}
