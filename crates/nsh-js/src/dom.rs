//! The renderer compiled JSX runs on: the natives of `nsh:rt` (internal:
//! only the bundler's JSX output imports it) and the parts of `nsh` built
//! on them (`context`, `mount`).
//!
//! A node is a JS object of a native class holding its id and what keeps
//! the tree alive: a parent holds its children's objects and a node holds
//! its event handlers, and the GC mark reports both, so the tree, the
//! handlers and whatever they capture are one heap the QuickJS collector
//! sees. The root view (id 1) is held by the renderer. A node nothing
//! refers to any more is collected and its native view released, as with
//! the JS runtime (a removed node may be inserted again while JS holds it).
//!
//! Every change becomes an [`Op`] pushed straight into the host's op queue
//! (no JS op buffer, no decoding of one). Dynamic children are kept by a
//! render effect whose value is the array of nodes it inserted, reconciled
//! with Solid's `reconcileArrays` against the parent's children, which the
//! renderer mirrors.
//!
//! Re-entrancy: freeing a JS value can finalize a node, whose finalizer
//! updates the renderer (`nodes`, the op queue). The renderer is only ever
//! reached by shared reference with short `RefCell` borrows, none held
//! across a call into JS, an allocation of a JS value or a free of one.

use std::{
    cell::{Cell, RefCell},
    ffi::{CStr, c_int, c_void},
    ptr,
    rc::Rc,
};

use nsh_core::{EventKind, NodeId, Op, Tag, platform::Os};
use rquickjs::{Ctx, Value, qjs};
use rustc_hash::FxHashMap;

use crate::{
    host::Shared,
    reactive::{self, Native, arg},
};

type JsValue = qjs::JSValue;
type JsContext = qjs::JSContext;

const UNDEFINED: JsValue = qjs::JS_UNDEFINED;
/// No parent.
const DETACHED: NodeId = 0;
/// The surface's root view.
const ROOT: NodeId = 1;
/// How deep nested arrays and accessors are unwrapped.
const MAX_DEPTH: u32 = 64;

/// The renderer's part of a context's state.
pub(crate) struct Dom {
    shared: Rc<Shared>,
    classes: Classes,
    /// The next id JS gives a node (even: the core gives odd ones).
    next_id: Cell<NodeId>,
    /// Live nodes by `id >> 1` (the root is 0); weak: a node's object owns
    /// its data.
    nodes: RefCell<Vec<*mut NodeData>>,
    /// The root view's object (owned).
    root: Cell<JsValue>,
    /// The platform: `ios={{…}}` / `android={{…}}` / `desktop={{…}}` props
    /// apply on it only.
    os: Cell<Os>,
    /// The layout direction sent to the root (`None`: nothing mounted yet).
    rtl: Cell<Option<bool>>,
    /// Whether the user's language is written right to left.
    env_rtl: Cell<bool>,
}

#[derive(Clone, Copy)]
struct Classes {
    node: qjs::JSClassID,
    each: qjs::JSClassID,
    context: qjs::JSClassID,
}

struct NodeData {
    id: NodeId,
    /// A text node.
    text: bool,
    /// The parent in the mirror ([`DETACHED`]: none).
    parent: NodeId,
    /// The children in order (owned objects).
    children: Vec<JsValue>,
    /// Event handlers (owned).
    handlers: Vec<(EventKind, JsValue)>,
}

/// The renderer of `ctx`.
#[inline]
fn dom<'a>(ctx: *mut JsContext) -> &'a Dom {
    // SAFETY: a projection of the state's field; the renderer is only ever
    // reached by shared reference.
    unsafe { &(*crate::state::state(ctx)).dom }
}

/// The node data an object holds, or null for other values.
#[inline]
fn node_ptr(ctx: *mut JsContext, v: JsValue) -> *mut NodeData {
    // SAFETY: reads the opaque pointer only for objects of the node class.
    unsafe { qjs::JS_GetOpaque(v, dom(ctx).classes.node) as *mut NodeData }
}

/// Whether two values are the same object.
#[inline]
fn same(a: JsValue, b: JsValue) -> bool {
    // SAFETY: inline tag and pointer reads.
    unsafe {
        qjs::JS_VALUE_GET_TAG(a) == qjs::JS_TAG_OBJECT
            && qjs::JS_VALUE_GET_TAG(b) == qjs::JS_TAG_OBJECT
            && qjs::JS_VALUE_GET_PTR(a) == qjs::JS_VALUE_GET_PTR(b)
    }
}

#[inline]
fn ptr_of(v: JsValue) -> usize {
    // SAFETY: inline pointer read.
    unsafe { qjs::JS_VALUE_GET_PTR(v) as usize }
}

fn throw_type(ctx: *mut JsContext, message: &str) -> JsValue {
    let message = std::ffi::CString::new(message).unwrap_or_default();
    // SAFETY: throws on a live context; `%s` keeps the message literal.
    unsafe { qjs::JS_ThrowTypeError(ctx, c"%s".as_ptr(), message.as_ptr()) }
}

/// A JS value's string (`ToString`), or `None` when that throws.
fn to_string(ctx: *mut JsContext, v: JsValue) -> Option<String> {
    // SAFETY: a live value; the C string is freed below.
    unsafe {
        let mut len = 0usize;
        let p = qjs::JS_ToCStringLen(ctx, &mut len, v);
        if p.is_null() {
            return None;
        }
        let s =
            String::from_utf8_lossy(std::slice::from_raw_parts(p.cast::<u8>(), len)).into_owned();
        qjs::JS_FreeCString(ctx, p);
        Some(s)
    }
}

impl Dom {
    pub(crate) fn new(raw: *mut JsContext, shared: Rc<Shared>) -> Dom {
        // SAFETY: registering classes on the context's runtime.
        let classes = unsafe {
            let rt = qjs::JS_GetRuntime(raw);
            let class = |name: &'static CStr,
                         finalizer: qjs::JSClassFinalizer,
                         gc_mark: qjs::JSClassGCMark,
                         call: qjs::JSClassCall| {
                let mut id: qjs::JSClassID = 0;
                qjs::JS_NewClassID(rt, &mut id);
                let def = qjs::JSClassDef {
                    class_name: name.as_ptr(),
                    finalizer,
                    gc_mark,
                    call,
                    exotic: ptr::null_mut(),
                };
                qjs::JS_NewClass(rt, id, &def);
                id
            };
            let classes = Classes {
                node: class(c"Node", Some(node_finalize), Some(node_mark), None),
                each: class(c"EachRows", Some(each_finalize), Some(each_mark), None),
                context: class(
                    c"Context",
                    Some(context_finalize),
                    Some(context_mark),
                    Some(context_call),
                ),
            };
            // Calls of a context are function calls.
            qjs::JS_SetClassProto(raw, classes.context, qjs::JS_GetFunctionProto(raw));
            classes
        };
        Dom {
            shared,
            classes,
            next_id: Cell::new(2),
            nodes: RefCell::new(Vec::new()),
            root: Cell::new(UNDEFINED),
            os: Cell::new(Os::Ios),
            rtl: Cell::new(None),
            env_rtl: Cell::new(false),
        }
    }

    /// Frees the values the renderer holds (the state is unregistered).
    pub(crate) fn free_all(&mut self, raw: *mut JsContext) {
        // SAFETY: the renderer's own value; no JS runs from here on.
        unsafe { qjs::JS_FreeValue(raw, self.root.replace(UNDEFINED)) };
    }

    /// Takes in the surface's environment.
    pub(crate) fn set_environment(&self, os: Os, rtl: bool) {
        self.os.set(os);
        self.env_rtl.set(rtl);
        if self.rtl.get().is_some_and(|sent| sent != rtl) {
            self.send_direction();
        }
    }

    fn send_direction(&self) {
        let rtl = self.env_rtl.get();
        self.rtl.set(Some(rtl));
        self.shared.ops.borrow_mut().push(Op::SetProp {
            id: ROOT,
            prop: nsh_core::Prop::LayoutDirection(rtl),
        });
    }

    /// Queues `op`; `Err` (with a thrown error) over the VM's limit.
    fn push(&self, ctx: *mut JsContext, op: Op) -> Result<(), JsValue> {
        let mut ops = self.shared.ops.borrow_mut();
        if ops.len() >= self.shared.limits.max_op_values {
            drop(ops);
            let reason = format!(
                "the app made more than {} changes at once and was stopped",
                self.shared.limits.max_op_values
            );
            self.shared.stop(reason.clone());
            // SAFETY: throws on a live context.
            return Err(unsafe {
                let message = std::ffi::CString::new(reason).unwrap_or_default();
                qjs::JS_ThrowRangeError(ctx, c"%s".as_ptr(), message.as_ptr())
            });
        }
        ops.push(op);
        Ok(())
    }

    fn by_id(&self, id: NodeId) -> *mut NodeData {
        self.nodes
            .borrow()
            .get((id >> 1) as usize)
            .copied()
            .unwrap_or(ptr::null_mut())
    }
}

/// Makes the root view's object (after the state is installed).
pub(crate) fn init(raw: *mut JsContext) {
    let root = new_node(raw, ROOT, false);
    dom(raw).root.set(root);
}

