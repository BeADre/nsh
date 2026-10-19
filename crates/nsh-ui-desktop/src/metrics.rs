//! The desktop set's density: NSH's design language (the theme's colors,
//! radii and font size, `ui::motion`, `ui::DISABLED_OPACITY`) at the sizes
//! a pointer and a keyboard call for.
//!
//! Controls are shorter than the mobile set's, whose controls are 44 points
//! tall for a finger; their corners scale with their height from there, so
//! a desktop control has a phone control's shape.

use nsh_core::ui::prelude::*;

/// The height of the mobile set's controls, which the theme's `radius` is
/// drawn for.
const MOBILE_HEIGHT: f32 = 44.0;

/// How large a control is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PropEnum)]
pub enum ControlSize {
    /// 24 points tall: toolbars, dense tables.
    Sm,
    /// 32 points tall: most controls.
    #[default]
    Md,
    /// 40 points tall: the main action of a window.
    Lg,
}

impl ControlSize {
    /// The control's height, in points.
    pub fn height(self) -> f32 {
        match self {
            Self::Sm => 24.0,
            Self::Md => 32.0,
            Self::Lg => 40.0,
        }
    }

    /// The padding at the control's start and end, in points.
    pub fn padding_x(self) -> f32 {
        match self {
            Self::Sm => 8.0,
            Self::Md => 12.0,
            Self::Lg => 16.0,
        }
    }

    /// The size of the control's text: the theme's font size, smaller for
    /// the denser sizes (reading subscribes to the theme).
    pub fn font_size(self) -> f32 {
        self.font_size_from(theme().font_size)
    }

    /// The control's corner radius: the theme's, scaled from the mobile
    /// set's control height to this one (reading subscribes to the theme).
    pub fn radius(self) -> f32 {
        self.radius_from(theme().radius)
    }

    /// The text size for a theme font size of `base`.
    fn font_size_from(self, base: f32) -> f32 {
        match self {
            Self::Sm => base - 3.0,
            Self::Md => base - 2.0,
            Self::Lg => base,
        }
    }

    /// The corner radius for a theme radius of `radius`.
    fn radius_from(self, radius: f32) -> f32 {
        radius * self.height() / MOBILE_HEIGHT
    }
}

/// The corner radius of a floating surface (a popover, a dialog): the
/// theme's radius of large surfaces, scaled as controls are (reading
/// subscribes to the theme).
pub(crate) fn surface_radius() -> f32 {
    theme().radius_lg * ControlSize::Md.height() / MOBILE_HEIGHT
}

/// How far a hovered control's background fades toward what is under it.
pub(crate) const HOVER_OPACITY: f32 = 0.9;

/// How strongly a hovered field's border takes the theme's `ring` color,
/// short of the focus ring's full color.
pub(crate) const FIELD_HOVER_OPACITY: f32 = 0.5;

/// The focus ring's width and its gap from the control, in points.
pub(crate) const RING_WIDTH: f32 = 2.0;
pub(crate) const RING_OFFSET: f32 = 2.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// Controls keep a phone control's shape at desktop sizes, and each size
    /// is denser than the next.
    #[test]
    fn sizes_scale_from_the_mobile_set() {
        let theme = Theme::default();
        let md = ControlSize::Md;
        assert_eq!(
            md.radius_from(theme.radius) / md.height(),
            theme.radius / 44.0
        );
        assert_eq!(md.font_size_from(theme.font_size), 14.0);
        assert!(ControlSize::Sm.height() < md.height());
        assert!(md.height() < ControlSize::Lg.height());
        assert!(ControlSize::Sm.font_size_from(16.0) < md.font_size_from(16.0));
    }
}
