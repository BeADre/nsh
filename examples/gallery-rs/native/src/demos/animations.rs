//! Animations: easing curves, composed animations (sequence, parallel,
//! stagger, loop) with stop and reset, springs, interpolation, `diff_clamp`
//! and layout animation (as the TSX gallery's `demos/Animations.tsx`).

use std::time::Duration;

use nsh_core::{
    nsh_anim::{Curve, EaseMode},
    ui::{AnimOutput, AnyElement, Extrapolate, LayoutAnimation, Text, View, prelude::*},
};

use crate::ui::{button, colors, page, section};

/// How far the dots and squares travel.
const TRACK: f32 = 220.0;

/// The colors of the composition's squares and of the layout's bars.
const PALETTE: [u32; 5] = [0xfd8c73, 0xf2cc60, 0x4ac26b, 0x54aeff, 0x8250df];

/// React Native's `Easing.ease` (CSS `ease`).
const EASE: Easing = Easing::Bezier(0.25, 0.1, 0.25, 1.0);

/// `Easing.inOut(Easing.cubic)`.
const CUBIC_IN_OUT: Easing = Easing::Curve(Curve::Cubic, EaseMode::InOut);

/// `Easing.out(Easing.back())`.
const BACK_OUT: Easing = Easing::Curve(Curve::Back(1.70158), EaseMode::Out);

/// `Easing.sin`.
const SIN: Easing = Easing::Curve(Curve::Sin, EaseMode::In);

/// What `Animated.timing` moves along without an easing.
const TIMING_DEFAULT: Easing = Easing::EaseInOut;

/// A pill-shaped text button.
fn chip(label: impl Into<SharedString>, on_press: Listener<()>) -> Text {
    text(label)
        .px_3()
        .py(px(6.0))
        .rounded(px(14.0))
        .overflow_hidden()
        .bg(rgb(0xddf4ff))
        .text_color(colors::accent())
        .font_weight(FontWeight::SEMIBOLD)
        .on_press(on_press)
}

/// A row of a track: its label, then `track`.
fn track_row(label: &str, track: View) -> AnyElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(
            text(label.to_owned())
                .w(px(84.0))
                .text_size(px(12.0))
                .text_color(colors::muted()),
        )
        .child(track)
        .into_element()
}

/// One dot per easing, all started together natively.
#[derive(Debug, Default)]
struct Curves {
    /// The dots' offsets, one per curve.
    dots: Vec<NativeValue>,
    /// At the end of the track (the next play goes back).
    there: bool,
}

impl Curves {
    fn curves() -> [(&'static str, Easing); 10] {
        [
            ("linear", Easing::Linear),
            ("ease", EASE),
            ("quad out", Easing::Curve(Curve::Quad, EaseMode::Out)),
            ("cubic inOut", CUBIC_IN_OUT),
            ("sin in", SIN),
            ("circle out", Easing::Curve(Curve::Circle, EaseMode::Out)),
            ("exp inOut", Easing::Curve(Curve::Exp, EaseMode::InOut)),
            ("back out", BACK_OUT),
            (
                "elastic out",
                Easing::Curve(Curve::Elastic(1.0), EaseMode::Out),
            ),
            ("bounce out", Easing::Curve(Curve::Bounce, EaseMode::Out)),
        ]
    }
}

impl Component for Curves {
    const NAME: &'static str = "gallery.animations.curves";
    type Props = ();
    type Event = NoEvent;

    fn setup(&mut self, _: &(), cx: &mut Context<'_, Self>) {
        self.dots = Self::curves()
            .iter()
            .map(|_| cx.native_value(0.0))
            .collect();
    }