/// A new node object for id `id` (owned).
fn new_node(ctx: *mut JsContext, id: NodeId, text: bool) -> JsValue {
    let d = dom(ctx);
    let data = Box::into_raw(Box::new(NodeData {
        id,
        text,
        parent: DETACHED,
        children: Vec::new(),
        handlers: Vec::new(),
    }));
    // SAFETY: a registered class; the opaque is the boxed data, freed by
    // the finalizer.
    let obj = unsafe {
        let obj = qjs::JS_NewObjectClass(ctx, d.classes.node as _);
        qjs::JS_SetOpaque(obj, data as *mut c_void);
        obj
    };
    let index = (id >> 1) as usize;
    let mut nodes = d.nodes.borrow_mut();
    if nodes.len() <= index {
        nodes.resize(index + 1, ptr::null_mut());
    }
    nodes[index] = data;
    obj
}

fn next_id(ctx: *mut JsContext) -> NodeId {
    let d = dom(ctx);
    let id = d.next_id.get();
    d.next_id.set(id + 2);
    id
}

unsafe extern "C" fn node_finalize(rt: *mut qjs::JSRuntime, obj: JsValue) {
    // SAFETY: the opaque of a node object is its boxed data.
    let p = unsafe { qjs::JS_GetOpaque(obj, qjs::JS_GetClassID(obj)) } as *mut NodeData;
    if p.is_null() {
        return;
    }
    let data = unsafe { Box::from_raw(p) };
    if let Some(state) = crate::state::of_runtime(rt) {
        // SAFETY: a projection of the live state's renderer.
        let d = unsafe { &(*state).dom };
        let index = (data.id >> 1) as usize;
        if let Some(slot) = d.nodes.borrow_mut().get_mut(index)
            && *slot == p
        {
            *slot = ptr::null_mut();
        }
        if data.id != ROOT {
            // Not the op queue: a finalizer may run while it is borrowed.
            d.shared.releases.borrow_mut().push(data.id);
        }
    }
    for v in data.children {
        // SAFETY: values the node owned.
        unsafe { qjs::JS_FreeValueRT(rt, v) };
    }
    for (_, v) in data.handlers {
        unsafe { qjs::JS_FreeValueRT(rt, v) };
    }
}

unsafe extern "C" fn node_mark(rt: *mut qjs::JSRuntime, obj: JsValue, mark: qjs::JS_MarkFunc) {
    // SAFETY: as in the finalizer; marking reads only.
    let p = unsafe { qjs::JS_GetOpaque(obj, qjs::JS_GetClassID(obj)) } as *const NodeData;
    if p.is_null() {
        return;
    }
    let data = unsafe { &*p };
    for &v in &data.children {
        unsafe { qjs::JS_MarkValue(rt, v, mark) };
    }
    for &(_, v) in &data.handlers {
        unsafe { qjs::JS_MarkValue(rt, v, mark) };
    }
}

// --- The tree mirror ----------------------------------------------------------

/// The node after `child` in its parent (`None`: the last one, or not a
/// child).
fn next_sibling(ctx: *mut JsContext, child: JsValue) -> Option<JsValue> {
    let c = node_ptr(ctx, child);
    if c.is_null() {
        return None;
    }
    // SAFETY: live node data, read without other references to it.
    let parent = unsafe { (*c).parent };
    let p = dom(ctx).by_id(parent);
    if p.is_null() {
        return None;
    }
    let children = unsafe { &(*p).children };
    let i = children.iter().position(|&v| same(v, child))?;
    children.get(i + 1).copied()
}

/// Whether `node` is `of` or one of its ancestors.
fn is_ancestor(ctx: *mut JsContext, node: NodeId, of: NodeId) -> bool {
    let d = dom(ctx);
    let mut at = of;
    while at != DETACHED {
        if at == node {
            return true;
        }
        let p = d.by_id(at);
        if p.is_null() {
            return false;
        }
        // SAFETY: live node data.
        at = unsafe { (*p).parent };
    }
    false
}

/// Moves node `child` into `parent` before `before` (`None`: at the end).
fn insert_before(
    ctx: *mut JsContext,
    parent: JsValue,
    child: JsValue,
    before: Option<JsValue>,
) -> Result<(), JsValue> {
    let (p, c) = (node_ptr(ctx, parent), node_ptr(ctx, child));
    if p.is_null() || c.is_null() {
        return Err(throw_type(ctx, "nsh: not a node"));
    }
    if before.is_some_and(|b| same(b, child)) {
        return Ok(());
    }
    // Already there: no move.
    // SAFETY: live node data.
    if unsafe { (*c).parent == (*p).id } {
        let next = next_sibling(ctx, child);
        let in_place = match (next, before) {
            (None, None) => true,
            (Some(n), Some(b)) => same(n, b),
            _ => false,
        };
        if in_place {
            return Ok(());
        }
    }
    // SAFETY: live node data; `p` and `c` differ (no node is its own
    // parent: checked below), each reached briefly.
    let (parent_id, child_id) = unsafe { ((*p).id, (*c).id) };
    if is_ancestor(ctx, child_id, parent_id) {
        return Err(throw_type(
            ctx,
            "nsh: a node cannot be inserted into itself",
        ));
    }
    let d = dom(ctx);
    // Out of its current parent (the core moves the view itself).
    let mut held = None;
    let old = unsafe { (*c).parent };
    if old != DETACHED {
        let op = d.by_id(old);
        if !op.is_null() {
            let children = unsafe { &mut (*op).children };
            if let Some(i) = children.iter().position(|&v| same(v, child)) {
                held = Some(children.remove(i));
            }
        }
    }
    let children = unsafe { &mut (*p).children };
    let (pos, before_id) = match before {
        Some(b) => match children.iter().position(|&v| same(v, b)) {
            Some(i) => (i, Some(unsafe { (*node_ptr(ctx, b)).id })),
            None => (children.len(), None),
        },
        None => (children.len(), None),
    };
    // SAFETY: a dup the parent owns from now on.
    let owned = held.unwrap_or_else(|| unsafe { qjs::JS_DupValue(ctx, child) });
    children.insert(pos, owned);
    unsafe { (*c).parent = parent_id };
    d.push(
        ctx,
        Op::Insert {
            parent: parent_id,
            child: child_id,
            before: before_id,
        },
    )
}

/// Removes node `child` from `parent` (nothing when it is not a child).
fn remove_child(ctx: *mut JsContext, parent: JsValue, child: JsValue) -> Result<(), JsValue> {
    let (p, c) = (node_ptr(ctx, parent), node_ptr(ctx, child));
    if p.is_null() || c.is_null() {
        return Ok(());
    }
    // SAFETY: live node data, reached briefly.
    let (parent_id, child_id) = unsafe { ((*p).id, (*c).id) };
    if unsafe { (*c).parent } != parent_id {
        return Ok(());
    }
    let children = unsafe { &mut (*p).children };
    let Some(i) = children.iter().position(|&v| same(v, child)) else {
        return Ok(());
    };
    let owned = children.remove(i);
    unsafe { (*c).parent = DETACHED };
    let pushed = dom(ctx).push(
        ctx,
        Op::Remove {
            parent: parent_id,
            child: child_id,
        },
    );
    // Last: this may finalize the child.
    unsafe { qjs::JS_FreeValue(ctx, owned) };
    pushed
}

// --- Elements, text, props -----------------------------------------------------

/// `element(tag)`: a new host element.
unsafe extern "C" fn element(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let Some(name) = to_string(ctx, arg(argc, argv, 0)) else {
        return qjs::JS_EXCEPTION;
    };
    let tag = Tag::from_name(&name).unwrap_or_else(|| {
        dom(ctx)
            .shared
            .warnings
            .borrow_mut()
            .warn(format!("unknown element <{name}>, rendering it as <view>"));
        Tag::View
    });
    let id = next_id(ctx);
    if let Err(e) = dom(ctx).push(ctx, Op::Create { id, tag }) {
        return e;
    }
    new_node(ctx, id, false)
}

/// A new text node showing `text`.
fn text_node(ctx: *mut JsContext, text: String) -> Result<JsValue, JsValue> {
    let id = next_id(ctx);
    dom(ctx).push(ctx, Op::CreateText { id, text })?;
    Ok(new_node(ctx, id, true))
}

/// `text(value)`: a new text node.
unsafe extern "C" fn text(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let Some(s) = to_string(ctx, arg(argc, argv, 0)) else {
        return qjs::JS_EXCEPTION;
    };
    text_node(ctx, s).unwrap_or_else(|e| e)
}

/// `append(parent, child)`: a static child at the end.
unsafe extern "C" fn append(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let (parent, child) = (arg(argc, argv, 0), arg(argc, argv, 1));
    let result = if node_ptr(ctx, child).is_null() {
        insert_value(ctx, parent, child, None)
    } else {
        insert_before(ctx, parent, child, None)
    };
    result.err().unwrap_or(UNDEFINED)
}

