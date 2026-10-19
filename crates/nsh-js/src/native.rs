//! Native UI exports: the same renderer, keyed owners and event protocol as
//! every other NSH component. Only application callbacks execute JavaScript.
use std::{
    ffi::{CString, c_int},
    ptr,
};

use nsh_core::{
    EventKind, Tag,
    ui::{ComponentSpec, PropKind},
};
use rquickjs::qjs;

use crate::{
    dom,
    reactive::{self, arg},
};
type Value = qjs::JSValue;
type Context = qjs::JSContext;
const UNDEFINED: Value = qjs::JS_UNDEFINED;
type Callback =
    unsafe extern "C" fn(*mut Context, Value, c_int, *mut Value, c_int, *mut Value) -> Value;
fn function(ctx: *mut Context, callback: Callback, data: &mut [Value]) -> Value {
    // SAFETY: QuickJS retains its own references to captured data.
    unsafe {
        qjs::JS_NewCFunctionData(
            ctx,
            Some(callback),
            0,
            0,
            data.len() as _,
            data.as_mut_ptr(),
        )
    }
}
fn string(ctx: *mut Context, text: &str) -> Value {
    unsafe { qjs::JS_NewStringLen(ctx, text.as_ptr().cast(), text.len() as _) }
}
fn get(ctx: *mut Context, object: Value, key: &str) -> Value {
    let key = CString::new(key).unwrap_or_default();
    unsafe { qjs::JS_GetPropertyStr(ctx, object, key.as_ptr()) }
}
fn has_own(ctx: *mut Context, object: Value, key: &str) -> bool {
    if !unsafe { qjs::JS_IsObject(object) } {
        return false;
    }
    unsafe {
        let atom = qjs::JS_NewAtomLen(ctx, key.as_ptr().cast(), key.len() as _);
        let mut descriptor = qjs::JSPropertyDescriptor {
            flags: 0,
            value: UNDEFINED,
            getter: UNDEFINED,
            setter: UNDEFINED,
        };
        let present = qjs::JS_GetOwnProperty(ctx, &mut descriptor, object, atom) > 0;
        qjs::JS_FreeAtom(ctx, atom);
        if present {
            for value in [descriptor.value, descriptor.getter, descriptor.setter] {
                qjs::JS_FreeValue(ctx, value);
            }
        }
        present
    }
}
fn keys(ctx: *mut Context, object: Value) -> Result<Vec<String>, Value> {
    if !unsafe { qjs::JS_IsObject(object) } {
        return Ok(Vec::new());
    }
    unsafe {
        let mut entries = ptr::null_mut();
        let mut length = 0;
        if qjs::JS_GetOwnPropertyNames(
            ctx,
            &mut entries,
            &mut length,
            object,
            (qjs::JS_GPN_STRING_MASK | qjs::JS_GPN_ENUM_ONLY) as _,
        ) != 0
        {
            return Err(qjs::JS_EXCEPTION);
        }
        let mut names = Vec::with_capacity(length as usize);
        for index in 0..length as usize {
            let value = qjs::JS_AtomToValue(ctx, (*entries.add(index)).atom);
            if let Some(name) = dom::string_arg(ctx, value) {
                names.push(name);
            }
            qjs::JS_FreeValue(ctx, value);
        }
        qjs::JS_FreePropertyEnum(ctx, entries, length);
        Ok(names)
    }
}
fn call(ctx: *mut Context, function: Value) -> Value {
    unsafe { qjs::JS_Call(ctx, function, UNDEFINED, 0, ptr::null_mut()) }
}
/// Creates the platform-native export directly from the existing typed spec.
pub(crate) fn component(
    ctx: *mut Context,
    spec: &ComponentSpec,
    controlled: &[(&'static str, Vec<&'static str>)],
) -> Value {
    let controls: serde_json::Map<String, serde_json::Value> = controlled
        .iter()
        .map(|(event, fields)| ((*event).to_owned(), serde_json::json!(fields)))
        .collect();
    let metadata = serde_json::json!({ "typed": true,
        "props": spec.props.iter().filter(|prop| prop.kind == PropKind::Value).map(|prop| prop.name).collect::<Vec<_>>(),
        "events": spec.events.iter().map(|event| event.name).collect::<Vec<_>>(),
        "slots": spec.slots().collect::<Vec<_>>(), "children": spec.takes_children(), "controlled": controls });
    unsafe {
        let json = CString::new(metadata.to_string()).expect("JSON escapes NUL");
        let schema = qjs::JS_ParseJSON(
            ctx,
            json.as_ptr(),
            json.as_bytes().len() as _,
            c"native schema".as_ptr(),
        );
        if qjs::JS_IsException(schema) {
            return schema;
        }
        let name = string(ctx, spec.name);
        let component = crate::platform_modules::native_factory(ctx, name, schema);
        qjs::JS_FreeValue(ctx, name);
        qjs::JS_FreeValue(ctx, schema);
        component
    }
}
/// Attaches actual children and named slots to a typed native node. Keyed
/// owners are the renderer's existing `each`, not a second cache or graph.
pub(crate) fn attach(
    ctx: *mut Context,
    node: Value,
    props: Value,
    schema: Value,
) -> Result<(), Value> {
    let children = get(ctx, schema, "children");
    let takes_children = unsafe { qjs::JS_ToBool(ctx, children) > 0 };
    unsafe {
        qjs::JS_FreeValue(ctx, children);
    }
    if takes_children {
        let reader = dom::prop_reader(ctx, props, "children");
        let inserted = dom::insert_value_at(ctx, node, reader, None);
        unsafe {
            qjs::JS_FreeValue(ctx, reader);
        }
        inserted?;
    }
    let declared = crate::platform_modules::names(ctx, schema, c"slots");
    if declared.is_empty() && !has_own(ctx, props, "slots") {
        return Ok(());
    }
    // Read a slots getter only once per dependency update. It may construct
    // JSX, so enumeration must not invoke it a second time in every row.
    let record = reactive::memo(ctx, dom::prop_reader(ctx, props, "slots"), false);
    let names = function(ctx, slot_names, &mut [props, record, schema]);
    let render = function(ctx, slot_row, &mut [props, record]);
    let mut args = [names, render];
    let rows = unsafe { dom::each(ctx, UNDEFINED, 2, args.as_mut_ptr()) };
    let result = if unsafe { qjs::JS_IsException(rows) } {
        Err(rows)
    } else {
        dom::insert_value_at(ctx, node, rows, None)
    };
    unsafe {
        for value in [record, names, render, rows] {
            qjs::JS_FreeValue(ctx, value);
        }
    }
    result
}
unsafe extern "C" fn slot_names(
    ctx: *mut Context,
    _: Value,
    _: c_int,
    _: *mut Value,
    _: c_int,
    data: *mut Value,
) -> Value {
    unsafe {
        let (props, record, schema) = (*data, *data.add(1), *data.add(2));
        let object = call(ctx, record);
        if qjs::JS_IsException(object) {
            return object;
        }
        let names = keys(ctx, object);
        qjs::JS_FreeValue(ctx, object);
        let mut names = match names {
            Ok(names) => names,
            Err(error) => return error,
        };
        for name in crate::platform_modules::names(ctx, schema, c"slots") {
            if !names.contains(&name) && has_own(ctx, props, &name) {
                names.push(name);
            }
        }
        let mut names: Vec<_> = names.iter().map(|name| string(ctx, name)).collect();
        qjs::JS_NewArrayFrom(ctx, names.len() as _, names.as_mut_ptr())
    }
}
unsafe extern "C" fn slot_row(
    ctx: *mut Context,
    _: Value,
    argc: c_int,
    argv: *mut Value,
    _: c_int,
    data: *mut Value,
) -> Value {
    unsafe {
        let name = call(ctx, arg(argc, argv, 0));
        if qjs::JS_IsException(name) {
            return name;
        }
        let reader = function(ctx, read_slot, &mut [*data, *data.add(1), name]);
        let result = make_slot(ctx, name, reader);
        qjs::JS_FreeValue(ctx, name);
        qjs::JS_FreeValue(ctx, reader);
        result.unwrap_or_else(|error| error)
    }
}
unsafe extern "C" fn read_slot(
    ctx: *mut Context,
    _: Value,
    _: c_int,
    _: *mut Value,
    _: c_int,
    data: *mut Value,
) -> Value {
    unsafe {
        let (props, record, name) = (*data, *data.add(1), *data.add(2));
        let Some(name) = dom::string_arg(ctx, name) else {
            return UNDEFINED;
        };
        let object = call(ctx, record);
        if qjs::JS_IsException(object) {
            return object;
        }
        let result = if has_own(ctx, object, &name) {
            get(ctx, object, &name)
        } else {
            get(ctx, props, &name)
        };
        qjs::JS_FreeValue(ctx, object);
        result
    }
}
fn make_slot(ctx: *mut Context, name: Value, reader: Value) -> Result<Value, Value> {
    let node = dom::create_element(ctx, Tag::Native)?;
    let result = (|| {
        let component = string(ctx, "__nsh.Slot");
        let set = dom::set_prop_value(ctx, node, "component", component);
        unsafe {
            qjs::JS_FreeValue(ctx, component);
        }
        set?;
        dom::set_prop_value(ctx, node, "$__nshSlot", name)?;
        // No allocation has been measured yet. Zero would constrain ordinary
        // NSH View content to zero and trap measurement in a zero-width loop.
        let width = reactive::new_signal(ctx, UNDEFINED);
        let style = function(ctx, slot_style, &mut [width]);
        let measured = function(ctx, slot_measure, &mut [width]);
        let bound = dom::bind_prop_with(ctx, node, "style", style);
        let listened = bound.and_then(|_| {
            dom::set_handler(
                ctx,
                dom::node_id(ctx, node).unwrap_or(0),
                EventKind::NativeEvent,
                measured,
            )
        });
        unsafe {
            for value in [width, style, measured] {
                qjs::JS_FreeValue(ctx, value);
            }
        }
        listened?;
        dom::insert_value_at(ctx, node, reader, None)
    })();
    if let Err(error) = result {
        unsafe {
            qjs::JS_FreeValue(ctx, node);
        }
        return Err(error);
    }
    Ok(node)
}
unsafe extern "C" fn slot_style(
    ctx: *mut Context,
    _: Value,
    _: c_int,
    _: *mut Value,
    _: c_int,
    data: *mut Value,
) -> Value {
    unsafe {
        let width = call(ctx, *data);
        if qjs::JS_IsException(width) {
            return width;
        }
        let style = qjs::JS_NewObject(ctx);
        qjs::JS_SetPropertyStr(ctx, style, c"width".as_ptr(), width);
        qjs::JS_SetPropertyStr(ctx, style, c"flexShrink".as_ptr(), qjs::JS_NewFloat64(0.));
        style
    }
}
unsafe extern "C" fn slot_measure(
    ctx: *mut Context,
    _: Value,
    argc: c_int,
    argv: *mut Value,
    _: c_int,
    data: *mut Value,
) -> Value {
    unsafe {
        let event = arg(argc, argv, 0);
        let payload = get(ctx, event, "data");
        let width = get(ctx, payload, "width");
        qjs::JS_FreeValue(ctx, payload);
        if qjs::JS_IsNumber(width) {
            reactive::set_signal(ctx, *data, width);
        } else {
            qjs::JS_FreeValue(ctx, width);
        }
        UNDEFINED
    }
}
/// Invokes the real handler, then acknowledges accepted getter values even
/// when they are unchanged. No component-specific JS wrapper is involved.
pub(crate) fn event(
    ctx: *mut Context,
    props: Value,
    node: Value,
    schema: Value,
    event: Value,
) -> Value {
    unsafe {
        let name = get(ctx, event, "name");
        let event_name = dom::string_arg(ctx, name);
        qjs::JS_FreeValue(ctx, name);
        let Some(name) = event_name else {
            return UNDEFINED;
        };
        let mut chars = name.chars();
        let handler_name = chars
            .next()
            .map(|first| format!("on{}{}", first.to_ascii_uppercase(), chars.as_str()))
            .unwrap_or_default();
        let handler = get(ctx, props, &handler_name);
        if qjs::JS_IsException(handler) {
            return handler;
        }
        let result = if qjs::JS_IsFunction(ctx, handler) {
            let mut data = [get(ctx, event, "data")];
            let result = qjs::JS_Call(ctx, handler, UNDEFINED, 1, data.as_mut_ptr());
            qjs::JS_FreeValue(ctx, data[0]);
            result
        } else {
            UNDEFINED
        };
        qjs::JS_FreeValue(ctx, handler);
        let thrown = qjs::JS_IsException(result).then(|| qjs::JS_GetException(ctx));
        let ack = acknowledge(ctx, props, node, schema, &name);
        if let Some(thrown) = thrown {
            if qjs::JS_IsException(ack) {
                qjs::JS_FreeValue(ctx, thrown);
                return ack;
            }
            qjs::JS_FreeValue(ctx, ack);
            return qjs::JS_Throw(ctx, thrown);
        }
        if qjs::JS_IsException(ack) {
            qjs::JS_FreeValue(ctx, result);
            return ack;
        }
        qjs::JS_FreeValue(ctx, ack);
        result
    }
}
fn acknowledge(ctx: *mut Context, props: Value, node: Value, schema: Value, event: &str) -> Value {
    let lower = event.to_ascii_lowercase();
    if ["error", "snapshot", "request"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return UNDEFINED;
    }
    unsafe {
        let controls = get(ctx, schema, "controlled");
        if !qjs::JS_IsObject(controls) {
            qjs::JS_FreeValue(ctx, controls);
            return UNDEFINED;
        }
        let fields = get(ctx, controls, event);
        qjs::JS_FreeValue(ctx, controls);
        if !qjs::JS_IsArray(fields) {
            qjs::JS_FreeValue(ctx, fields);
            return UNDEFINED;
        }
        let mut length = 0i64;
        qjs::JS_GetLength(ctx, fields, &mut length);
        let accepted = qjs::JS_NewObject(ctx);
        let mut count = 0;
        for index in 0..length {
            let field = qjs::JS_GetPropertyInt64(ctx, fields, index);
            if let Some(key) = dom::string_arg(ctx, field) {
                let value = get(ctx, props, &key);
                if qjs::JS_IsException(value) {
                    qjs::JS_FreeValue(ctx, field);
                    qjs::JS_FreeValue(ctx, fields);
                    qjs::JS_FreeValue(ctx, accepted);
                    return value;
                }
                if !qjs::JS_IsUndefined(value) {
                    let key = CString::new(key).unwrap_or_default();
                    qjs::JS_SetPropertyStr(ctx, accepted, key.as_ptr(), value);
                    count += 1;
                } else {
                    qjs::JS_FreeValue(ctx, value);
                }
            }
            qjs::JS_FreeValue(ctx, field);
        }
        qjs::JS_FreeValue(ctx, fields);
        if count == 0 {
            qjs::JS_FreeValue(ctx, accepted);
            return UNDEFINED;
        }
        let command = get(ctx, node, "command");
        let mut args = [string(ctx, "__nsh.controlled_ack"), accepted];
        let result = qjs::JS_Call(ctx, command, node, 2, args.as_mut_ptr());
        for value in [command, args[0], args[1]] {
            qjs::JS_FreeValue(ctx, value);
        }
        result
    }
}

