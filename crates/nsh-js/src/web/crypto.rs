//! `crypto` (Web Crypto): `getRandomValues` and `randomUUID`, from the
//! operating system's secure random source (`getrandom`; in WebAssembly the
//! page's `crypto.getRandomValues`, the `nsh.random_fill` import).

use std::ffi::{CStr, c_int};

use rquickjs::qjs;

use super::js::{self, Ctx, R, V, native};

/// The global's name.
pub(crate) const NAME: &CStr = c"crypto";

/// The most bytes one `getRandomValues` call fills (Web Crypto's quota).
const QUOTA: usize = 65536;

/// The `crypto` object.
pub(crate) fn namespace(ctx: Ctx) -> R<V> {
    let crypto = js::new_object(ctx)?;
    let f = js::function(ctx, c"getRandomValues", get_random_values, 1)?;
    js::define(ctx, crypto.raw(), "getRandomValues", f, false)?;
    let f = js::function(ctx, c"randomUUID", random_uuid, 0)?;
    js::define(ctx, crypto.raw(), "randomUUID", f, false)?;
    let tag = js::string(ctx, "Crypto")?;
    // SAFETY: the property takes the string.
    unsafe {
        qjs::JS_DefinePropertyValue(
            ctx,
            crypto.raw(),
            qjs::JS_ATOM_Symbol_toStringTag as _,
            tag.take(),
            qjs::JS_PROP_CONFIGURABLE as c_int,
        );
    }
    Ok(crypto)
}

/// Whether typed array `kind` holds integers (the arrays Web Crypto fills).
fn is_integer(kind: c_int) -> bool {
    use qjs::*;
    [
        JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8C,
        JSTypedArrayEnum_JS_TYPED_ARRAY_INT8,
        JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8,
        JSTypedArrayEnum_JS_TYPED_ARRAY_INT16,
        JSTypedArrayEnum_JS_TYPED_ARRAY_UINT16,
        JSTypedArrayEnum_JS_TYPED_ARRAY_INT32,
        JSTypedArrayEnum_JS_TYPED_ARRAY_UINT32,
        JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_INT64,
        JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_UINT64,
    ]
    .contains(&(kind as JSTypedArrayEnum))
}

native! {
    fn get_random_values(ctx, _this, args) {
        let array = args.get(0);
        let Some(view) = js::view(ctx, array) else {
            return Err(js::type_error(
                ctx,
                "crypto.getRandomValues: the argument is not an ArrayBufferView",
            ));
        };
        if !view.kind.is_some_and(is_integer) {
            return Err(js::throw_dom(
                ctx,
                "crypto.getRandomValues: the array is not of an integer type",
                "TypeMismatchError",
            ));
        }
        if view.length > QUOTA {
            return Err(js::throw_dom(
                ctx,
                &format!(
                    "crypto.getRandomValues: the array's {} bytes exceed the quota of {QUOTA}",
                    view.length
                ),
                "QuotaExceededError",
            ));
        }
        // SAFETY: filled before any JS runs; nothing else borrows the bytes.
        if let Some(bytes) = unsafe { js::bytes_mut(ctx, array) } {
            fill(bytes).map_err(|()| {
                js::throw_dom(ctx, "crypto.getRandomValues: no random source", "OperationError")
            })?;
        }
        Ok(V::dup(ctx, array))
    }
}

native! {
    fn random_uuid(ctx, _this, args) {
        let mut b = [0u8; 16];
        fill(&mut b).map_err(|()| {
            js::throw_dom(ctx, "crypto.randomUUID: no random source", "OperationError")
        })?;
        js::string(ctx, &uuid_v4(b))
    }
}

/// A version 4 UUID of 16 random bytes, in lower case (RFC 9562).
fn uuid_v4(mut b: [u8; 16]) -> String {
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let mut out = String::with_capacity(36);
    for (i, byte) in b.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Fills `bytes` from the operating system's secure random source.
#[cfg(not(target_arch = "wasm32"))]
fn fill(bytes: &mut [u8]) -> Result<(), ()> {
    getrandom::fill(bytes).map_err(|e| log::error!("random source: {e}"))
}

/// Fills `bytes` from the page's `crypto.getRandomValues`.
#[cfg(target_arch = "wasm32")]
fn fill(bytes: &mut [u8]) -> Result<(), ()> {
    #[link(wasm_import_module = "nsh")]
    unsafe extern "C" {
        fn random_fill(ptr: *mut u8, len: usize);
    }
    // SAFETY: the slice is valid for the call; JS writes only into it.
    unsafe { random_fill(bytes.as_mut_ptr(), bytes.len()) };
    Ok(())
}

/// The TypeScript declaration of `crypto` (`globals.d.ts`).
pub(crate) const TYPES: &str = r#"/** Web Crypto's random values, from the system's secure random source. */
interface Crypto {
  /** Fills `array` (of an integer type, at most 65536 bytes) with random values and returns it. */
  getRandomValues<T extends Int8Array | Uint8Array | Uint8ClampedArray | Int16Array | Uint16Array | Int32Array | Uint32Array | BigInt64Array | BigUint64Array>(array: T): T;
  /** A random version 4 UUID, in lower case. */
  randomUUID(): `${string}-${string}-${string}-${string}-${string}`;
}

declare var crypto: Crypto;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuids_carry_version_4_and_the_rfc_variant() {
        assert_eq!(uuid_v4([0; 16]), "00000000-0000-4000-8000-000000000000");
        assert_eq!(uuid_v4([0xff; 16]), "ffffffff-ffff-4fff-bfff-ffffffffffff");
    }
}