/// Sets prop `name` of node `node` to `value` (all borrowed): an event
/// handler, the platform props, the ref, or a host prop the core decodes.
fn set_prop(
    ctx: *mut JsContext,
    node: JsValue,
    name: &str,
    value: JsValue,
    prev: JsValue,
) -> Result<(), JsValue> {
    let p = node_ptr(ctx, node);
    if p.is_null() {
        return Err(throw_type(ctx, "nsh: not a node"));
    }
    // SAFETY: live node data.
    let id = unsafe { (*p).id };
    match name {
        "children" => return Ok(()),
        "ref" => {
            // SAFETY: a borrowed function, called untracked with the node.
            if unsafe { qjs::JS_IsFunction(ctx, value) } {
                let mut args = [node];
                let r = reactive::untracked_with(ctx, || unsafe {
                    qjs::JS_Call(ctx, value, UNDEFINED, 1, args.as_mut_ptr())
                });
                if unsafe { qjs::JS_IsException(r) } {
                    return Err(r);
                }
                unsafe { qjs::JS_FreeValue(ctx, r) };
            }
            return Ok(());
        }
        "ios" | "android" | "desktop" => return platform_props(ctx, node, name, value, prev),
        _ => {}
    }
    if name.len() > 2
        && name.starts_with("on")
        && name.as_bytes()[2].is_ascii_uppercase()
        && let Some(event) = EventKind::from_prop(name)
    {
        // SAFETY: an inline check, then a dup the node owns.
        let handler = unsafe { qjs::JS_IsFunction(ctx, value) }
            .then(|| unsafe { qjs::JS_DupValue(ctx, value) });
        let handlers = unsafe { &mut (*p).handlers };
        let at = handlers.iter().position(|&(e, _)| e == event);
        let (was, old) = match (at, handler) {
            (Some(i), Some(h)) => (true, Some(std::mem::replace(&mut handlers[i].1, h))),
            (Some(i), None) => (true, Some(handlers.remove(i).1)),
            (None, Some(h)) => {
                handlers.push((event, h));
                (false, None)
            }
            (None, None) => (false, None),
        };
        let now = handler.is_some();
        let pushed = if was != now {
            dom(ctx).push(
                ctx,
                Op::Listen {
                    id,
                    event,
                    enabled: now,
                },
            )
        } else {
            Ok(())
        };
        if let Some(old) = old {
            unsafe { qjs::JS_FreeValue(ctx, old) };
        }
        return pushed;
    }
    // SAFETY: a context reference for the decoder, which reads `value`.
    let cx = unsafe { Ctx::from_raw(std::ptr::NonNull::new_unchecked(ctx)) };
    let v = unsafe { Value::from_raw(cx, qjs::JS_DupValue(ctx, value)) };
    // Decoding may run JS (getters): no borrow is held meanwhile.
    let mut warnings = crate::protocol::Warnings::deferred();
    let decoded = crate::protocol::decode_prop(id, name, v, &mut warnings);
    dom(ctx).shared.warnings.borrow_mut().merge(warnings);
    match decoded {
        Ok(Some(op)) => dom(ctx).push(ctx, op),
        Ok(None) => Ok(()),
        Err(rquickjs::Error::Exception) => Err(qjs::JS_EXCEPTION),
        Err(e) => Err(throw_type(ctx, &format!("nsh: prop `{name}`: {e}"))),
    }
}

/// `ios={{…}}` / `android={{…}}` / `desktop={{…}}`: the props of this
/// platform only, each set (or cleared) on its own.
fn platform_props(
    ctx: *mut JsContext,
    node: JsValue,
    name: &str,
    value: JsValue,
    prev: JsValue,
) -> Result<(), JsValue> {
    let os = dom(ctx).os.get();
    let desktop = matches!(os, Os::MacOs | Os::Windows | Os::Linux);
    let applies = match name {
        "desktop" => desktop,
        other => other == os.name(),
    };
    if !applies {
        return Ok(());
    }
    // SAFETY: property reads of live values; every value got is freed.
    unsafe {
        let keys = |obj: JsValue| -> Vec<String> {
            let mut out = Vec::new();
            if !qjs::JS_IsObject(obj) {
                return out;
            }
            let mut tab: *mut qjs::JSPropertyEnum = ptr::null_mut();
            let mut len: u32 = 0;
            if qjs::JS_GetOwnPropertyNames(
                ctx,
                &mut tab,
                &mut len,
                obj,
                (qjs::JS_GPN_STRING_MASK | qjs::JS_GPN_ENUM_ONLY) as c_int,
            ) != 0
            {
                return out;
            }
            for i in 0..len as usize {
                let atom = (*tab.add(i)).atom;
                let s = qjs::JS_AtomToCStringLen(ctx, std::ptr::null_mut(), atom);
                if !s.is_null() {
                    out.push(CStr::from_ptr(s).to_string_lossy().into_owned());
                    qjs::JS_FreeCString(ctx, s);
                }
            }
            qjs::JS_FreePropertyEnum(ctx, tab, len);
            out
        };
        let get = |obj: JsValue, key: &str| -> JsValue {
            if !qjs::JS_IsObject(obj) {
                return UNDEFINED;
            }
            let k = std::ffi::CString::new(key).unwrap_or_default();
            qjs::JS_GetPropertyStr(ctx, obj, k.as_ptr())
        };
        let next_keys = keys(value);
        for key in keys(prev) {
            if !next_keys.contains(&key) {
                let old = get(prev, &key);
                let r = set_prop(ctx, node, &key, UNDEFINED, old);
                qjs::JS_FreeValue(ctx, old);
                r?;
            }
        }
        for key in next_keys {
            let (v, old) = (get(value, &key), get(prev, &key));
            let r = set_prop(ctx, node, &key, v, old);
            qjs::JS_FreeValue(ctx, v);
            qjs::JS_FreeValue(ctx, old);
            r?;
        }
    }
    Ok(())
}

/// `prop(node, name, value)`: a static prop.
unsafe extern "C" fn prop(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let Some(name) = to_string(ctx, arg(argc, argv, 1)) else {
        return qjs::JS_EXCEPTION;
    };
    set_prop(
        ctx,
        arg(argc, argv, 0),
        &name,
        arg(argc, argv, 2),
        UNDEFINED,
    )
    .err()
    .unwrap_or(UNDEFINED)
}

/// `bindProp(node, name, get)`: a prop following `get()`.
unsafe extern "C" fn bind_prop(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let mut data = [arg(argc, argv, 0), arg(argc, argv, 1), arg(argc, argv, 2)];
    // SAFETY: the function keeps dups of `data`.
    let func =
        unsafe { qjs::JS_NewCFunctionData(ctx, Some(bind_prop_run), 1, 0, 3, data.as_mut_ptr()) };
    let r = reactive::render_effect(ctx, func);
    unsafe { qjs::JS_FreeValue(ctx, r) };
    reactive::finish_value(ctx, UNDEFINED)
}

/// A run of a bound prop: `prev` in, the new value out (kept by the graph).
unsafe extern "C" fn bind_prop_run(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: three data values (`bind_prop`).
    let (node, name, get) = unsafe { (*data, *data.add(1), *data.add(2)) };
    let prev = arg(argc, argv, 0);
    let next = unsafe { qjs::JS_Call(ctx, get, UNDEFINED, 0, ptr::null_mut()) };
    if unsafe { qjs::JS_IsException(next) } {
        return next;
    }
    if unsafe { qjs::JS_IsStrictEqual(ctx, prev, next) } {
        return next;
    }
    let Some(name) = to_string(ctx, name) else {
        unsafe { qjs::JS_FreeValue(ctx, next) };
        return qjs::JS_EXCEPTION;
    };
    match set_prop(ctx, node, &name, next, prev) {
        Ok(()) => next,
        Err(e) => {
            unsafe { qjs::JS_FreeValue(ctx, next) };
            e
        }
    }
}

/// `spread(node, props)`: every prop of `props` (an object, possibly with
/// getters), followed.
unsafe extern "C" fn spread(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let mut data = [arg(argc, argv, 0), arg(argc, argv, 1)];
    // SAFETY: the function keeps dups of `data`.
    let func =
        unsafe { qjs::JS_NewCFunctionData(ctx, Some(spread_run), 1, 0, 2, data.as_mut_ptr()) };
    let r = reactive::render_effect(ctx, func);
    unsafe { qjs::JS_FreeValue(ctx, r) };
    reactive::finish_value(ctx, UNDEFINED)
}