/// Compatibility factories use the same native renderer as typed UI sets.
/// The old leaf factory deliberately retains its original behavior.
pub(crate) unsafe extern "C" fn with_children(
    ctx: *mut Context,
    _: Value,
    argc: c_int,
    argv: *mut Value,
) -> Value {
    compatibility(ctx, arg(argc, argv, 0), arg(argc, argv, 1), true)
}
pub(crate) unsafe extern "C" fn without_children(
    ctx: *mut Context,
    _: Value,
    argc: c_int,
    argv: *mut Value,
) -> Value {
    compatibility(ctx, arg(argc, argv, 0), arg(argc, argv, 1), true)
}
fn compatibility(ctx: *mut Context, name: Value, source: Value, children: bool) -> Value {
    const MUTATIONS: &[&str] = &[
        "change",
        "select",
        "openChange",
        "selectionChange",
        "checkedChange",
        "valueChange",
        "pageChange",
        "activeChange",
        "resize",
        "expandedChange",
        "expandChange",
        "queryChange",
        "search",
        "sort",
        "filter",
        "release",
    ];
    unsafe {
        let schema = qjs::JS_NewObject(ctx);
        for key in ["props", "events"] {
            let key = CString::new(key).unwrap();
            let value = qjs::JS_GetPropertyStr(ctx, source, key.as_ptr());
            qjs::JS_SetPropertyStr(ctx, schema, key.as_ptr(), value);
        }
        qjs::JS_SetPropertyStr(ctx, schema, c"typed".as_ptr(), qjs::JS_TRUE);
        qjs::JS_SetPropertyStr(
            ctx,
            schema,
            c"children".as_ptr(),
            if children {
                qjs::JS_TRUE
            } else {
                qjs::JS_FALSE
            },
        );
        let configured = get(ctx, source, "controlled");
        let fields = crate::platform_modules::names(ctx, source, c"props");
        let default = ["value", "checked", "open", "page", "selectedIndex"];
        let controls = qjs::JS_NewObject(ctx);
        for event in crate::platform_modules::names(ctx, source, c"events") {
            let lower = event.to_ascii_lowercase();
            if ["error", "snapshot", "request"]
                .iter()
                .any(|word| lower.contains(word))
            {
                continue;
            }
            let list = if qjs::JS_IsUndefined(configured) {
                if !MUTATIONS.contains(&event.as_str()) {
                    continue;
                }
                let array = qjs::JS_NewArray(ctx);
                for (index, field) in fields
                    .iter()
                    .filter(|field| default.contains(&field.as_str()))
                    .enumerate()
                {
                    qjs::JS_SetPropertyUint32(ctx, array, index as _, string(ctx, field));
                }
                array
            } else if qjs::JS_IsArray(configured) {
                if !MUTATIONS.contains(&event.as_str()) {
                    continue;
                }
                qjs::JS_DupValue(ctx, configured)
            } else {
                get(ctx, configured, &event)
            };
            let key = CString::new(event).unwrap();
            qjs::JS_SetPropertyStr(ctx, controls, key.as_ptr(), list);
        }
        qjs::JS_FreeValue(ctx, configured);
        qjs::JS_SetPropertyStr(ctx, schema, c"controlled".as_ptr(), controls);
        let result = crate::platform_modules::native_factory(ctx, name, schema);
        qjs::JS_FreeValue(ctx, schema);
        result
    }
}
/// A public named subtree uses the same measured slot element as generated exports.
pub(crate) unsafe extern "C" fn slot(
    ctx: *mut Context,
    _: Value,
    argc: c_int,
    argv: *mut Value,
) -> Value {
    let props = arg(argc, argv, 0);
    let name = string(ctx, "");
    let reader = dom::prop_reader(ctx, props, "children");
    let result = make_slot(ctx, name, reader);
    unsafe {
        qjs::JS_FreeValue(ctx, name);
        qjs::JS_FreeValue(ctx, reader);
    }
    let node = match result {
        Ok(node) => node,
        Err(error) => return error,
    };
    let name = function(ctx, slot_name, &mut [props]);
    let bound = dom::bind_prop_with(ctx, node, "$__nshSlot", name);
    unsafe {
        qjs::JS_FreeValue(ctx, name);
    }
    match bound {
        Ok(()) => reactive::finish_value(ctx, node),
        Err(error) => {
            unsafe {
                qjs::JS_FreeValue(ctx, node);
            }
            error
        }
    }
}
unsafe extern "C" fn slot_name(
    ctx: *mut Context,
    _: Value,
    _: c_int,
    _: *mut Value,
    _: c_int,
    data: *mut Value,
) -> Value {
    let name = get(ctx, unsafe { *data }, "name");
    if unsafe { qjs::JS_IsUndefined(name) || qjs::JS_IsNull(name) } {
        unsafe {
            qjs::JS_FreeValue(ctx, name);
        }
        string(ctx, "")
    } else {
        name
    }
}
