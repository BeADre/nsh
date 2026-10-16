//! Native animation values for Rust components (TS: `createNativeValue`,
//! `Animated.spring`, `interpolate`): nodes of the platform's animation
//! graph that styles, drags, scroll offsets and drawings are bound to, so
//! they move on the UI thread without a render per frame.
//!
//! Ids are odd (JS gives its own nodes and animations even ones), so the
//! graph, which indexes nodes by id, stays dense.

use std::{
    fmt,
    rc::{Rc, Weak},
};

use nsh_anim::{AnimOp, Driver, Easing, Extrapolate, MathOp, Operand, Output};

use super::{
    Color,
    task::{Action, Answer, Executor},
};

/// A node of the graph, released when its last handle goes.
pub(crate) struct Node {
    id: nsh_anim::NodeId,
    executor: Weak<Executor>,
    /// The nodes it derives from, alive while it is: a chain such as
    /// `v.mul(TAU).sub(1.0)` keeps no handle of its middle node.
    _inputs: Vec<Rc<Node>>,
}

impl Node {
    fn new(
        executor: &Rc<Executor>,
        op: impl FnOnce(nsh_anim::NodeId) -> AnimOp,
        inputs: Vec<Rc<Node>>,
    ) -> Rc<Self> {
        let id = executor.next_anim_id();
        executor.act(Action::Anim(op(id)));
        Rc::new(Self {
            id,
            executor: Rc::downgrade(executor),
            _inputs: inputs,
        })
    }

    fn act(&self, op: AnimOp) {
        if let Some(executor) = self.executor.upgrade() {
            executor.act(Action::Anim(op));
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        self.act(AnimOp::Release { node: self.id });
    }
}

/// `op` (a node-making operation) making node `id`.
fn with_node(mut op: AnimOp, id: nsh_anim::NodeId) -> AnimOp {
    match &mut op {
        AnimOp::CreateValue { node, .. }
        | AnimOp::CreateInterpolate { node, .. }
        | AnimOp::CreateDiffClamp { node, .. }
        | AnimOp::CreateMath { node, .. } => *node = id,
        _ => {}
    }
    op
}

/// Derived nodes by the operation that made them (its node id 0), while
/// alive.
#[derive(Default)]
pub(crate) struct Derived(pub(crate) Vec<(AnimOp, Weak<Node>)>);

impl Derived {
    pub(crate) fn get(&mut self, key: &AnimOp) -> Option<Rc<Node>> {
        self.0.retain(|(_, n)| n.strong_count() > 0);
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, n)| n.upgrade())
    }
}

/// A value of the graph: [`NativeValue`] or [`NativeNode`], to bind
/// (`Styled::bind_opacity`) or derive from.
pub trait Animated {
    /// The node.
    fn native_node(&self) -> NativeNode;
}

/// A node derived from values (an interpolation, arithmetic): bound like a
/// value, moving when they move.
#[derive(Clone)]
pub struct NativeNode(Rc<Node>);

impl NativeNode {
    pub(crate) fn id(&self) -> nsh_anim::NodeId {
        self.0.id
    }

    fn executor(&self) -> Option<Rc<Executor>> {
        self.0.executor.upgrade()
    }

    /// The node `op` makes from this one: the live one made by the same
    /// operation if there is one (a render deriving it again gets the same
    /// node, so its bindings stay as they are).
    fn derive(
        &self,
        op: impl FnOnce(nsh_anim::NodeId) -> AnimOp,
        other: Option<&NativeNode>,
    ) -> NativeNode {
        let Some(executor) = self.executor() else {
            // The surface is gone: a node nothing reads.
            return self.clone();
        };
        let key = op(0);
        if let Some(node) = executor.derived(&key) {
            return NativeNode(node);
        }
        let mut inputs = vec![self.0.clone()];
        inputs.extend(other.map(|o| o.0.clone()));
        let node = Node::new(&executor, |id| with_node(key.clone(), id), inputs);
        executor.remember_derived(key, &node);
        NativeNode(node)
    }