/// A run of a spread: the previous values (an object) in, the new ones
/// out.
unsafe extern "C" fn spread_run(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: two data values (`spread`); every value got is freed.
    unsafe {
        let (node, props) = (*data, *data.add(1));
        let prev = arg(argc, argv, 0);
        let next = qjs::JS_NewObject(ctx);
        let mut tab: *mut qjs::JSPropertyEnum = ptr::null_mut();
        let mut len: u32 = 0;
        if !qjs::JS_IsObject(props)
            || qjs::JS_GetOwnPropertyNames(
                ctx,
                &mut tab,
                &mut len,
                props,
                (qjs::JS_GPN_STRING_MASK | qjs::JS_GPN_ENUM_ONLY) as c_int,
            ) != 0
        {
            return next;
        }
        let mut failed = None;
        for i in 0..len as usize {
            let atom = (*tab.add(i)).atom;
            let v = qjs::JS_GetProperty(ctx, props, atom);
            if qjs::JS_IsException(v) {
                failed = Some(v);
                break;
            }
            let old = if qjs::JS_IsObject(prev) {
                qjs::JS_GetProperty(ctx, prev, atom)
            } else {
                UNDEFINED
            };
            let name = qjs::JS_AtomToCStringLen(ctx, std::ptr::null_mut(), atom);
            if name.is_null() {
                qjs::JS_FreeValue(ctx, old);
                qjs::JS_FreeValue(ctx, v);
                failed = Some(qjs::JS_EXCEPTION);
                break;
            }
            let name_str = CStr::from_ptr(name).to_string_lossy().into_owned();
            qjs::JS_FreeCString(ctx, name);
            let r = if qjs::JS_IsStrictEqual(ctx, old, v) {
                Ok(())
            } else {
                set_prop(ctx, node, &name_str, v, old)
            };
            qjs::JS_FreeValue(ctx, old);
            qjs::JS_SetProperty(ctx, next, atom, v);
            if let Err(e) = r {
                failed = Some(e);
                break;
            }
        }
        qjs::JS_FreePropertyEnum(ctx, tab, len);
        match failed {
            Some(e) => {
                qjs::JS_FreeValue(ctx, next);
                e
            }
            None => next,
        }
    }
}

// --- Children -------------------------------------------------------------------

/// What a child value renders as.
enum Item {
    Node(JsValue),
    Text(String),
}

/// Collects what `value` (borrowed) renders as into `out` (node values
/// borrowed from `keep`, which owns what calls returned). Functions are
/// called (tracked by the running computation).
fn resolve(
    ctx: *mut JsContext,
    value: JsValue,
    out: &mut Vec<Item>,
    keep: &mut Vec<JsValue>,
    depth: u32,
) -> Result<(), JsValue> {
    if depth > MAX_DEPTH {
        return Err(throw_type(ctx, "nsh: children nested too deeply"));
    }
    // SAFETY: inline tag checks and reads of live values; returned values
    // are kept in `keep` (freed by the caller).
    unsafe {
        let tag = qjs::JS_VALUE_GET_TAG(value);
        if tag == qjs::JS_TAG_UNDEFINED || tag == qjs::JS_TAG_NULL || tag == qjs::JS_TAG_BOOL {
            return Ok(());
        }
        if qjs::JS_IsString(value) || qjs::JS_IsNumber(value) || tag == qjs::JS_TAG_BIG_INT {
            let Some(s) = to_string(ctx, value) else {
                return Err(qjs::JS_EXCEPTION);
            };
            out.push(Item::Text(s));
            return Ok(());
        }
        if !node_ptr(ctx, value).is_null() {
            out.push(Item::Node(value));
            return Ok(());
        }
        if qjs::JS_IsFunction(ctx, value) {
            let r = qjs::JS_Call(ctx, value, UNDEFINED, 0, ptr::null_mut());
            if qjs::JS_IsException(r) {
                return Err(r);
            }
            keep.push(r);
            return resolve(ctx, r, out, keep, depth + 1);
        }
        if qjs::JS_IsArray(value) {
            let mut len: i64 = 0;
            if qjs::JS_GetLength(ctx, value, &mut len) != 0 {
                return Err(qjs::JS_EXCEPTION);
            }
            for i in 0..len.max(0) as u32 {
                let item = qjs::JS_GetPropertyUint32(ctx, value, i);
                if qjs::JS_IsException(item) {
                    return Err(item);
                }
                keep.push(item);
                resolve(ctx, item, out, keep, depth + 1)?;
            }
            return Ok(());
        }
    }
    dom(ctx)
        .shared
        .warnings
        .borrow_mut()
        .warn("a child that is not an element, text, array or function is not rendered".into());
    Ok(())
}

/// Turns items into node values (owned): text becomes new text nodes,
/// except a lone text over a lone text node `reuse`, which is updated.
fn materialize(
    ctx: *mut JsContext,
    items: Vec<Item>,
    reuse: Option<JsValue>,
) -> Result<Vec<JsValue>, JsValue> {
    let reuse = reuse.filter(|&n| {
        let p = node_ptr(ctx, n);
        // SAFETY: live node data.
        !p.is_null() && unsafe { (*p).text }
    });
    if let (Some(node), [Item::Text(_)]) = (reuse, items.as_slice()) {
        let Some(Item::Text(text)) = items.into_iter().next() else {
            unreachable!()
        };
        // SAFETY: a live text node's data.
        let id = unsafe { (*node_ptr(ctx, node)).id };
        dom(ctx).push(ctx, Op::SetText { id, text })?;
        return Ok(vec![unsafe { qjs::JS_DupValue(ctx, node) }]);
    }
    let mut nodes = Vec::with_capacity(items.len());
    for item in items {
        match item {
            // SAFETY: a dup of a live node.
            Item::Node(v) => nodes.push(unsafe { qjs::JS_DupValue(ctx, v) }),
            Item::Text(text) => match text_node(ctx, text) {
                Ok(v) => nodes.push(v),
                Err(e) => {
                    free_values(ctx, nodes);
                    return Err(e);
                }
            },
        }
    }
    Ok(nodes)
}

fn free_values(ctx: *mut JsContext, values: Vec<JsValue>) {
    for v in values {
        // SAFETY: owned values.
        unsafe { qjs::JS_FreeValue(ctx, v) };
    }
}

/// Whether `value` holds a function (directly or in nested arrays): its
/// children change over time.
fn is_dynamic(ctx: *mut JsContext, value: JsValue, depth: u32) -> bool {
    // SAFETY: inline checks and reads of live values, each freed.
    unsafe {
        if qjs::JS_IsFunction(ctx, value) {
            return true;
        }
        if depth > MAX_DEPTH || !qjs::JS_IsArray(value) {
            return false;
        }
        let mut len: i64 = 0;
        if qjs::JS_GetLength(ctx, value, &mut len) != 0 {
            qjs::JS_FreeValue(ctx, qjs::JS_GetException(ctx));
            return false;
        }
        for i in 0..len.max(0) as u32 {
            let item = qjs::JS_GetPropertyUint32(ctx, value, i);
            let dynamic = is_dynamic(ctx, item, depth + 1);
            qjs::JS_FreeValue(ctx, item);
            if dynamic {
                return true;
            }
        }
    }
    false
}

/// Inserts `value` (borrowed) into `parent` before `marker`: once for a
/// static value, kept up to date by a render effect for a dynamic one.
fn insert_value(
    ctx: *mut JsContext,
    parent: JsValue,
    value: JsValue,
    marker: Option<JsValue>,
) -> Result<(), JsValue> {
    if is_dynamic(ctx, value, 0) {
        let mut data = [parent, marker.unwrap_or(UNDEFINED), value];
        // SAFETY: the function keeps dups of `data`.
        let func =
            unsafe { qjs::JS_NewCFunctionData(ctx, Some(insert_run), 1, 0, 3, data.as_mut_ptr()) };
        let r = reactive::render_effect(ctx, func);
        unsafe { qjs::JS_FreeValue(ctx, r) };
        // A throw in the effect is the round's error.
        let r = reactive::finish_value(ctx, UNDEFINED);
        return if unsafe { qjs::JS_IsException(r) } {
            Err(r)
        } else {
            Ok(())
        };
    }
    let mut items = Vec::new();
    let mut keep = Vec::new();
    let resolved = resolve(ctx, value, &mut items, &mut keep, 0);
    let result = resolved.and_then(|()| {
        let nodes = materialize(ctx, items, None)?;
        let mut result = Ok(());
        for &n in &nodes {
            result = insert_before(ctx, parent, n, marker);
            if result.is_err() {
                break;
            }
        }
        free_values(ctx, nodes);
        result
    });
    free_values(ctx, keep);
    result
}

/// `insert(parent, value, marker)`: children before `marker` (or at the
/// end).
unsafe extern "C" fn insert(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let marker = arg(argc, argv, 2);
    let marker = (!node_ptr(ctx, marker).is_null()).then_some(marker);
    insert_value(ctx, arg(argc, argv, 0), arg(argc, argv, 1), marker)
        .err()
        .unwrap_or(UNDEFINED)
}

/// The values of array `array` (owned), empty for other values.
fn array_items(ctx: *mut JsContext, array: JsValue) -> Vec<JsValue> {
    let mut out = Vec::new();
    // SAFETY: reads of a live array.
    unsafe {
        if !qjs::JS_IsArray(array) {
            return out;
        }
        let mut len: i64 = 0;
        if qjs::JS_GetLength(ctx, array, &mut len) != 0 {
            qjs::JS_FreeValue(ctx, qjs::JS_GetException(ctx));
            return out;
        }
        for i in 0..len.max(0) as u32 {
            out.push(qjs::JS_GetPropertyUint32(ctx, array, i));
        }
    }
    out
}

/// A new array of `values` (taken).
fn new_array(ctx: *mut JsContext, mut values: Vec<JsValue>) -> JsValue {
    // SAFETY: the array takes the values.
    unsafe { qjs::JS_NewArrayFrom(ctx, values.len() as c_int, values.as_mut_ptr()) }
}