    fn render(&self, _: &(), cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        let rows = Self::curves()
            .into_iter()
            .zip(&self.dots)
            .map(|((label, _), dot)| {
                track_row(
                    label,
                    view()
                        .w(px(TRACK + 14.0))
                        .h(px(14.0))
                        .rounded(px(7.0))
                        .bg(rgb(0xf6f8fa))
                        .child(
                            view()
                                .size(px(14.0))
                                .rounded(px(7.0))
                                .bg(colors::accent())
                                .bind_translate_x(dot),
                        ),
                )
            });
        view()
            .gap(px(6.0))
            .children(rows)
            .child(button("Play", colors::accent()).on_press(cx.listener(
                |this: &mut Self, _, _| {
                    this.there = !this.there;
                    let to = if this.there { TRACK } else { 0.0 };
                    for (dot, (_, easing)) in this.dots.iter().zip(Self::curves()) {
                        // Each runs to its end natively; nothing follows it up.
                        let _ = dot.timing(to, 1000.0, easing);
                    }
                },
            )))
    }
}

/// A composed animation of [`Composition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Run {
    Stagger,
    Sequence,
    Loop,
}

impl Run {
    /// In the order the stop and reset chips go through them.
    const ALL: [Run; 3] = [Run::Stagger, Run::Sequence, Run::Loop];

    fn name(self) -> &'static str {
        match self {
            Run::Stagger => "stagger",
            Run::Sequence => "sequence",
            Run::Loop => "loop",
        }
    }
}

/// A started [`Run`]: dropping its task stops following it up.
#[derive(Debug)]
struct Running {
    run: Run,
    /// Tells this start from a later start of the same run.
    serial: u32,
    task: Task<()>,
}

/// `sequence`, `parallel`, `stagger` and `loop`, with stop and reset.
///
/// The Rust API has no composite animations (TS: `Animated.sequence`, run
/// natively step after step), so each composition is a task awaiting its
/// steps' ends and starting the next ones.
#[derive(Debug, Default)]
struct Composition {
    /// The squares' vertical offsets.
    ys: Vec<NativeValue>,
    /// The squares' turn (0..1, each square turning its own angle).
    spin: Option<NativeValue>,
    /// The squares' opacity.
    fade: Option<NativeValue>,
    /// What last happened ("idle", "stagger…", "loop: stopped").
    state: String,
    /// The runs going on.
    running: Vec<Running>,
    /// The last start's serial.
    serial: u32,
}

impl Composition {
    fn spin(&self) -> &NativeValue {
        self.spin.as_ref().expect("set up")
    }

    fn fade(&self) -> &NativeValue {
        self.fade.as_ref().expect("set up")
    }

    /// Starts `run` (again: a start replaces the one going on): `play`
    /// animates and says whether every step reached its end.
    fn start<F: Future<Output = bool> + 'static>(
        &mut self,
        run: Run,
        play: impl FnOnce(AsyncCx) -> F + 'static,
        cx: &mut Context<'_, Self>,
    ) {
        self.serial += 1;
        let serial = self.serial;
        self.running.retain(|r| r.run != run);
        let task = cx.spawn(move |this, cx| async move {
            let finished = play(cx).await;
            this.update(move |this, cx| {
                this.running.retain(|r| r.serial != serial);
                let end = if finished { "finished" } else { "stopped" };
                this.state = format!("{}: {end}", run.name());
                cx.notify();
            })
            .await;
        });
        self.running.push(Running { run, serial, task });
        self.state = format!("{}…", run.name());
        cx.notify();
    }

    /// The squares jump one after the other, 80 ms apart.
    fn stagger(&mut self, cx: &mut Context<'_, Self>) {
        let parts: Vec<Task<bool>> = self
            .ys
            .iter()
            .enumerate()
            .map(|(i, y)| {
                let y = y.clone();
                cx.spawn(move |_, cx| async move {
                    cx.timer(Duration::from_millis(80 * i as u64)).await;
                    y.timing(-36.0, 220.0, Easing::Curve(Curve::Quad, EaseMode::Out))
                        .await
                        && y.spring(
                            0.0,
                            Spring {
                                stiffness: 300.0,
                                damping: 12.0,
                                ..Spring::default()
                            },
                        )
                        .await
                })
            })
            .collect();
        self.start(
            Run::Stagger,
            |_| async move {
                let mut finished = true;
                for part in parts {
                    finished &= part.await;
                }
                finished
            },
            cx,
        );
    }

    /// Turn, wait, fade and turn back together, fade in.
    fn sequence(&mut self, cx: &mut Context<'_, Self>) {
        let (spin, fade) = (self.spin().clone(), self.fade().clone());
        self.start(
            Run::Sequence,
            |cx| async move {
                if !spin.timing(1.0, 500.0, CUBIC_IN_OUT).await {
                    return false;
                }
                cx.timer(Duration::from_millis(200)).await;
                let faded = fade.timing(0.2, 300.0, TIMING_DEFAULT);
                let turned = spin.timing(0.0, 600.0, BACK_OUT);
                if !(faded.await & turned.await) {
                    return false;
                }
                fade.timing(1.0, 300.0, TIMING_DEFAULT).await
            },
            cx,
        );
    }

    /// A pulse of the opacity, until stopped.
    fn pulse(&mut self, cx: &mut Context<'_, Self>) {
        let fade = self.fade().clone();
        self.start(
            Run::Loop,
            |_| async move {
                loop {
                    if !(fade.timing(0.3, 400.0, SIN).await && fade.timing(1.0, 400.0, SIN).await) {
                        return false;
                    }
                }
            },
            cx,
        );
    }

    /// Stops every run where it is (each going on reports it stopped).
    fn stop(&mut self) {
        for run in Run::ALL {
            if self.running.iter().any(|r| r.run == run) {
                self.state = format!("{}: stopped", run.name());
            }
        }
        // Dropping the tasks also ends the waits between steps.
        for running in self.running.drain(..) {
            drop(running.task);
        }
        for y in &self.ys {
            y.stop();
        }
        self.spin().stop();
        self.fade().stop();
    }
}

