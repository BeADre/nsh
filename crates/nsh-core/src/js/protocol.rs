//! The op buffer's codes: the operations JS pushes (`OP_*`) and the
//! native animation graph's sub-operations (`ANIM_*`). The canvas's codes
//! are in [`super::canvas`].

codes! { OPS:
    /// id, tag
    OP_CREATE = 0,
    /// id, text
    OP_CREATE_TEXT = 1,
    /// id, text
    OP_SET_TEXT = 2,
    /// id, name, value
    OP_SET_PROP = 3,
    /// parent, child, before (0 = append)
    OP_INSERT = 4,
    /// parent, child
    OP_REMOVE = 5,
    /// id
    OP_RELEASE = 6,
    /// sub-op (`ANIM_*`), node, args...
    OP_ANIM = 7,
    /// id, name, argument
    OP_COMMAND = 8,
    /// id, [canvas operations (`CV_*`)]
    OP_DRAW = 9,
}

codes! { ANIM:
    /// node, value
    ANIM_VALUE = 0,
    /// node, input, inputRange, outputRange, extrapolateLeft, extrapolateRight, easing
    ANIM_INTERPOLATE = 1,
    /// node, op, a, b
    ANIM_MATH = 2,
    /// node, value
    ANIM_SET = 3,
    /// node, animation id, config
    ANIM_START = 4,
    /// node
    ANIM_STOP = 5,
    /// node
    ANIM_RELEASE = 6,
    /// node, input, min, max
    ANIM_DIFF_CLAMP = 7,
    /// animation id, composite key, spec
    ANIM_START_COMPOSITE = 8,
    /// animation id
    ANIM_STOP_ANIMATION = 9,
    /// composite key
    ANIM_RESET_COMPOSITE = 10,
    /// composite key
    ANIM_RELEASE_COMPOSITE = 11,
}

/// `protocol.gen.ts`: the codes as constants (scope hoisting makes them
/// plain module-level constants of the bundle, as written by hand).
pub fn typescript() -> String {
    use std::fmt::Write as _;
    let mut out = super::banner("nsh_core::js::protocol");
    for (name, value, doc) in OPS.iter().chain(ANIM).chain(super::canvas::CODES) {
        let doc = doc.trim();
        let _ = if doc.is_empty() {
            writeln!(out, "export const {name} = {value};")
        } else {
            writeln!(out, "export const {name} = {value}; // {doc}")
        };
    }
    out
}