/// A run of a dynamic insert: the nodes inserted last time (an array) in,
/// the nodes inserted now out.
unsafe extern "C" fn insert_run(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: three data values (`insert_value`).
    let (parent, marker, value) = unsafe { (*data, *data.add(1), *data.add(2)) };
    let marker = (!node_ptr(ctx, marker).is_null()).then_some(marker);
    let old = array_items(ctx, arg(argc, argv, 0));
    let mut items = Vec::new();
    let mut keep = Vec::new();
    if let Err(e) = resolve(ctx, value, &mut items, &mut keep, 0) {
        free_values(ctx, keep);
        free_values(ctx, old);
        return e;
    }
    // A lone text node shown again as text is updated in place.
    let reuse = match old.as_slice() {
        [only] => Some(*only),
        _ => None,
    };
    let lone_text = matches!(items.as_slice(), [Item::Text(_)]);
    let new = match materialize(ctx, items, reuse.filter(|_| lone_text)) {
        Ok(new) => new,
        Err(e) => {
            free_values(ctx, keep);
            free_values(ctx, old);
            return e;
        }
    };
    free_values(ctx, keep);
    let result = reconcile(ctx, parent, &old, &new, marker);
    // The old nodes are freed after the reconcile: a node it removes stays
    // alive until then.
    free_values(ctx, old);
    match result {
        Ok(()) => new_array(ctx, new),
        Err(e) => {
            free_values(ctx, new);
            e
        }
    }
}

/// Solid's `reconcileArrays`: changes `parent`'s children from `old` to
/// `new` (the nodes of one insert, before `marker`) with few moves.
fn reconcile(
    ctx: *mut JsContext,
    parent: JsValue,
    old: &[JsValue],
    new: &[JsValue],
    marker: Option<JsValue>,
) -> Result<(), JsValue> {
    let mut a: Vec<JsValue> = old.to_vec();
    let b = new;
    let b_len = b.len();
    let (mut a_start, mut b_start) = (0usize, 0usize);
    let (mut a_end, mut b_end) = (a.len(), b_len);
    let after = match a.last() {
        Some(&last) => next_sibling(ctx, last).or(marker),
        None => marker,
    };
    let mut map: Option<FxHashMap<usize, usize>> = None;
    while a_start < a_end || b_start < b_end {
        if a_start < a_end && b_start < b_end && same(a[a_start], b[b_start]) {
            a_start += 1;
            b_start += 1;
            continue;
        }
        while a_end > a_start && b_end > b_start && same(a[a_end - 1], b[b_end - 1]) {
            a_end -= 1;
            b_end -= 1;
        }
        if a_end == a_start {
            // Appends.
            let node = if b_end < b_len {
                if b_start > 0 {
                    next_sibling(ctx, b[b_start - 1])
                } else {
                    Some(b[b_end - b_start])
                }
            } else {
                after
            };
            while b_start < b_end {
                insert_before(ctx, parent, b[b_start], node)?;
                b_start += 1;
            }
        } else if b_end == b_start {
            // Removals.
            while a_start < a_end {
                if map
                    .as_ref()
                    .is_none_or(|m| !m.contains_key(&ptr_of(a[a_start])))
                {
                    remove_child(ctx, parent, a[a_start])?;
                }
                a_start += 1;
            }
        } else if same(a[a_start], b[b_end - 1]) && same(b[b_start], a[a_end - 1]) {
            // A swap.
            a_end -= 1;
            let node = next_sibling(ctx, a[a_end]);
            let first_next = next_sibling(ctx, a[a_start]);
            insert_before(ctx, parent, b[b_start], first_next)?;
            b_start += 1;
            a_start += 1;
            b_end -= 1;
            insert_before(ctx, parent, b[b_end], node)?;
            a[a_end] = b[b_end];
        } else {
            let map =
                map.get_or_insert_with(|| (b_start..b_end).map(|i| (ptr_of(b[i]), i)).collect());
            match map.get(&ptr_of(a[a_start])).copied() {
                Some(index) if b_start < index && index < b_end => {
                    let mut i = a_start;
                    let mut sequence = 1;
                    loop {
                        i += 1;
                        if !(i < a_end && i < b_end) {
                            break;
                        }
                        match map.get(&ptr_of(a[i])) {
                            Some(&t) if t == index + sequence => sequence += 1,
                            _ => break,
                        }
                    }
                    if sequence > index - b_start {
                        let node = a[a_start];
                        while b_start < index {
                            insert_before(ctx, parent, b[b_start], Some(node))?;
                            b_start += 1;
                        }
                    } else {
                        // Replaces a[a_start] with b[b_start]; a node that
                        // comes again later stays (it is moved then, not
                        // removed and inserted: the core would remake its
                        // view).
                        insert_before(ctx, parent, b[b_start], Some(a[a_start]))?;
                        if !map.contains_key(&ptr_of(a[a_start])) {
                            remove_child(ctx, parent, a[a_start])?;
                        }
                        b_start += 1;
                        a_start += 1;
                    }
                }
                Some(_) => a_start += 1,
                None => {
                    remove_child(ctx, parent, a[a_start])?;
                    a_start += 1;
                }
            }
        }
    }
    Ok(())
}

// --- Components and control flow ------------------------------------------------

/// `merge(...sources)`: a new object with the own enumerable properties of
/// each source, later ones winning; getters stay getters (props spread
/// into a component stay live).
unsafe extern "C" fn merge(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    // SAFETY: property reads and definitions on live objects; descriptor
    // values are freed after use.
    unsafe {
        let target = qjs::JS_NewObject(ctx);
        for i in 0..argc.max(0) as usize {
            let source = *argv.add(i);
            if !qjs::JS_IsObject(source) {
                continue;
            }
            let mut tab: *mut qjs::JSPropertyEnum = ptr::null_mut();
            let mut len: u32 = 0;
            if qjs::JS_GetOwnPropertyNames(
                ctx,
                &mut tab,
                &mut len,
                source,
                (qjs::JS_GPN_STRING_MASK | qjs::JS_GPN_SYMBOL_MASK | qjs::JS_GPN_ENUM_ONLY)
                    as c_int,
            ) != 0
            {
                qjs::JS_FreeValue(ctx, target);
                return qjs::JS_EXCEPTION;
            }
            for j in 0..len as usize {
                let atom = (*tab.add(j)).atom;
                let mut desc = qjs::JSPropertyDescriptor {
                    flags: 0,
                    value: UNDEFINED,
                    getter: UNDEFINED,
                    setter: UNDEFINED,
                };
                if qjs::JS_GetOwnProperty(ctx, &mut desc, source, atom) <= 0 {
                    continue;
                }
                let flags = if desc.flags & qjs::JS_PROP_GETSET as c_int != 0 {
                    qjs::JS_PROP_HAS_GET | qjs::JS_PROP_HAS_SET
                } else {
                    qjs::JS_PROP_HAS_VALUE | qjs::JS_PROP_HAS_WRITABLE | qjs::JS_PROP_WRITABLE
                } | qjs::JS_PROP_HAS_ENUMERABLE
                    | qjs::JS_PROP_ENUMERABLE
                    | qjs::JS_PROP_HAS_CONFIGURABLE
                    | qjs::JS_PROP_CONFIGURABLE;
                qjs::JS_DefineProperty(
                    ctx,
                    target,
                    atom,
                    desc.value,
                    desc.getter,
                    desc.setter,
                    flags as c_int,
                );
                qjs::JS_FreeValue(ctx, desc.value);
                qjs::JS_FreeValue(ctx, desc.getter);
                qjs::JS_FreeValue(ctx, desc.setter);
            }
            qjs::JS_FreePropertyEnum(ctx, tab, len);
        }
        target
    }
}

/// `component(Comp, props)`: renders a component (untracked).
unsafe extern "C" fn component(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let (comp, props) = (arg(argc, argv, 0), arg(argc, argv, 1));
    let mut args = [props];
    // SAFETY: a borrowed function and argument.
    reactive::untracked_with(ctx, || unsafe {
        qjs::JS_Call(ctx, comp, UNDEFINED, 1, args.as_mut_ptr())
    })
}

/// `cond(when, then, otherwise)`: what `then()` renders while `when()` is
/// truthy, else what `otherwise()` renders (each branch rendered once per
/// change of truthiness, under its own owner).
unsafe extern "C" fn cond(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    // SAFETY: dups the memos and functions own.
    let when = unsafe { qjs::JS_DupValue(ctx, arg(argc, argv, 0)) };
    let condition = reactive::memo(ctx, when, true);
    let mut data = [condition, arg(argc, argv, 1), arg(argc, argv, 2)];
    let func = unsafe { qjs::JS_NewCFunctionData(ctx, Some(cond_run), 0, 0, 3, data.as_mut_ptr()) };
    unsafe { qjs::JS_FreeValue(ctx, condition) };
    let branch = reactive::memo(ctx, func, false);
    reactive::finish_value(ctx, branch)
}