impl Component for Composition {
    const NAME: &'static str = "gallery.animations.composition";
    type Props = ();
    type Event = NoEvent;

    fn setup(&mut self, _: &(), cx: &mut Context<'_, Self>) {
        self.ys = PALETTE.iter().map(|_| cx.native_value(0.0)).collect();
        self.spin = Some(cx.native_value(0.0));
        self.fade = Some(cx.native_value(1.0));
        self.state = "idle".into();
    }

    fn render(&self, _: &(), cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        let squares = PALETTE
            .iter()
            .zip(&self.ys)
            .enumerate()
            .map(|(i, (&color, y))| {
                let angle = (i + 1) as f32 * 36.0;
                view()
                    .size(px(34.0))
                    .rounded(px(8.0))
                    .bg(rgb(color))
                    .bind_opacity(self.fade())
                    .bind_translate_y(y)
                    .bind_rotate(&self.spin().interpolate(&[0.0, 1.0], &[0.0, angle]))
            });
        view()
            .gap_3()
            .child(
                view()
                    .flex_row()
                    .justify_center()
                    .items_end()
                    .gap(px(10.0))
                    .h(px(80.0))
                    .children(squares),
            )
            .child(
                text(self.state.clone())
                    .text_size(px(13.0))
                    .text_color(colors::muted())
                    .text_center(),
            )
            .child(
                view()
                    .flex_row()
                    .flex_wrap()
                    .gap_2()
                    .child(chip(
                        "Stagger",
                        cx.listener(|this: &mut Self, _, cx| this.stagger(cx)),
                    ))
                    .child(chip(
                        "Sequence",
                        cx.listener(|this: &mut Self, _, cx| this.sequence(cx)),
                    ))
                    .child(chip(
                        "Loop",
                        cx.listener(|this: &mut Self, _, cx| this.pulse(cx)),
                    ))
                    .child(chip(
                        "Stop",
                        cx.listener(|this: &mut Self, _, cx| {
                            this.stop();
                            cx.notify();
                        }),
                    ))
                    .child(chip(
                        "Reset",
                        cx.listener(|this: &mut Self, _, cx| {
                            this.stop();
                            // Where they rest (TS's reset puts them back
                            // where they were when the run started).
                            for y in &this.ys {
                                y.set(0.0);
                            }
                            this.spin().set(0.0);
                            this.fade().set(1.0);
                            cx.notify();
                        }),
                    )),
            )
    }
}

