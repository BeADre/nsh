//! `StatusBar`: what the status bar shows while it renders.

use nsh_core::{
    platform::Os,
    ui::{StatusBarSetting, StatusBarStyle, prelude::*},
};

/// What the status bar shows while this renders: the most recently
/// mounted `StatusBar` wins, prop by prop. In an app embedded in a native
/// one, the host app's view controller decides on iOS.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct StatusBarProps {
    /// The color of the bar's content: `"light"` for dark backgrounds,
    /// `"dark"` for light ones, `"auto"` the platform's for the appearance.
    pub foreground: Prop<Option<Foreground>>,
    /// Hidden.
    pub hidden: Prop<Option<bool>>,
    /// Animate changes of `foreground` and `hidden`.
    pub animated: Prop<Option<bool>>,
    /// What only the web has.
    pub web: Prop<Option<StatusBarWeb>>,
}

/// The props of a [`StatusBar`] only the web has.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct StatusBarWeb {
    /// The color the browser gives its own bars around the page (the
    /// page's `<meta name="theme-color">`).
    pub theme_color: Option<Color>,
}

/// The color of the status bar's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PropEnum)]
pub enum Foreground {
    /// The platform's for the appearance.
    Auto,
    /// Light content, for dark backgrounds.
    Light,
    /// Dark content, for light backgrounds.
    Dark,
}

impl From<Foreground> for StatusBarStyle {
    fn from(foreground: Foreground) -> Self {
        match foreground {
            Foreground::Auto => StatusBarStyle::Default,
            Foreground::Light => StatusBarStyle::Light,
            Foreground::Dark => StatusBarStyle::Dark,
        }
    }
}

/// What `props` set (reading subscribes): the background is the
/// browser's theme color on the web.
fn setting(props: &StatusBarProps) -> StatusBarSetting {
    let background = if os() == Os::Web {
        props.web.get().and_then(|w| w.theme_color)
    } else {
        None
    };
    StatusBarSetting {
        style: props.foreground.get().map(Into::into),
        hidden: props.hidden.get(),
        animated: props.animated.get(),
        background,
    }
}

/// A rendered status bar setting ([`StatusBarProps`]): over the others while
/// it is mounted.
#[component]
pub fn StatusBar(props: StatusBarProps) -> impl IntoElement {
    let entry = add_status_bar(untracked(|| setting(&props)));
    effect(move || update_status_bar(entry, setting(&props)));
    // Nothing to lay out.
    view().hidden()
}