unsafe extern "C" fn cond_run(
    ctx: *mut JsContext,
    _this: JsValue,
    _argc: c_int,
    _argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: three data values (`cond`).
    let (condition, then, otherwise) = unsafe { (*data, *data.add(1), *data.add(2)) };
    let c = unsafe { qjs::JS_Call(ctx, condition, UNDEFINED, 0, ptr::null_mut()) };
    if unsafe { qjs::JS_IsException(c) } {
        return c;
    }
    let truthy = unsafe { qjs::JS_ToBool(ctx, c) } == 1;
    unsafe { qjs::JS_FreeValue(ctx, c) };
    let branch = if truthy { then } else { otherwise };
    if !unsafe { qjs::JS_IsFunction(ctx, branch) } {
        return UNDEFINED;
    }
    reactive::untracked_with(ctx, || unsafe {
        qjs::JS_Call(ctx, branch, UNDEFINED, 0, ptr::null_mut())
    })
}

/// A key rows are matched by: objects by identity, primitives by value.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    Object(usize),
    Text(Box<str>),
    Number(u64),
    Other(i32, i64),
}

impl Key {
    fn of(ctx: *mut JsContext, v: JsValue) -> Key {
        // SAFETY: inline reads of a live value.
        unsafe {
            let tag = qjs::JS_VALUE_GET_TAG(v);
            if tag == qjs::JS_TAG_OBJECT {
                Key::Object(ptr_of(v))
            } else if qjs::JS_IsString(v) {
                Key::Text(to_string(ctx, v).unwrap_or_default().into())
            } else if qjs::JS_IsNumber(v) {
                let mut n = 0f64;
                qjs::JS_ToFloat64(ctx, &mut n, v);
                // SameValueZero: -0 is 0, NaN is NaN.
                let n = if n == 0.0 {
                    0.0
                } else if n.is_nan() {
                    f64::NAN
                } else {
                    n
                };
                Key::Number(n.to_bits())
            } else {
                Key::Other(tag, qjs::JS_VALUE_GET_INT(v) as i64)
            }
        }
    }
}

/// A row of `each`.
struct Row {
    key: Key,
    /// The key value (owned: an object key stays alive while it names the
    /// row).
    key_value: JsValue,
    /// Signals of the row's item and index (owned).
    item: JsValue,
    index: JsValue,
    /// The row's owner.
    owner: u32,
    /// What the row rendered (owned).
    output: JsValue,
}

#[derive(Default)]
struct EachData {
    rows: Vec<Row>,
}

unsafe extern "C" fn each_finalize(rt: *mut qjs::JSRuntime, obj: JsValue) {
    // SAFETY: the opaque of an each object is its boxed data.
    let p = unsafe { qjs::JS_GetOpaque(obj, qjs::JS_GetClassID(obj)) } as *mut EachData;
    if p.is_null() {
        return;
    }
    let data = unsafe { Box::from_raw(p) };
    for row in data.rows {
        for v in [row.key_value, row.item, row.index, row.output] {
            unsafe { qjs::JS_FreeValueRT(rt, v) };
        }
    }
}

unsafe extern "C" fn each_mark(rt: *mut qjs::JSRuntime, obj: JsValue, mark: qjs::JS_MarkFunc) {
    // SAFETY: as in the finalizer; marking reads only.
    let p = unsafe { qjs::JS_GetOpaque(obj, qjs::JS_GetClassID(obj)) } as *const EachData;
    if p.is_null() {
        return;
    }
    for row in unsafe { &(*p).rows } {
        for v in [row.key_value, row.item, row.index, row.output] {
            unsafe { qjs::JS_MarkValue(rt, v, mark) };
        }
    }
}

/// `each(list, render, key)`: `render(item, index)` for each item of
/// `list()` (both accessors), one row per key (`key(item)`, or the item
/// itself) kept while the key stays.
unsafe extern "C" fn each(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    // SAFETY: a new object of the each class owning its boxed data.
    let rows = unsafe {
        let obj = qjs::JS_NewObjectClass(ctx, dom(ctx).classes.each as _);
        qjs::JS_SetOpaque(
            obj,
            Box::into_raw(Box::<EachData>::default()) as *mut c_void,
        );
        obj
    };
    // The rows go with the owner `each` is called under.
    let mut cleanup_data = [rows];
    let cleanup = unsafe {
        qjs::JS_NewCFunctionData(ctx, Some(each_dispose), 0, 0, 1, cleanup_data.as_mut_ptr())
    };
    let mut args = [cleanup];
    let r = unsafe { reactive::on_cleanup(ctx, UNDEFINED, 1, args.as_mut_ptr()) };
    unsafe {
        qjs::JS_FreeValue(ctx, r);
        qjs::JS_FreeValue(ctx, cleanup);
    }
    let mut data = [
        arg(argc, argv, 0),
        arg(argc, argv, 1),
        arg(argc, argv, 2),
        rows,
    ];
    let func = unsafe { qjs::JS_NewCFunctionData(ctx, Some(each_run), 0, 0, 4, data.as_mut_ptr()) };
    unsafe { qjs::JS_FreeValue(ctx, rows) };
    let memo = reactive::memo(ctx, func, false);
    reactive::finish_value(ctx, memo)
}

fn each_data(ctx: *mut JsContext, rows: JsValue) -> *mut EachData {
    // SAFETY: the opaque of an each object.
    unsafe { qjs::JS_GetOpaque(rows, dom(ctx).classes.each) as *mut EachData }
}

/// Disposes a row (taken).
fn dispose_row(ctx: *mut JsContext, row: Row) {
    reactive::dispose_owner(ctx, row.owner);
    for v in [row.key_value, row.item, row.index, row.output] {
        // SAFETY: values the row owned.
        unsafe { qjs::JS_FreeValue(ctx, v) };
    }
}

unsafe extern "C" fn each_dispose(
    ctx: *mut JsContext,
    _this: JsValue,
    _argc: c_int,
    _argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: one data value (`each`).
    let p = each_data(ctx, unsafe { *data });
    if !p.is_null() {
        let rows = std::mem::take(unsafe { &mut (*p).rows });
        for row in rows {
            dispose_row(ctx, row);
        }
    }
    UNDEFINED
}

unsafe extern "C" fn each_run(
    ctx: *mut JsContext,
    _this: JsValue,
    _argc: c_int,
    _argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: four data values (`each`).
    let (list, render, key_fn, rows) = unsafe { (*data, *data.add(1), *data.add(2), *data.add(3)) };
    let items = unsafe { qjs::JS_Call(ctx, list, UNDEFINED, 0, ptr::null_mut()) };
    if unsafe { qjs::JS_IsException(items) } {
        return items;
    }
    let items_v = array_items(ctx, items);
    unsafe { qjs::JS_FreeValue(ctx, items) };
    let keyed = unsafe { qjs::JS_IsFunction(ctx, key_fn) };
    let result = reactive::untracked_with(ctx, || {
        let p = each_data(ctx, rows);
        let old = std::mem::take(unsafe { &mut (*p).rows });
        let mut old: Vec<Option<Row>> = old.into_iter().map(Some).collect();
        let mut by_key: FxHashMap<Key, Vec<usize>> = FxHashMap::default();
        for (i, row) in old.iter().enumerate().rev() {
            let row = row.as_ref().expect("all present");
            by_key.entry(row.key.clone()).or_default().push(i);
        }
        let mut new_rows = Vec::with_capacity(items_v.len());
        let mut failed = None;
        for (i, &item) in items_v.iter().enumerate() {
            // SAFETY: calls of borrowed functions with live values; the
            // values got are owned by the row or freed.
            let key_value = if keyed {
                let mut args = [item];
                let k = unsafe { qjs::JS_Call(ctx, key_fn, UNDEFINED, 1, args.as_mut_ptr()) };
                if unsafe { qjs::JS_IsException(k) } {
                    failed = Some(k);
                    break;
                }
                k
            } else {
                unsafe { qjs::JS_DupValue(ctx, item) }
            };
            let key = Key::of(ctx, key_value);
            let reused = by_key
                .get_mut(&key)
                .and_then(|v| v.pop())
                .and_then(|at| old[at].take());
            match reused {
                Some(row) => {
                    unsafe { qjs::JS_FreeValue(ctx, key_value) };
                    if !unsafe { qjs::JS_IsStrictEqual(ctx, reactive::peek(ctx, row.item), item) } {
                        reactive::set_signal(ctx, row.item, unsafe { qjs::JS_DupValue(ctx, item) });
                    }
                    let index = qjs::JS_NewFloat64(i as f64);
                    if !unsafe { qjs::JS_IsStrictEqual(ctx, reactive::peek(ctx, row.index), index) }
                    {
                        reactive::set_signal(ctx, row.index, index);
                    }
                    new_rows.push(row);
                }
                None => {
                    let item_signal = reactive::new_signal(ctx, item);
                    let index_signal = reactive::new_signal(ctx, qjs::JS_NewFloat64(i as f64));
                    let (owner, output) = reactive::with_root(ctx, || {
                        let mut args = [item_signal, index_signal];
                        unsafe { qjs::JS_Call(ctx, render, UNDEFINED, 2, args.as_mut_ptr()) }
                    });
                    let row = Row {
                        key,
                        key_value,
                        item: item_signal,
                        index: index_signal,
                        owner,
                        output: UNDEFINED,
                    };
                    if unsafe { qjs::JS_IsException(output) } {
                        dispose_row(ctx, row);
                        failed = Some(output);
                        break;
                    }
                    new_rows.push(Row { output, ..row });
                }
            }
        }
        for row in old.into_iter().flatten() {
            dispose_row(ctx, row);
        }
        let outputs: Vec<JsValue> = new_rows
            .iter()
            // SAFETY: dups for the returned array.
            .map(|r| unsafe { qjs::JS_DupValue(ctx, r.output) })
            .collect();
        let p = each_data(ctx, rows);
        unsafe { (*p).rows = new_rows };
        match failed {
            Some(e) => {
                free_values(ctx, outputs);
                e
            }
            None => new_array(ctx, outputs),
        }
    });
    free_values(ctx, items_v);
    result
}

