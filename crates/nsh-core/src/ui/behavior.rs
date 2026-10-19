//! What controls do, apart from how they look: shared by every UI set, so
//! a mobile and a desktop control of the same kind behave alike while each
//! draws itself for its form factor.
//!
//! [`controllable`] is a value the app may control or leave to the
//! component, [`NumberRange`] the bounds and step of a number changed in
//! steps, [`press_repeat`] an action repeated while a press is held. They
//! hold no elements and no styles; a component makes them in its body,
//! where they go with it.

use std::{rc::Rc, time::Duration};

use super::{
    ambient::{Spawned, spawn, timer},
    reactive::{IntoReactive, Memo, Reactive, Signal, Stored, computed, signal, stored, untracked},
};

/// A value the app may control (a text field's text, a stepper's value):
/// while the app passes one (`Some`) it is shown, and the app updates it
/// from the component's event; otherwise the component keeps its own,
/// starting at a default. A `Copy` handle of the current owner
/// ([`controllable`]).
pub struct Controllable<T: 'static> {
    value: Stored<Reactive<Option<T>>>,
    own: Signal<T>,
    shown: Memo<T>,
}

impl<T> Clone for Controllable<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Controllable<T> {}

impl<T> std::fmt::Debug for Controllable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Controllable")
            .field("own", &self.own)
            .field("shown", &self.shown)
            .finish_non_exhaustive()
    }
}

/// A value shown from `value` while it is `Some` (the app controls it),
/// from the component's own otherwise, which starts at `default`. `value`
/// is usually the component's prop (`props.value`); a closure adapts
/// another shape (`move || props.value.get().map(Some)` for a choice that
/// may be none).
pub fn controllable<T: Clone + PartialEq + 'static>(
    value: impl IntoReactive<Option<T>>,
    default: T,
) -> Controllable<T> {
    let value = value.into_reactive();
    let own = signal(default);
    let shown = {
        let value = value.clone();
        computed(move || value.get().unwrap_or_else(|| own.get()))
    };
    Controllable {
        value: stored(value),
        own,
        shown,
    }
}

impl<T: Clone + PartialEq + 'static> Controllable<T> {
    /// The value shown (reading subscribes).
    pub fn get(&self) -> T {
        self.shown.get()
    }

    /// `f` of the value shown (reading subscribes).
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.shown.with(f)
    }

    /// The value shown, subscribing nothing (in listeners and tasks).
    pub fn get_untracked(&self) -> T {
        self.shown.get_untracked()
    }

    /// Whether the app controls the value (reading subscribes).
    pub fn is_controlled(&self) -> bool {
        let value = self.value.with(|v| v.clone());
        value.get().is_some()
    }

    /// The value as the user changed it: kept when the component keeps its
    /// own, left to the app when it controls the value (it passes the new
    /// one back after the component's event, or keeps the old). Emitting
    /// the event is the component's.
    pub fn set(&self, value: T) {
        if !untracked(|| self.is_controlled()) {
            self.own.set(value);
        }
    }

    /// The value shown as a memo, to pass on.
    pub fn memo(&self) -> Memo<T> {
        self.shown
    }
}

impl<T: Clone + PartialEq + 'static> IntoReactive<T> for Controllable<T> {
    fn into_reactive(self) -> Reactive<T> {
        self.shown.into_reactive()
    }
}

/// The bounds and the step of a number changed in steps (a stepper, a
/// slider, a number field). Values are rounded to the step's decimals (a
/// step of 0.25 keeps two) and kept within the bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberRange {
    min: f64,
    max: f64,
    step: f64,
    decimals: usize,
}

impl NumberRange {
    /// From bounds given in either order and a step whose sign does not
    /// matter.
    pub fn new(min: f64, max: f64, step: f64) -> Self {
        let step = step.abs();
        Self {
            min: min.min(max),
            max: min.max(max),
            step,
            decimals: decimals(step),
        }
    }

    /// The smallest value.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// The largest value.
    pub fn max(&self) -> f64 {
        self.max
    }

    /// How much a step changes the value (not negative).
    pub fn step(&self) -> f64 {
        self.step
    }

    /// The decimals of the step (0.25 has 2): what values are rounded to
    /// and shown with.
    pub fn decimals(&self) -> usize {
        self.decimals
    }

    /// `value` rounded to the step's decimals, within the bounds.
    pub fn fit(&self, value: f64) -> f64 {
        let scale = 10f64.powi(self.decimals as i32);
        ((value * scale).round() / scale).clamp(self.min, self.max)
    }

    /// `value` moved by `steps` steps (negative: down), fitted.
    pub fn step_by(&self, value: f64, steps: f64) -> f64 {
        self.fit(value + steps * self.step)
    }

    /// Whether `value` is at the smallest value (or below it).
    pub fn at_min(&self, value: f64) -> bool {
        value <= self.min
    }

    /// Whether `value` is at the largest value (or above it).
    pub fn at_max(&self, value: f64) -> bool {
        value >= self.max
    }

    /// `value` written with the step's decimals (`1.50` for a step of
    /// 0.25).
    pub fn format(&self, value: f64) -> String {
        format!("{value:.*}", self.decimals)
    }
}

/// The decimals of `step` as written (0.25 has 2).
fn decimals(step: f64) -> usize {
    let text = format!("{step}");
    text.split_once('.').map_or(0, |(_, d)| d.len())
}