/// Springs with different settings, released together.
#[derive(Debug, Default)]
struct Springs {
    /// The squares' offsets, one per setting.
    xs: Vec<NativeValue>,
    /// At the end of the track.
    there: bool,
}

impl Springs {
    fn settings() -> [(&'static str, Spring); 3] {
        let spring = |stiffness, damping| Spring {
            stiffness,
            damping,
            ..Spring::default()
        };
        [
            ("gentle", spring(120.0, 14.0)),
            ("wobbly", spring(180.0, 8.0)),
            ("stiff", spring(400.0, 30.0)),
        ]
    }
}

impl Component for Springs {
    const NAME: &'static str = "gallery.animations.springs";
    type Props = ();
    type Event = NoEvent;

    fn setup(&mut self, _: &(), cx: &mut Context<'_, Self>) {
        self.xs = Self::settings()
            .iter()
            .map(|_| cx.native_value(0.0))
            .collect();
    }

    fn render(&self, _: &(), cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        let rows = Self::settings()
            .into_iter()
            .zip(&self.xs)
            .map(|((label, _), x)| {
                track_row(
                    label,
                    view().w(px(TRACK + 20.0)).h(px(20.0)).child(
                        view()
                            .size(px(20.0))
                            .rounded(px(4.0))
                            .bg(colors::purple())
                            .bind_translate_x(x),
                    ),
                )
            });
        view()
            .gap(px(6.0))
            .children(rows)
            .child(button("Release", colors::purple()).on_press(cx.listener(
                |this: &mut Self, _, _| {
                    this.there = !this.there;
                    let to = if this.there { TRACK } else { 0.0 };
                    for (x, (_, spring)) in this.xs.iter().zip(Self::settings()) {
                        let _ = x.spring(to, spring);
                    }
                },
            )))
    }
}

/// Colors and angles from one value; outside its range it extends on the
/// right only.
#[derive(Debug, Default)]
struct Interpolations {
    /// The value: 0, 1 or 2 at rest.
    t: Option<NativeValue>,
    /// Where it goes.
    at: u8,
}

impl Interpolations {
    fn t(&self) -> &NativeValue {
        self.t.as_ref().expect("set up")
    }
}

impl Component for Interpolations {
    const NAME: &'static str = "gallery.animations.interpolations";
    type Props = ();
    type Event = NoEvent;

    fn setup(&mut self, _: &(), cx: &mut Context<'_, Self>) {
        self.t = Some(cx.native_value(0.0));
    }

    fn render(&self, _: &(), cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        let t = self.t();
        let color = t.interpolate_colors(
            &[0.0, 1.0, 2.0],
            &[rgb(0x54aeff), rgb(0x8250df), rgb(0xcf222e)],
        );
        // Extends on both sides (TS: on the right only; `t` never goes
        // below 0, so the left side never shows).
        let angle = t.node().interpolate_with(
            &[0.0, 1.0],
            AnimOutput::Numbers(vec![0.0, 180.0]),
            Extrapolate::Extend,
            Easing::Linear,
        );
        let scale = t.interpolate(&[0.0, 1.0], &[0.6, 1.2]);
        let chips = (0..3u8).map(|n| {
            let label = format!("t = {n}{}", if self.at == n { " ●" } else { "" });
            chip(
                label,
                cx.listener(move |this: &mut Self, _, cx| {
                    this.at = n;
                    let _ = this.t().timing(f32::from(n), 700.0, CUBIC_IN_OUT);
                    cx.notify();
                }),
            )
        });
        view()
            .gap(px(10.0))
            .child(
                view()
                    .flex_row()
                    .justify_center()
                    .gap_6()
                    .h(px(90.0))
                    .items_center()
                    .child(
                        view()
                            .size(px(60.0))
                            .rounded(px(12.0))
                            .bind_background(&color)
                            .bind_rotate(&angle),
                    )
                    .child(
                        view()
                            .size(px(60.0))
                            .rounded(px(30.0))
                            .bg(rgb(0x4ac26b))
                            .bind_scale(&scale),
                    ),
            )
            .child(view().flex_row().gap_2().children(chips))
            .child(
                text("Color through three stops; rotation extends past 1 (to 360° at 2); scale clamps at 1.2.")
                    .text_size(px(12.0))
                    .text_color(colors::muted()),
            )
    }
}

/// A header that hides while the list scrolls up and comes back as it
/// scrolls down: `diff_clamp` of the scroll offset, natively.
#[derive(Debug, Default)]
struct HidingHeader {
    /// The scroll offset.
    scroll: Option<NativeValue>,
}

impl Component for HidingHeader {
    const NAME: &'static str = "gallery.animations.hiding_header";
    type Props = ();
    type Event = NoEvent;