// --- Context ----------------------------------------------------------------------

/// A context's default value (owned).
struct ContextData {
    default: JsValue,
}

unsafe extern "C" fn context_finalize(rt: *mut qjs::JSRuntime, obj: JsValue) {
    // SAFETY: the opaque of a context object is its boxed data.
    let p = unsafe { qjs::JS_GetOpaque(obj, qjs::JS_GetClassID(obj)) } as *mut ContextData;
    if !p.is_null() {
        let data = unsafe { Box::from_raw(p) };
        unsafe { qjs::JS_FreeValueRT(rt, data.default) };
    }
}

unsafe extern "C" fn context_mark(rt: *mut qjs::JSRuntime, obj: JsValue, mark: qjs::JS_MarkFunc) {
    // SAFETY: as in the finalizer.
    let p = unsafe { qjs::JS_GetOpaque(obj, qjs::JS_GetClassID(obj)) } as *const ContextData;
    if !p.is_null() {
        unsafe { qjs::JS_MarkValue(rt, (*p).default, mark) };
    }
}

/// Calling a context: its nearest provided value, else the default.
unsafe extern "C" fn context_call(
    ctx: *mut JsContext,
    func: JsValue,
    _this: JsValue,
    _argc: c_int,
    _argv: *mut JsValue,
    _flags: c_int,
) -> JsValue {
    if let Some(v) = reactive::lookup(ctx, func) {
        return v;
    }
    // SAFETY: the opaque of a context object.
    let p = unsafe { qjs::JS_GetOpaque(func, dom(ctx).classes.context) } as *const ContextData;
    if p.is_null() {
        return UNDEFINED;
    }
    unsafe { qjs::JS_DupValue(ctx, (*p).default) }
}

/// `context(default)`: a context, read by calling it; `<Ctx.Provider
/// value={…}>` provides a value to what it renders.
pub(crate) unsafe extern "C" fn context(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    // SAFETY: a new object of the context class owning its boxed data; the
    // provider function keeps a dup of it.
    unsafe {
        let obj = qjs::JS_NewObjectClass(ctx, dom(ctx).classes.context as _);
        let data = Box::new(ContextData {
            default: qjs::JS_DupValue(ctx, arg(argc, argv, 0)),
        });
        qjs::JS_SetOpaque(obj, Box::into_raw(data) as *mut c_void);
        let mut fdata = [obj];
        let provider = qjs::JS_NewCFunctionData(ctx, Some(provider), 1, 0, 1, fdata.as_mut_ptr());
        qjs::JS_SetPropertyStr(ctx, obj, c"Provider".as_ptr(), provider);
        obj
    }
}

/// A provider component: `{ value, children }`.
unsafe extern "C" fn provider(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: one data value (`context`); the function keeps dups.
    let mut fdata = [unsafe { *data }, arg(argc, argv, 0)];
    let func =
        unsafe { qjs::JS_NewCFunctionData(ctx, Some(provider_run), 0, 0, 2, fdata.as_mut_ptr()) };
    let rendered = reactive::render_effect(ctx, func);
    reactive::finish_value(ctx, rendered)
}

/// The provider's owner: sets the value, then renders the children in a
/// memo under it (so functions among them run with the value in reach).
unsafe extern "C" fn provider_run(
    ctx: *mut JsContext,
    _this: JsValue,
    _argc: c_int,
    _argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: two data values (`provider`); values got are freed.
    let (key, props) = unsafe { (*data, *data.add(1)) };
    reactive::untracked_with(ctx, || unsafe {
        let value = qjs::JS_GetPropertyStr(ctx, props, c"value".as_ptr());
        if qjs::JS_IsException(value) {
            return value;
        }
        reactive::provide(ctx, key, value);
        qjs::JS_FreeValue(ctx, value);
        let children = qjs::JS_GetPropertyStr(ctx, props, c"children".as_ptr());
        if qjs::JS_IsException(children) {
            return children;
        }
        let mut fdata = [children];
        let func = qjs::JS_NewCFunctionData(ctx, Some(children_run), 0, 0, 1, fdata.as_mut_ptr());
        qjs::JS_FreeValue(ctx, children);
        reactive::memo(ctx, func, false)
    })
}

/// Unwraps children: functions called (tracked), arrays flattened.
unsafe extern "C" fn children_run(
    ctx: *mut JsContext,
    _this: JsValue,
    _argc: c_int,
    _argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: one data value.
    let children = unsafe { *data };
    let mut items = Vec::new();
    let mut keep = Vec::new();
    let resolved = resolve(ctx, children, &mut items, &mut keep, 0);
    // Text stays text (the insert makes its nodes); nodes are kept.
    let out = resolved.map(|()| {
        items
            .into_iter()
            .map(|item| match item {
                // SAFETY: a dup for the array.
                Item::Node(v) => unsafe { qjs::JS_DupValue(ctx, v) },
                Item::Text(t) => unsafe {
                    qjs::JS_NewStringLen(ctx, t.as_ptr().cast(), t.len() as _)
                },
            })
            .collect::<Vec<_>>()
    });
    free_values(ctx, keep);
    match out {
        Ok(values) => new_array(ctx, values),
        Err(e) => e,
    }
}

// --- Mounting -----------------------------------------------------------------------

/// `mount(App, props)`: renders `App` into the surface; returns a function
/// that removes it.
pub(crate) unsafe extern "C" fn mount(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let d = dom(ctx);
    if d.rtl.get().is_none() {
        d.send_direction();
    }
    let root = d.root.get();
    // SAFETY: reads of the root's children; dups for the dispose function.
    let before: Vec<JsValue> = unsafe { &(*node_ptr(ctx, root)).children }
        .iter()
        .map(|&v| unsafe { qjs::JS_DupValue(ctx, v) })
        .collect();
    let (app, props) = (arg(argc, argv, 0), arg(argc, argv, 1));
    let (owner, result) = reactive::with_root(ctx, || {
        let props = if unsafe { qjs::JS_IsObject(props) } {
            unsafe { qjs::JS_DupValue(ctx, props) }
        } else {
            unsafe { qjs::JS_NewObject(ctx) }
        };
        let mut args = [props];
        let out = reactive::untracked_with(ctx, || unsafe {
            qjs::JS_Call(ctx, app, UNDEFINED, 1, args.as_mut_ptr())
        });
        unsafe { qjs::JS_FreeValue(ctx, props) };
        if unsafe { qjs::JS_IsException(out) } {
            return Err(out);
        }
        let r = insert_value(ctx, root, out, None);
        unsafe { qjs::JS_FreeValue(ctx, out) };
        r
    });
    if let Err(e) = result {
        free_values(ctx, before);
        reactive::dispose_owner(ctx, owner);
        return e;
    }
    let mut data = [qjs::JS_NewFloat64(owner as f64), new_array(ctx, before)];
    // SAFETY: the function keeps dups of `data`.
    let dispose =
        unsafe { qjs::JS_NewCFunctionData(ctx, Some(unmount), 0, 0, 2, data.as_mut_ptr()) };
    unsafe { qjs::JS_FreeValue(ctx, data[1]) };
    reactive::finish_value(ctx, dispose)
}

unsafe extern "C" fn unmount(
    ctx: *mut JsContext,
    _this: JsValue,
    _argc: c_int,
    _argv: *mut JsValue,
    _magic: c_int,
    data: *mut JsValue,
) -> JsValue {
    // SAFETY: two data values (`mount`).
    let (owner, before) = unsafe { (*data, *data.add(1)) };
    let mut id = 0f64;
    unsafe { qjs::JS_ToFloat64(ctx, &mut id, owner) };
    reactive::dispose_owner(ctx, id as u32);
    let before = array_items(ctx, before);
    let root = dom(ctx).root.get();
    let current: Vec<JsValue> = unsafe { &(*node_ptr(ctx, root)).children }
        .iter()
        .map(|&v| unsafe { qjs::JS_DupValue(ctx, v) })
        .collect();
    let mut result = UNDEFINED;
    for &c in &current {
        if !before.iter().any(|&b| same(b, c))
            && let Err(e) = remove_child(ctx, root, c)
        {
            result = e;
            break;
        }
    }
    free_values(ctx, current);
    free_values(ctx, before);
    result
}

/// `setup(tailwind, theme)`: the app's Tailwind theme and the theme of the
/// Rust components (JSON strings, or null), before anything renders.
unsafe extern "C" fn setup(
    ctx: *mut JsContext,
    _this: JsValue,
    argc: c_int,
    argv: *mut JsValue,
) -> JsValue {
    let root = dom(ctx).root.get();
    for (i, name) in ["tailwindTheme", "uiTheme"].into_iter().enumerate() {
        let v = arg(argc, argv, i);
        // SAFETY: an inline tag check.
        if unsafe { qjs::JS_IsString(v) }
            && let Err(e) = set_prop(ctx, root, name, v, UNDEFINED)
        {
            return e;
        }
    }
    UNDEFINED
}