/// An action repeated while a press is held: once when the press starts,
/// again after [`PressRepeat::HOLD`], then every
/// [`PressRepeat::INTERVAL`] until it is stopped (a stepper's buttons, a
/// scroll bar's arrows). A `Copy` handle of the current owner
/// ([`press_repeat`]); the repeating stops with it.
#[derive(Clone, Copy)]
pub struct PressRepeat {
    task: Stored<Option<Spawned<()>>>,
}

impl std::fmt::Debug for PressRepeat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let running = self.task.with(|t| t.is_some());
        f.debug_struct("PressRepeat")
            .field("running", &running)
            .finish()
    }
}

/// An action repeated while a press is held ([`PressRepeat`]), not
/// running.
pub fn press_repeat() -> PressRepeat {
    PressRepeat { task: stored(None) }
}

impl PressRepeat {
    /// How long a press is held before the action repeats.
    pub const HOLD: Duration = Duration::from_millis(400);
    /// How often it repeats after that.
    pub const INTERVAL: Duration = Duration::from_millis(80);

    /// Runs `action` now and keeps running it while the press is held,
    /// until [`Self::stop`], which `action` may call itself (at a bound),
    /// the first time too. A repetition already running stops first.
    pub fn start(&self, action: impl Fn() + 'static) {
        self.stop();
        let action = Rc::new(action);
        // The repetition is in place before the first run, so a stop
        // there ends it.
        let repeated = action.clone();
        self.task.replace(Some(spawn(async move {
            timer(Self::HOLD).await;
            loop {
                repeated();
                timer(Self::INTERVAL).await;
            }
        })));
        action();
    }

    /// Stops repeating (the press ended, or the action reached a bound).
    pub fn stop(&self) {
        if let Some(task) = self.task.replace(None) {
            task.cancel();
        }
    }

    /// Whether the action is repeating.
    pub fn is_running(&self) -> bool {
        self.task.with(|t| t.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::reactive::{Runtime, create_root};

    /// Makes what `setup` makes in a root of a fresh runtime, then runs
    /// `check` on it.
    fn run<T>(setup: impl FnOnce() -> T, check: impl FnOnce(T)) {
        let runtime = Runtime::default();
        let _entered = runtime.enter();
        let (owner, made) = create_root(setup);
        check(made);
        owner.dispose();
    }

    #[test]
    fn an_uncontrolled_value_is_the_components_own() {
        run(
            || controllable(|| None::<i32>, 3),
            |value| {
                assert_eq!(value.get_untracked(), 3);
                assert!(!untracked(|| value.is_controlled()));
                value.set(5);
                assert_eq!(value.get_untracked(), 5);
            },
        );
    }

    #[test]
    fn a_controlled_value_follows_the_app_and_ignores_the_users_change() {
        run(
            || {
                let app = signal(Some(10));
                (app, controllable(app, 0))
            },
            |(app, value)| {
                assert_eq!(value.get_untracked(), 10);
                // The user's change is the app's to take.
                value.set(11);
                assert_eq!(value.get_untracked(), 10);
                app.set(Some(11));
                assert_eq!(value.get_untracked(), 11);
                // Released: the component's own value shows again.
                app.set(None);
                assert_eq!(value.get_untracked(), 0);
                value.set(2);
                assert_eq!(value.get_untracked(), 2);
            },
        );
    }

    #[test]
    fn a_choice_that_may_be_none_is_controlled_through_a_closure() {
        run(
            || {
                let app = signal(None::<u32>);
                (app, controllable(move || app.get().map(Some), None))
            },
            |(app, chosen)| {
                assert_eq!(chosen.get_untracked(), None);
                chosen.set(Some(1));
                assert_eq!(chosen.get_untracked(), Some(1));
                app.set(Some(4));
                assert_eq!(chosen.get_untracked(), Some(4));
            },
        );
    }

    #[test]
    fn readers_of_a_controllable_value_run_again_when_it_changes() {
        let seen = Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = seen.clone();
        run(
            move || {
                let value = controllable(|| None::<i32>, 1);
                crate::ui::reactive::effect(move || log.borrow_mut().push(value.get()));
                value
            },
            |value| {
                value.set(2);
                value.set(2);
                value.set(3);
                assert_eq!(*seen.borrow(), [1, 2, 3]);
            },
        );
    }

    #[test]
    fn a_number_range_orders_its_bounds_and_rounds_to_the_steps_decimals() {
        let range = NumberRange::new(10.0, 0.0, -0.25);
        assert_eq!((range.min(), range.max(), range.step()), (0.0, 10.0, 0.25));
        assert_eq!(range.decimals(), 2);
        assert_eq!(range.fit(1.234), 1.23);
        assert_eq!(range.fit(-3.0), 0.0);
        assert_eq!(range.fit(12.0), 10.0);
        assert_eq!(range.step_by(1.0, 1.0), 1.25);
        assert_eq!(range.step_by(0.1, -1.0), 0.0);
        assert!(range.at_min(0.0) && !range.at_min(0.25));
        assert!(range.at_max(10.0) && !range.at_max(9.75));
        assert_eq!(range.format(1.5), "1.50");
        // Whole steps keep whole numbers.
        let whole = NumberRange::new(0.0, 100.0, 1.0);
        assert_eq!(whole.decimals(), 0);
        assert_eq!(whole.fit(42.4), 42.0);
        assert_eq!(whole.format(42.0), "42");
    }
}
