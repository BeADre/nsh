//! TypeScript declarations for editors: written to `.nsh/types` next to an
//! app, and found through the app's `tsconfig.json` ([`TSCONFIG`]). Apps
//! need no `node_modules` for them, like for the code.
//!
//! The declarations of nsh's native modules are written by hand
//! (`types/nsh`); those of what Rust specs describe (styles, elements,
//! animations, canvas methods, the UI sets' components and the runtime's
//! modules) are generated from the specs.

use oxc::{
    allocator::Allocator,
    codegen::Codegen,
    isolated_declarations::{IsolatedDeclarations, IsolatedDeclarationsOptions},
    parser::Parser,
    span::SourceType,
};

/// `tsconfig.json` of a new app (`nsh init`): resolves `nsh` to
/// `.nsh/types`, and JSX to nsh's elements.
pub const TSCONFIG: &str = r#"{
  "compilerOptions": {
    "target": "ES2023",
    "lib": ["ES2023"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "preserve",
    "jsxImportSource": "nsh",
    "strict": true,
    "noEmit": true,
    "isolatedModules": true,
    "skipLibCheck": true,
    "types": [],
    "paths": {
      "nsh": ["./.nsh/types/nsh/index.d.ts"],
      "nsh/*": ["./.nsh/types/nsh/*.d.ts"]
    }
  },
  "include": ["**/*.ts", "**/*.tsx", ".nsh/types/nsh/globals.d.ts"]
}
"#;

macro_rules! declared {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("../types/", $path)))),*]
    };
}

/// The hand-written declarations, by path under `.nsh/types`.
const DECLARED: &[(&str, &str)] = declared![
    "nsh/index.d.ts",
    "nsh/_core.d.ts",
    "nsh/types.d.ts",
    "nsh/globals.d.ts",
    "nsh/accessibility.d.ts",
    "nsh/android.d.ts",
    "nsh/animated.d.ts",
    "nsh/canvas.d.ts",
    "nsh/device.d.ts",
    "nsh/gesture.d.ts",
    "nsh/ios.d.ts",
    "nsh/list.d.ts",
    "nsh/native.d.ts",
    "nsh/video.d.ts",
    "nsh/webview.d.ts",
    "nsh/ui/mobile/navigation.d.ts",
];

/// Declarations of generated TypeScript (it has the explicit types
/// `--isolatedDeclarations` asks for).
fn declarations(source: &str) -> String {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let ret = IsolatedDeclarations::new(
        &allocator,
        IsolatedDeclarationsOptions {
            strip_internal: true,
        },
    )
    .build(&parsed.program);
    debug_assert!(
        ret.diagnostics.is_empty(),
        "generated TypeScript needs explicit types: {:?}",
        ret.diagnostics
    );
    Codegen::new().build(&ret.program).code
}

/// The files of `.nsh/types`, as (path relative to it, contents), with a
/// JSX namespace naming the app's kebab-case components (`elements`).
pub fn type_definitions(elements: &[crate::AppElement]) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = DECLARED
        .iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect();
    let generated = [
        ("nsh/style.gen.d.ts", nsh_core::js::style::typescript()),
        ("nsh/elements.gen.d.ts", nsh_core::js::host::typescript()),
        ("nsh/anim.gen.d.ts", nsh_core::js::anim::typescript()),
        ("nsh/canvas.gen.d.ts", nsh_core::js::canvas::typescript()),
        (
            "nsh/ui/mobile/components.gen.d.ts",
            nsh_core::ui::typescript(&nsh_ui_mobile::registry()),
        ),
    ];
    for (path, source) in generated {
        files.push((path.to_owned(), declarations(&source)));
    }
    // The runtime's modules: generated alone (`nsh/image`), or next to the
    // hand-written part that re-exports them (`nsh/android.gen`).
    for registry in nsh_core::runtime::registries() {
        let specifier = registry.namespace().expect("runtime modules have one");
        let written = format!("{specifier}.d.ts");
        let path = match DECLARED.iter().any(|(p, _)| *p == written) {
            true => format!("{specifier}.gen.d.ts"),
            false => written,
        };
        files.push((path, declarations(&nsh_core::ui::typescript(registry))));
    }
    files.extend(
        [
            (
                "nsh/ui/mobile/index.d.ts",
                "export * from \"./components.gen\";\nexport * from \"./navigation\";\n".to_owned(),
            ),
            (
                "nsh/ui/mobile.d.ts",
                "export * from \"./mobile/index\";\n".to_owned(),
            ),
            ("nsh/jsx-runtime.d.ts", native_jsx(elements)),
        ]
        .map(|(path, text)| (path.to_owned(), text)),
    );
    files
}

