//! Android backend for nsh.
//!
//! Built as `libnsh.so`, loaded by the Java runtime in `java/com/longbridge/nsh`. The
//! split mirrors the Apple backend:
//!
//! - The engine thread runs JS and layout. Text is typeset there with
//!   `StaticLayout` (through JNI), and every commit is encoded into one
//!   `byte[]` (+ an object array for strings and layouts) that a single JNI
//!   call hands to the UI thread, where `Mounter.java` applies it.
//! - Native animations (the `nsh-anim` graph) run on the UI thread in Rust:
//!   `Choreographer` ticks call in, and changed values come back as a
//!   compact array.
//!
//! On non-Android targets this crate is empty.

#![cfg(target_os = "android")]

mod anim;
#[cfg(feature = "dev")]
mod dev;
mod encode;
mod host;
mod java;
mod logger;
mod net;
#[cfg(feature = "js")]
mod package;
mod text;

use std::ffi::c_void;

pub use host::run_root;

/// The UI sets apps import ([`set_ui_sets`]; by default the mobile UI).
static UI_SETS: std::sync::RwLock<Option<Vec<nsh_core::ui::Registry>>> =
    std::sync::RwLock::new(None);

/// Sets the UI sets the engines started from now on serve: the mobile UI
/// and an app's own Rust components (`nsh.json` `"rust"`), which a host
/// built for the app passes before it starts.
pub fn set_ui_sets(sets: Vec<nsh_core::ui::Registry>) {
    *UI_SETS.write().unwrap_or_else(|e| e.into_inner()) = Some(sets);
}

/// The UI sets engines serve.
fn ui_sets() -> Vec<nsh_core::ui::Registry> {
    UI_SETS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_else(|| vec![nsh_ui_mobile::registry()])
}
/// The JNI bindings [`main`] expands to.
pub use jni;
use jni::{JavaVM, sys::jint};

/// Called by the VM when `System.loadLibrary("nsh")` runs.
#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
    logger::init();
    if let Err(err) = java::init(vm) {
        log::error!("nsh: JNI setup failed: {err}");
    }
    jni::sys::JNI_VERSION_1_6
}

/// Makes this library a Rust app's `libnsh.so` (built by
/// `nsh package --rust`): defines `Native.runRust`, which `AppActivity`
/// calls to run `$root` (a [`nsh_engine::Root`]).
#[macro_export]
macro_rules! main {
    ($root:expr) => {
        #[unsafe(no_mangle)]
        extern "system" fn Java_com_longbridge_nsh_Native_runRust(
            _: $crate::jni::JNIEnv<'_>,
            _: $crate::jni::objects::JClass<'_>,
            handle: $crate::jni::sys::jlong,
        ) {
            $crate::run_root(handle, $root);
        }
    };
}
