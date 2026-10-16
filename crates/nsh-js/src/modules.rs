//! The native ES modules apps compiled by nsh import: nsh's own (`nsh`,
//! `nsh:rt`, `nsh/device`, `nsh/accessibility`; [`nsh_core::js::NATIVE_MODULES`])
//! and those of registries (`Registry::specifier`): the UI sets a host
//! serves (`nsh/ui/mobile`) and the runtime's (`nsh/android`, ...), whose
//! components, modules and re-exports become exports (`components.rs`).
//! A module of both kinds (`nsh/accessibility`) has the exports of both.
//!
//! The exports are made when a bundle first imports the module. QuickJS-NG
//! resolves imports when it compiles a module, so the bytecode compiler
//! installs the same loader (a module then only declares its export names;
//! nothing runs).

use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    sync::Arc,
};

use nsh_core::ui::{ComponentSpec, ModuleSpec, Registry};
use rquickjs::qjs;

use crate::{
    components, dom, env,
    reactive::{self, Native},
};

type JsValue = qjs::JSValue;
type JsContext = qjs::JSContext;

/// A value nsh's own modules export.
#[derive(Clone, Copy)]
pub(crate) enum Export {
    /// A native function and its `length`.
    Function(Native, c_int),
    /// A value made when the module is first imported (owned).
    Value(fn(*mut JsContext) -> JsValue),
}

/// One of nsh's own modules.
struct NativeModule {
    name: &'static str,
    exports: &'static [(&'static str, Export)],
}

/// `nsh`: the API apps write against.
const NSH: &[(&str, Export)] = &[
    ("signal", Export::Function(reactive::signal, 2)),
    ("computed", Export::Function(reactive::computed, 2)),
    ("effect", Export::Function(reactive::effect, 1)),
    ("batch", Export::Function(reactive::batch, 1)),
    ("untracked", Export::Function(reactive::untrack, 1)),
    ("cleanup", Export::Function(reactive::on_cleanup, 1)),
    ("context", Export::Function(dom::context, 1)),
    ("mount", Export::Function(dom::mount, 2)),
];

const MODULES: &[NativeModule] = &[
    NativeModule {
        name: "nsh",
        exports: NSH,
    },
    NativeModule {
        name: "nsh:rt",
        exports: dom::RT_EXPORTS,
    },
    NativeModule {
        name: "nsh/device",
        exports: env::DEVICE_EXPORTS,
    },
    NativeModule {
        name: "nsh/accessibility",
        exports: env::ACCESSIBILITY_EXPORTS,
    },
];