    /// Maps the value through `input` breakpoints (ascending) to numbers,
    /// clamped at both ends: `x.interpolate(&[0.0, 300.0], &[1.0, 0.0])`.
    pub fn interpolate(&self, input: &[f32], output: &[f32]) -> NativeNode {
        self.interpolate_with(
            input,
            Output::Numbers(output.to_vec()),
            Extrapolate::Clamp,
            Easing::Linear,
        )
    }

    /// Maps the value to colors (`backgroundColor` bindings).
    pub fn interpolate_colors(&self, input: &[f32], colors: &[Color]) -> NativeNode {
        let colors = colors.iter().map(|c| c.0).collect();
        self.interpolate_with(
            input,
            Output::Colors(colors),
            Extrapolate::Clamp,
            Easing::Linear,
        )
    }

    /// [`Self::interpolate`] with any output, extrapolation and curve.
    pub fn interpolate_with(
        &self,
        input: &[f32],
        output: Output,
        extrapolate: Extrapolate,
        easing: Easing,
    ) -> NativeNode {
        let source = self.id();
        let input_range = input.to_vec();
        self.derive(
            move |node| AnimOp::CreateInterpolate {
                node,
                input: source,
                input_range,
                output,
                extrapolate_left: extrapolate,
                extrapolate_right: extrapolate,
                easing,
            },
            None,
        )
    }

    fn math(&self, op: MathOp, other: AnimOperand) -> NativeNode {
        let a = Operand::Node(self.id());
        let b = other.operand;
        self.derive(
            move |node| AnimOp::CreateMath { node, op, a, b },
            other.node.as_ref(),
        )
    }

    /// The sum with `other`.
    pub fn add(&self, other: impl Into<AnimOperand>) -> NativeNode {
        self.math(MathOp::Add, other.into())
    }

    /// The difference with `other`.
    pub fn sub(&self, other: impl Into<AnimOperand>) -> NativeNode {
        self.math(MathOp::Sub, other.into())
    }

    /// The product with `other`.
    pub fn mul(&self, other: impl Into<AnimOperand>) -> NativeNode {
        self.math(MathOp::Mul, other.into())
    }

    /// The quotient by `other`.
    pub fn div(&self, other: impl Into<AnimOperand>) -> NativeNode {
        self.math(MathOp::Div, other.into())
    }

    /// The remainder by `other`.
    pub fn modulo(&self, other: impl Into<AnimOperand>) -> NativeNode {
        self.math(MathOp::Mod, other.into())
    }

    /// Follows the value's changes but stays within `min..=max` (a header
    /// hiding as content scrolls up).
    pub fn diff_clamp(&self, min: f32, max: f32) -> NativeNode {
        let input = self.id();
        self.derive(
            move |node| AnimOp::CreateDiffClamp {
                node,
                input,
                min,
                max,
            },
            None,
        )
    }
}

impl Animated for NativeNode {
    fn native_node(&self) -> NativeNode {
        self.clone()
    }
}

impl PartialEq for NativeNode {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl fmt::Debug for NativeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NativeNode").field(&self.id()).finish()
    }
}

/// An operand of arithmetic on nodes: a number or another node.
#[derive(Debug, Clone)]
pub struct AnimOperand {
    operand: Operand,
    /// The node, kept alive by what derives from it.
    node: Option<NativeNode>,
}

impl From<f32> for AnimOperand {
    fn from(n: f32) -> Self {
        Self {
            operand: Operand::Const(n),
            node: None,
        }
    }
}

impl<A: Animated> From<&A> for AnimOperand {
    fn from(a: &A) -> Self {
        let node = a.native_node();
        Self {
            operand: Operand::Node(node.id()),
            node: Some(node),
        }
    }
}

/// A settable, animatable value (made with
/// [`super::Context::native_value`]): set it, animate it, bind it, derive
/// from it. Clones are the same value.
#[derive(Clone, PartialEq, Debug)]
pub struct NativeValue(NativeNode);