// --- Events ---------------------------------------------------------------------

/// The handler node `id` has for `event` (owned), if any.
pub(crate) fn handler(ctx: *mut JsContext, id: NodeId, event: EventKind) -> Option<JsValue> {
    let p = dom(ctx).by_id(id);
    if p.is_null() {
        return None;
    }
    // SAFETY: live node data; a dup of the handler.
    unsafe { &(*p).handlers }
        .iter()
        .find(|&&(e, _)| e == event)
        .map(|&(_, h)| unsafe { qjs::JS_DupValue(ctx, h) })
}

/// The exports of `nsh:rt`.
pub(crate) const RT_EXPORTS: &[(&CStr, Native, c_int)] = &[
    (c"element", element, 1),
    (c"text", text, 1),
    (c"append", append, 2),
    (c"insert", insert, 3),
    (c"prop", prop, 3),
    (c"bindProp", bind_prop, 3),
    (c"spread", spread, 2),
    (c"component", component, 2),
    (c"cond", cond, 3),
    (c"each", each, 3),
    (c"merge", merge, 2),
    (c"setup", setup, 2),
];

#[cfg(test)]
mod tests {
    use nsh_core::{EventKind, EventPayload, Op};

    use crate::Host;

    /// The ops since the last call, one line each (`Release` and the root's
    /// layout direction left out unless `all`).
    fn ops(host: &Host, all: bool) -> Vec<String> {
        let mut out = Vec::new();
        host.take_ops(&mut out);
        out.into_iter()
            .filter(|op| {
                all || !matches!(
                    op,
                    Op::Release { .. }
                        | Op::SetProp {
                            prop: nsh_core::Prop::LayoutDirection(_),
                            ..
                        }
                )
            })
            .map(|op| match op {
                Op::Create { id, tag } => format!("create {id} {tag:?}"),
                Op::CreateText { id, text } => format!("text {id} {text:?}"),
                Op::SetText { id, text } => format!("set text {id} {text:?}"),
                Op::Insert {
                    parent,
                    child,
                    before,
                } => match before {
                    Some(b) => format!("insert {child} into {parent} before {b}"),
                    None => format!("insert {child} into {parent}"),
                },
                Op::Remove { parent, child } => format!("remove {child} from {parent}"),
                Op::Release { id } => format!("release {id}"),
                Op::Listen { id, event, enabled } => format!("listen {id} {event:?} {enabled}"),
                Op::SetStyle { id, .. } => format!("style {id}"),
                other => format!("{other:?}"),
            })
            .collect()
    }

    /// The text JS reported with `report(value)` (a text node).
    fn reported(host: &Host) -> Vec<String> {
        let mut out = Vec::new();
        host.take_ops(&mut out);
        out.into_iter()
            .filter_map(|op| match op {
                Op::CreateText { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    const REPORT: &str = r#"
        import { text } from "nsh:rt";
        const report = (v) => text(JSON.stringify(v));
    "#;

    #[test]
    fn the_reactive_api() {
        let host = Host::new().unwrap();
        host.eval_source(&format!(
            r#"{REPORT}
            import {{ signal, computed, effect, batch, untracked, cleanup }} from "nsh";
            const log = [];
            const n = signal(1);
            const double = computed(() => n() * 2);
            effect(() => {{
                log.push(`effect ${{double()}} ${{untracked(() => n())}}`);
                cleanup(() => log.push("cleanup"));
            }});
            n.set(2);
            n.update((x) => x + 1);
            batch(() => {{ n.set(10); n.set(4); }});
            log.push(typeof n, typeof double, "set" in n, "set" in double);
            report(log);
            "#
        ))
        .unwrap();
        assert_eq!(
            reported(&host),
            [
                r#"["effect 2 1","cleanup","effect 4 2","cleanup","effect 6 3","cleanup","effect 8 4","function","function",true,false]"#
            ]
        );
    }

    #[test]
    fn mounting_props_events_and_text_updates() {
        let host = Host::new().unwrap();
        host.eval_source(
            r#"
            import { mount, signal } from "nsh";
            import { element, insert, prop, bindProp, append } from "nsh:rt";
            const count = signal(0);
            mount(() => {
                const v = element("view");
                prop(v, "onPress", () => count.update((c) => c + 1));
                bindProp(v, "accessibilityLabel", () => (count() > 1 ? "many" : "few"));
                const t = element("text");
                insert(t, () => count());
                append(v, t);
                return v;
            });
            "#,
        )
        .unwrap();
        assert_eq!(
            ops(&host, false),
            [
                "create 2 View",
                "listen 2 Press true",
                "SetProp { id: 2, prop: Accessibility(Label(Some(\"few\"))) }",
                "create 4 Text",
                "text 6 \"0\"",
                "insert 6 into 4",
                "insert 4 into 2",
                "insert 2 into 1",
            ]
        );
        host.dispatch_event(2, EventKind::Press, &EventPayload::None);
        assert_eq!(ops(&host, false), ["set text 6 \"1\""]);
        host.dispatch_event(2, EventKind::Press, &EventPayload::None);
        assert_eq!(
            ops(&host, false),
            [
                "SetProp { id: 2, prop: Accessibility(Label(Some(\"many\"))) }",
                "set text 6 \"2\""
            ]
        );
    }

    #[test]
    fn control_flow_and_keyed_rows() {
        let host = Host::new().unwrap();
        host.eval_source(
            r#"
            import { mount, signal } from "nsh";
            import { element, insert, text, cond, each } from "nsh:rt";
            const show = signal(true);
            const items = signal([{ id: "a" }, { id: "b" }, { id: "c" }]);
            globalThis.toggle = () => show.update((s) => !s);
            globalThis.reorder = () => items.set([{ id: "c" }, { id: "a" }, { id: "d" }]);
            mount(() => {
                const v = element("view");
                insert(v, cond(() => show(), () => text("on"), () => text("off")));
                insert(v, each(() => items(), (item, index) => {
                    const t = element("text");
                    insert(t, () => `${item().id}${index()}`);
                    return t;
                }, (item) => item.id));
                return v;
            });
            "#,
        )
        .unwrap();
        let first = ops(&host, false);
        assert!(first.contains(&"text 4 \"on\"".to_owned()), "{first:?}");
        host.eval_script("toggle.js", "toggle()").unwrap();
        assert_eq!(
            ops(&host, false),
            [
                "text 18 \"off\"",
                "remove 4 from 2",
                "insert 18 into 2 before 6"
            ]
        );
        // `b` goes, `c` moves first, `d` is new; `a` and `c` show their new
        // indexes in place.
        host.eval_script("reorder.js", "reorder()").unwrap();
        assert_eq!(
            ops(&host, false),
            [
                "create 20 Text",
                "text 22 \"d2\"",
                "insert 22 into 20",
                "insert 14 into 2 before 6",
                "remove 10 from 2",
                "insert 20 into 2",
                "set text 16 \"c0\"",
                "set text 8 \"a1\"",
            ]
        );
    }

    #[test]
    fn contexts_reach_what_providers_render() {
        let host = Host::new().unwrap();
        host.eval_source(&format!(
            r#"{REPORT}
            import {{ context, mount }} from "nsh";
            import {{ component }} from "nsh:rt";
            const Theme = context("light");
            const seen = [Theme()];
            const Child = () => {{ seen.push(Theme()); return null; }};
            mount(() => component(Theme.Provider, {{
                value: "dark",
                get children() {{ return [component(Child, {{}}), () => component(Child, {{}})]; }},
            }}));
            report(seen);
            "#
        ))
        .unwrap();
        assert_eq!(reported(&host), [r#"["light","dark","dark"]"#]);
    }

    #[test]
    fn collected_nodes_are_released() {
        let host = Host::new().unwrap();
        host.eval_source(
            r#"
            import { element, append } from "nsh:rt";
            (() => {
                const v = element("view");
                append(v, element("text"));
            })();
            globalThis.keep = element("view");
            "#,
        )
        .unwrap();
        // Unreferenced at once: released with the first batch.
        let first = ops(&host, true);
        assert_eq!(&first[first.len() - 2..], ["release 2", "release 4"]);
        host.collect_garbage();
        assert!(ops(&host, true).is_empty(), "the kept view stays");
    }

    #[test]
    fn bytecode_imports_native_modules() {
        let bytecode = crate::compile_bytecode(
            r#"import { signal } from "nsh"; import { text } from "nsh:rt"; const s = signal(7); text(String(s()));"#,
            false,
        )
        .unwrap();
        let host = Host::new().unwrap();
        host.eval_bytecode(&bytecode).unwrap();
        assert_eq!(reported(&host), ["7"]);
        // Names are checked when the module links.
        let bytecode = crate::compile_bytecode(r#"import { nope } from "nsh";"#, false).unwrap();
        let err = Host::new().unwrap().eval_bytecode(&bytecode).unwrap_err();
        assert!(err.message.contains("nope"), "{err}");
    }
}
