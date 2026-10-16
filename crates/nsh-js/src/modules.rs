//! The native ES modules apps import: `nsh` (the reactive API and
//! `mount`) and `nsh:rt` (the renderer the bundler's JSX output calls).
//!
//! Their exports are native functions made when a bundle first imports
//! the module. QuickJS-NG resolves imports when it compiles a module, so
//! the bytecode compiler installs the same loader (a module then only
//! declares its export names; nothing runs).

use std::ffi::{CStr, c_char, c_int, c_void};

use rquickjs::qjs;

use crate::{
    dom,
    reactive::{self, Native},
};

/// A native module: its name and exports (name, function, `length`).
struct NativeModule {
    name: &'static CStr,
    exports: &'static [(&'static CStr, Native, c_int)],
}

/// `nsh`: the API apps write against.
const NSH: &[(&CStr, Native, c_int)] = &[
    (c"signal", reactive::signal, 2),
    (c"computed", reactive::computed, 2),
    (c"effect", reactive::effect, 1),
    (c"batch", reactive::batch, 1),
    (c"untracked", reactive::untrack, 1),
    (c"cleanup", reactive::on_cleanup, 1),
    (c"context", dom::context, 1),
    (c"mount", dom::mount, 2),
];

const MODULES: &[NativeModule] = &[
    NativeModule {
        name: c"nsh",
        exports: NSH,
    },
    NativeModule {
        name: c"nsh:rt",
        exports: dom::RT_EXPORTS,
    },
];

fn find(name: &CStr) -> Option<&'static NativeModule> {
    MODULES.iter().find(|m| m.name == name)
}

/// Makes runtime `rt` load the native modules.
pub(crate) fn set_loader(rt: *mut qjs::JSRuntime) {
    // SAFETY: a plain C loader without opaque data; the default name
    // normalization keeps bare specifiers as they are.
    unsafe { qjs::JS_SetModuleLoaderFunc(rt, None, Some(load), std::ptr::null_mut()) };
}

unsafe extern "C" fn load(
    ctx: *mut qjs::JSContext,
    name: *const c_char,
    _opaque: *mut c_void,
) -> *mut qjs::JSModuleDef {
    // SAFETY: QuickJS passes a NUL-terminated module name.
    let name = unsafe { CStr::from_ptr(name) };
    let Some(module) = find(name) else {
        // SAFETY: throws on a live context; `%s` keeps the name literal.
        unsafe {
            qjs::JS_ThrowReferenceError(
                ctx,
                c"could not load module '%s' (only nsh's own modules can be imported at run time)"
                    .as_ptr(),
                name.as_ptr(),
            )
        };
        return std::ptr::null_mut();
    };
    // SAFETY: a new C module declaring its exports (set by `init`).
    unsafe {
        let m = qjs::JS_NewCModule(ctx, module.name.as_ptr(), Some(init));
        if m.is_null() {
            return m;
        }
        for (export, ..) in module.exports {
            qjs::JS_AddModuleExport(ctx, m, export.as_ptr());
        }
        m
    }
}

unsafe extern "C" fn init(ctx: *mut qjs::JSContext, m: *mut qjs::JSModuleDef) -> c_int {
    // SAFETY: the module's name atom, freed below; exports are new
    // functions the module takes.
    unsafe {
        let atom = qjs::JS_GetModuleName(ctx, m);
        let name = qjs::JS_AtomToCStringLen(ctx, std::ptr::null_mut(), atom);
        qjs::JS_FreeAtom(ctx, atom);
        if name.is_null() {
            return -1;
        }
        let module = find(CStr::from_ptr(name));
        qjs::JS_FreeCString(ctx, name);
        let Some(module) = module else {
            return -1;
        };
        for &(export, f, length) in module.exports {
            let func = reactive::new_function(ctx, export, f, length);
            if qjs::JS_SetModuleExport(ctx, m, export.as_ptr(), func) != 0 {
                return -1;
            }
        }
    }
    0
}