/// The built-in elements and their props type.
const NATIVE_ELEMENTS: &[(&str, &str)] = &[
    ("view", "ViewProps"),
    ("text", "TextProps"),
    ("image", "ImageProps"),
    ("scroll", "ScrollProps"),
    ("input", "InputProps"),
    ("canvas", "ControlProps"),
    ("web-view", "ControlProps"),
    ("video", "ControlProps"),
    ("modal", "ControlProps"),
    ("input-accessory", "ControlProps"),
    ("list", "ListProps<any>"),
];

/// `nsh/jsx-runtime`: the built-in elements and the app's components named
/// in kebab case (whose names win, as in the
/// compiler).
fn native_jsx(elements: &[crate::AppElement]) -> String {
    let mut out = String::from(
        "import type { ViewProps, TextProps, ImageProps, ScrollProps, InputProps, ControlProps, JSX as Base } from \"./types\";\n\
         import type { ListProps } from \"./list\";\n\n\
         type PropsOf<C> = C extends (props: infer P) => any ? P : Record<string, any>;\n\n\
         export declare namespace JSX {\n\
         \x20 /** What components return and children accept. */\n\
         \x20 type Element = Base.Element;\n\
         \x20 interface ElementChildrenAttribute {\n    children: {};\n  }\n\
         \x20 interface IntrinsicElements {\n",
    );
    let quote = |s: &str| serde_json::to_string(s).expect("a string");
    for (tag, props) in NATIVE_ELEMENTS {
        if !elements.iter().any(|e| e.tag == *tag) {
            out.push_str(&format!("    {}: {props};\n", quote(tag)));
        }
    }
    if !elements.iter().any(|e| e.tag == "if") {
        out.push_str(
            "    /** Renders its children while `when` is truthy, else `else`. */\n\
             \x20   if: { when: unknown; else?: Element; children?: Element };\n",
        );
    }
    if !elements.iter().any(|e| e.tag == "each") {
        out.push_str(
            "    /** Renders `children(item, index)` for each item of `of`; rows are kept by `key` (default: the item). */\n\
             \x20   each: { of: readonly any[] | null | undefined; key?: (item: any) => unknown; children: (item: () => any, index: () => number) => Element };\n",
        );
    }
    for e in elements {
        let ty = match &e.source {
            // From `.nsh/types/nsh/`: the app directory is three levels up.
            Some((module, export)) => {
                let module = match module.strip_prefix("./") {
                    Some(rel) => format!("../../../{rel}"),
                    None => module.clone(),
                };
                format!(
                    "PropsOf<typeof import({})[{}]>",
                    quote(&module),
                    quote(export)
                )
            }
            None => "Record<string, any>".to_owned(),
        };
        out.push_str(&format!("    {}: {ty};\n", quote(&e.tag)));
    }
    out.push_str("  }\n}\n");
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_jsx_names_the_apps_components() {
        let jsx = super::native_jsx(&[
            crate::AppElement {
                tag: "weather-panel".into(),
                source: Some(("./demos/Weather".into(), "WeatherPanel".into())),
            },
            crate::AppElement {
                tag: "text".into(),
                source: None,
            },
        ]);
        assert!(jsx.contains(
            r#""weather-panel": PropsOf<typeof import("../../../demos/Weather")["WeatherPanel"]>;"#
        ));
        // The app's `Text` wins over the built-in `<text>`.
        assert!(jsx.contains(r#""text": Record<string, any>;"#));
        assert!(!jsx.contains("TextProps;"));
        assert!(jsx.contains(r#""web-view": ControlProps;"#));
    }

    #[test]
    fn declarations_cover_every_module() {
        let files = super::type_definitions(&[]);
        let has = |path: &str, text: &str| files.iter().any(|(p, t)| p == path && t.contains(text));
        assert!(has("nsh/index.d.ts", "export declare function signal"));
        assert!(has("nsh/list.d.ts", "export declare function List"));
        assert!(has("nsh/ui/mobile.d.ts", "./mobile/index"));
        assert!(has(
            "nsh/ui/mobile/navigation.d.ts",
            "export declare function Stack"
        ));
        assert!(has("nsh/ui/mobile/components.gen.d.ts", "Button"));
        assert!(has("nsh/style.gen.d.ts", "ViewStyle"));
        assert!(has("nsh/android.gen.d.ts", "Toast"));
        assert!(has("nsh/image.d.ts", "Image"));
        // Every module nsh serves has declarations.
        for spec in nsh_core::js::NATIVE_MODULES {
            let Some(name) = spec.strip_prefix("nsh/") else {
                continue;
            };
            let path = format!("nsh/{name}.d.ts");
            assert!(files.iter().any(|(p, _)| *p == path), "{path}");
        }
        // Nothing is left of Solid.
        assert!(!files.iter().any(|(_, t)| t.contains("solid-js")));
    }
}
