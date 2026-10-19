//! The motion of NSH's design language: how surfaces move, the same in
//! every UI set (`nsh/ui/mobile`, a desktop set), as the theme's colors
//! and radii are ([`super::Theme`]). Components animate with these instead
//! of numbers of their own, so a sheet on a phone and a panel on a desktop
//! come to rest and leave the same way.

use super::{Easing, Spring, Timing};

/// A surface coming to rest where it belongs: a sheet sliding in, or
/// springing back after a drag (with the drag's velocity:
/// `Spring { velocity, ..motion::SETTLE }`). Quick, and just under
/// critically damped: it barely overshoots.
pub const SETTLE: Spring = Spring {
    stiffness: 280.0,
    damping: 30.0,
    mass: 1.0,
    velocity: None,
};

/// A surface leaving: a sheet sliding away. Eases in, speeding up as it
/// goes, and takes less time than coming in.
pub const EXIT: Timing = Timing::new(220.0, Easing::EaseIn);