/// How a spring moves (React Native's defaults: stiffness 100, damping 10,
/// mass 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    /// Stiffness `k`.
    pub stiffness: f32,
    /// Damping `c`.
    pub damping: f32,
    /// Mass `m`.
    pub mass: f32,
    /// The velocity it starts with (per second); `None` keeps the current
    /// one.
    pub velocity: Option<f32>,
}

impl Default for Spring {
    fn default() -> Self {
        Self {
            stiffness: 100.0,
            damping: 10.0,
            mass: 1.0,
            velocity: None,
        }
    }
}

/// The end of an animation: `true` when it reached its end, `false` when
/// it was stopped or replaced. The animation runs either way: await this
/// in a task to follow it up, or drop it.
#[derive(Debug)]
pub struct AnimationEnd(Answer<bool>);

impl std::future::IntoFuture for AnimationEnd {
    type Output = bool;
    type IntoFuture = Answer<bool>;

    fn into_future(self) -> Answer<bool> {
        self.0
    }
}

impl NativeValue {
    pub(crate) fn new(executor: &Rc<Executor>, value: f32) -> Self {
        Self(NativeNode(Node::new(
            executor,
            |node| AnimOp::CreateValue { node, value },
            Vec::new(),
        )))
    }

    /// The node, to derive from.
    pub fn node(&self) -> &NativeNode {
        &self.0
    }

    /// Sets the value at once, stopping its animation.
    pub fn set(&self, value: f32) {
        self.0.0.act(AnimOp::Set {
            node: self.0.id(),
            value,
        });
    }

    /// Stops its animation where it is.
    pub fn stop(&self) {
        self.0.0.act(AnimOp::Stop { node: self.0.id() });
    }

    fn start(&self, driver: Driver) -> AnimationEnd {
        let (end, slot) = Answer::new();
        let end = AnimationEnd(end);
        if let Some(executor) = self.0.executor() {
            let animation = executor.next_anim_id();
            executor.act(Action::Animate {
                op: AnimOp::Start {
                    node: self.0.id(),
                    animation,
                    driver,
                },
                animation,
                end: slot,
            });
        }
        end
    }

    /// Springs to `to`.
    pub fn spring(&self, to: f32, spring: Spring) -> AnimationEnd {
        self.start(Driver::Spring {
            to,
            stiffness: spring.stiffness,
            damping: spring.damping,
            mass: spring.mass,
            velocity: spring.velocity,
        })
    }

    /// Moves to `to` in `ms` milliseconds along `easing`.
    pub fn timing(&self, to: f32, ms: f32, easing: Easing) -> AnimationEnd {
        self.start(Driver::Timing {
            to,
            duration_ms: ms,
            easing,
            delay_ms: 0.0,
        })
    }

    /// Glides from `velocity` (per second; `None`: the current one),
    /// slowing down by `deceleration` per millisecond (0.998: a fling).
    pub fn decay(&self, velocity: Option<f32>, deceleration: f32) -> AnimationEnd {
        self.start(Driver::Decay {
            velocity,
            deceleration,
        })
    }

    /// See [`NativeNode::interpolate`].
    pub fn interpolate(&self, input: &[f32], output: &[f32]) -> NativeNode {
        self.0.interpolate(input, output)
    }

    /// See [`NativeNode::interpolate_colors`].
    pub fn interpolate_colors(&self, input: &[f32], colors: &[Color]) -> NativeNode {
        self.0.interpolate_colors(input, colors)
    }
}

impl Animated for NativeValue {
    fn native_node(&self) -> NativeNode {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn released(executor: &Executor) -> usize {
        executor
            .take_actions()
            .into_iter()
            .filter(|a| matches!(a, Action::Anim(AnimOp::Release { .. })))
            .count()
    }

    #[test]
    fn a_derived_chain_keeps_its_middle_nodes() {
        let executor = Rc::new(Executor::default());
        let v = NativeValue::new(&executor, 0.5);
        // The product has no handle of its own once the difference exists.
        let end = v.node().mul(2.0).sub(1.0);
        assert_eq!(released(&executor), 0, "the product lives with the end");
        drop(end);
        assert_eq!(released(&executor), 2, "then both go");
        drop(v);
        assert_eq!(released(&executor), 1);
    }
}