/// One export of a module, as it is made.
enum Made {
    Own(Export),
    /// A Rust component, with the functions of a module of the same name
    /// (and that module's namespace).
    Component(
        Arc<ComponentSpec>,
        Option<(Arc<ModuleSpec>, Option<&'static str>)>,
    ),
    /// A module object (its namespace for calls).
    Module(Arc<ModuleSpec>, Option<&'static str>),
    /// One function of a flat module.
    Function(Arc<ModuleSpec>, Option<&'static str>, &'static str),
    /// A runtime value a registry exports as it is (`safeArea`).
    Reexport(&'static str),
}

/// The modules a VM (or the bytecode compiler) serves: nsh's own, the
/// runtime's registries and the host's UI sets.
pub(crate) struct ModuleTable {
    registries: Vec<Registry>,
}

impl ModuleTable {
    pub(crate) fn new(ui_sets: &[Registry]) -> ModuleTable {
        let mut registries: Vec<Registry> = nsh_core::runtime::registries().to_vec();
        registries.extend(ui_sets.iter().filter(|r| r.specifier().is_some()).cloned());
        ModuleTable { registries }
    }

    /// The exports of module `name`, if it is one.
    fn exports(&self, name: &str) -> Option<Vec<(String, Made)>> {
        let mut out: Vec<(String, Made)> = Vec::new();
        let mut found = false;
        if let Some(m) = MODULES.iter().find(|m| m.name == name) {
            found = true;
            out.extend(m.exports.iter().map(|&(n, e)| (n.to_owned(), Made::Own(e))));
        }
        for registry in self
            .registries
            .iter()
            .filter(|r| r.specifier() == Some(name))
        {
            found = true;
            let namespace = registry.namespace();
            let modules = registry.module_specs();
            let specs = registry.specs();
            for spec in &specs {
                let module = modules
                    .iter()
                    .find(|m| m.name == spec.export)
                    .map(|m| (m.clone(), namespace));
                out.push((spec.export.clone(), Made::Component(spec.clone(), module)));
            }
            for m in &modules {
                if specs.iter().any(|s| s.export == m.name) {
                    continue;
                }
                if m.flat {
                    for f in &m.functions {
                        out.push((
                            f.name.to_owned(),
                            Made::Function(m.clone(), namespace, f.name),
                        ));
                    }
                } else {
                    out.push((m.name.to_owned(), Made::Module(m.clone(), namespace)));
                }
            }
            for &value in registry.reexports().0 {
                out.push((value.to_owned(), Made::Reexport(value)));
            }
        }
        // nsh's own export wins over a registry's of the same name (it
        // builds on it: `AccessibilityInfo`).
        let mut seen = std::collections::HashSet::new();
        out.retain(|(name, _)| seen.insert(name.clone()));
        found.then_some(out)
    }
}

/// Makes runtime `rt` load the modules of `table` (which outlives it).
pub(crate) fn set_loader(rt: *mut qjs::JSRuntime, table: *const ModuleTable) {
    // SAFETY: a plain C loader; the default name normalization keeps bare
    // specifiers as they are.
    unsafe {
        qjs::JS_SetModuleLoaderFunc(rt, None, Some(load), table as *mut c_void);
    }
}

unsafe extern "C" fn load(
    ctx: *mut JsContext,
    name: *const c_char,
    opaque: *mut c_void,
) -> *mut qjs::JSModuleDef {
    // SAFETY: QuickJS passes a NUL-terminated module name; the table is the
    // one `set_loader` was given.
    let (cname, table) = unsafe { (CStr::from_ptr(name), &*(opaque as *const ModuleTable)) };
    let exports = cname.to_str().ok().and_then(|n| table.exports(n));
    let Some(exports) = exports else {
        // SAFETY: throws on a live context; `%s` keeps the name literal.
        unsafe {
            qjs::JS_ThrowReferenceError(
                ctx,
                c"could not load module '%s' (only nsh's own modules can be imported at run time)"
                    .as_ptr(),
                name,
            )
        };
        return std::ptr::null_mut();
    };
    // SAFETY: a new C module declaring its exports (set by `init`).
    unsafe {
        let m = qjs::JS_NewCModule(ctx, name, Some(init));
        if m.is_null() {
            return m;
        }
        for (export, _) in &exports {
            let export = CString::new(export.as_str()).unwrap_or_default();
            qjs::JS_AddModuleExport(ctx, m, export.as_ptr());
        }
        m
    }
}

unsafe extern "C" fn init(ctx: *mut JsContext, m: *mut qjs::JSModuleDef) -> c_int {
    // SAFETY: the module's name atom, freed below; exports are new values
    // the module takes; the state's table lives as long as the context.
    unsafe {
        let atom = qjs::JS_GetModuleName(ctx, m);
        let name = qjs::JS_AtomToCStringLen(ctx, std::ptr::null_mut(), atom);
        qjs::JS_FreeAtom(ctx, atom);
        if name.is_null() {
            return -1;
        }
        let state = crate::state::state(ctx);
        let exports = CStr::from_ptr(name)
            .to_str()
            .ok()
            .and_then(|n| (*state).modules.exports(n));
        qjs::JS_FreeCString(ctx, name);
        let Some(exports) = exports else {
            return -1;
        };
        for (export, made) in exports {
            let value = match made {
                Made::Own(Export::Function(f, length)) => {
                    let cname = CString::new(export.as_str()).unwrap_or_default();
                    reactive::new_function(ctx, &cname, f, length)
                }
                Made::Own(Export::Value(make)) => make(ctx),
                Made::Component(spec, module) => components::component(ctx, spec, module),
                Made::Module(spec, namespace) => components::module(ctx, &spec, namespace),
                Made::Function(spec, namespace, f) => {
                    components::module_function(ctx, &spec, namespace, f)
                }
                Made::Reexport(value) => env::reexport(ctx, value),
            };
            if qjs::JS_IsException(value) {
                return -1;
            }
            let cname = CString::new(export.as_str()).unwrap_or_default();
            if qjs::JS_SetModuleExport(ctx, m, cname.as_ptr(), value) != 0 {
                return -1;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn nshs_own_modules_are_the_ones_the_bundler_knows() {
        let names: Vec<&str> = super::MODULES.iter().map(|m| m.name).collect();
        assert_eq!(names, nsh_core::js::NATIVE_MODULES);
    }
}