    fn setup(&mut self, _: &(), cx: &mut Context<'_, Self>) {
        self.scroll = Some(cx.native_value(0.0));
    }

    fn render(&self, _: &(), _: &mut RenderCx<'_, Self>) -> impl IntoElement {
        let scroll_value = self.scroll.as_ref().expect("set up");
        let header = scroll_value.node().diff_clamp(0.0, 44.0);
        let rows = (1..=30).map(|n| {
            view()
                .p(px(10.0))
                .rounded(px(8.0))
                .bg(colors::surface())
                .child(text(format!("Row {n}")))
        });
        view()
            .h(px(260.0))
            .rounded(px(10.0))
            .overflow_hidden()
            .bg(rgb(0xf6f8fa))
            .child(
                scroll()
                    .flex_1()
                    .scroll_value(scroll_value)
                    .child(view().pt(px(52.0)).px_3().pb_3().gap_2().children(rows)),
            )
            .child(
                view()
                    .absolute()
                    .left(px(0.0))
                    .right(px(0.0))
                    .top(px(0.0))
                    .h(px(44.0))
                    .justify_center()
                    .px_3()
                    .bg(colors::accent())
                    .bind_translate_y(&header.mul(-1.0))
                    .bind_opacity(&header.interpolate_with(
                        &[0.0, 44.0],
                        AnimOutput::Numbers(vec![1.0, 0.0]),
                        Extrapolate::Clamp,
                        Easing::Curve(Curve::Quad, EaseMode::In),
                    ))
                    .child(
                        text("Scroll up to hide me")
                            .text_color(rgb(0xffffff))
                            .font_weight(FontWeight::BOLD),
                    ),
            )
    }
}

/// Layout changes animated natively by the next commit.
#[derive(Debug, Default)]
struct Layout {
    /// The bars in a column (else in a row).
    open: bool,
}

impl Component for Layout {
    const NAME: &'static str = "gallery.animations.layout";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        let open = self.open;
        let bars = PALETTE.iter().map(|&color| {
            view()
                .h(px(if open { 24.0 } else { 44.0 }))
                .when(!open, |v| v.flex_1())
                .rounded(px(8.0))
                .bg(rgb(color))
        });
        view()
            .gap(px(10.0))
            .child(
                view()
                    .map(|v| if open { v.flex_col() } else { v.flex_row() })
                    .gap_2()
                    .children(bars),
            )
            .child(
                button(if open { "Row" } else { "Column" }, colors::gray()).on_press(cx.listener(
                    |this: &mut Self, _, cx| {
                        cx.animate_layout(LayoutAnimation::SPRING);
                        this.open = !this.open;
                        cx.notify();
                    },
                )),
            )
    }
}

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct Animations;

impl Component for Animations {
    const NAME: &'static str = "gallery.animations";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([
            section("Easing curves", [Curves::element(()).into_element()]).into_element(),
            section(
                "Sequence, parallel, stagger, loop",
                [Composition::element(()).into_element()],
            )
            .into_element(),
            section("Springs", [Springs::element(()).into_element()]).into_element(),
            section(
                "Interpolation",
                [Interpolations::element(()).into_element()],
            )
            .into_element(),
            section(
                "diffClamp: a header that hides on scroll",
                [HidingHeader::element(()).into_element()],
            )
            .into_element(),
            section("Layout animation", [Layout::element(()).into_element()]).into_element(),
        ]))
    }
}
