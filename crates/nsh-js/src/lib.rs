//! QuickJS host for nsh.
//!
//! [`Host`] owns one QuickJS runtime and context, installs the host APIs the
//! runtime needs (`console`, timers, the op bridge, web APIs such as
//! `fetch`, the native reactive graph), decodes the op buffer
//! produced by `js/runtime/index.ts` into [`nsh_core::Op`]s and dispatches
//! native events back into JS.
//!
//! A host is single-threaded (`!Send`): create and use it on the engine
//! thread.

mod animated;
mod bytecode;
mod canvas;
mod components;
mod console;
mod dom;
mod env;
mod error;
mod host;
mod list;
mod modules;
mod navigation;
mod net;
mod platform_modules;
mod protocol;
mod reactive;
mod sandbox;
mod state;
mod timers;
mod touch;
mod web;

pub use nsh_core::{
    js::Limits,
    platform::{ColorScheme, Environment, Idiom, Insets, Os},
};

pub use crate::{
    bytecode::{BUNDLE_NAME, BYTECODE_VERSION, compile_bytecode, compile_bytecode_with},
    error::JsError,
    host::{ErrorHandler, Host, HostOptions, TextMeasurer},
    net::{HttpRequest, HttpResponse, NetworkQueue, RequestId},
};
