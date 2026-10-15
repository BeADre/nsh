//! The runtime's modules written in Rust. Their JS face is generated like
//! a UI set's ([`crate::ui::typescript`]): `nsh/android`'s `Toast`,
//! `nsh/accessibility`'s commands, all of `nsh/image`. The hand-written TS
//! of a runtime module re-exports its generated part and adds what has to
//! run in JS (listeners of the app, reactive accessors).

use std::sync::OnceLock;

use crate::ui::{Function, Registry};

mod accessibility;
mod android;
mod image;

/// The runtime's Rust modules, one registry per JS module (its
/// [`Registry::namespace`]: `nsh/android`).
pub fn registries() -> &'static [Registry] {
    static REGISTRIES: OnceLock<Vec<Registry>> = OnceLock::new();
    REGISTRIES.get_or_init(|| {
        let mut android = Registry::runtime("nsh/android");
        android
            .module::<android::Toast>()
            .module::<android::BackHandler>()
            .module::<android::Android>();
        let mut accessibility = Registry::runtime("nsh/accessibility");
        accessibility.module::<accessibility::AccessibilityInfo>();
        let mut image = Registry::runtime("nsh/image");
        image.module::<image::Image>();
        vec![android, accessibility, image]
    })
}

/// Function `name` of a runtime module (`nsh/android:Toast.show`).
pub(crate) fn function(name: &str) -> Option<&'static Function> {
    let (namespace, _) = name.split_once(':')?;
    registries()
        .iter()
        .find(|r| r.namespace() == Some(namespace))?
        .function(name)
}
