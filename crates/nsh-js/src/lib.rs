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

mod bytecode;
mod console;
mod error;
mod host;
mod net;
mod protocol;
mod reactive;
mod sandbox;
mod timers;
mod web;

pub use nsh_core::{
    js::Limits,
    platform::{ColorScheme, Environment, Idiom, Insets, Os},
};

pub use crate::{
    bytecode::{BUNDLE_NAME, BYTECODE_VERSION, compile_bytecode},
    error::JsError,
    host::{ErrorHandler, Host, HostOptions, TextMeasurer},
    net::{HttpRequest, HttpResponse, NetworkQueue, RequestId},
};
