//! TypeScript declarations for editors: written to `.nsh/types` next to an
//! app, and found through the app's `tsconfig.json` ([`TSCONFIG`]). Apps
//! need no `node_modules` for them, like for the code.

use oxc::{
    allocator::Allocator,
    codegen::Codegen,
    isolated_declarations::{IsolatedDeclarations, IsolatedDeclarationsOptions},
    parser::Parser,
    span::SourceType,
};

/// `tsconfig.json` of a new app (`nsh init`): resolves `nsh` and `solid-js`
/// to `.nsh/types`, and JSX to nsh's elements.
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
      "nsh/*": ["./.nsh/types/nsh/*.d.ts"],
      "solid-js": ["./.nsh/types/solid-js/types/index.d.ts"],
      "solid-js/universal": ["./.nsh/types/solid-js/types/universal/index.d.ts"]
    }
  },
  "include": ["**/*.ts", "**/*.tsx", ".nsh/types/nsh/globals.d.ts"]
}
"#;

const SOLID_TYPES: &[(&str, &str)] = &[
    (
        "solid-js/types/index.d.ts",
        include_str!("../../../js/vendor/solid-js/types/index.d.ts"),
    ),
    (
        "solid-js/types/reactive/signal.d.ts",
        include_str!("../../../js/vendor/solid-js/types/reactive/signal.d.ts"),
    ),
    (
        "solid-js/types/reactive/observable.d.ts",
        include_str!("../../../js/vendor/solid-js/types/reactive/observable.d.ts"),
    ),
    (
        "solid-js/types/reactive/scheduler.d.ts",
        include_str!("../../../js/vendor/solid-js/types/reactive/scheduler.d.ts"),
    ),
    (
        "solid-js/types/reactive/array.d.ts",
        include_str!("../../../js/vendor/solid-js/types/reactive/array.d.ts"),
    ),
    (
        "solid-js/types/render/index.d.ts",
        include_str!("../../../js/vendor/solid-js/types/render/index.d.ts"),
    ),
    (
        "solid-js/types/render/component.d.ts",
        include_str!("../../../js/vendor/solid-js/types/render/component.d.ts"),
    ),
    (
        "solid-js/types/render/flow.d.ts",
        include_str!("../../../js/vendor/solid-js/types/render/flow.d.ts"),
    ),
    (
        "solid-js/types/render/Suspense.d.ts",
        include_str!("../../../js/vendor/solid-js/types/render/Suspense.d.ts"),
    ),
    (
        "solid-js/types/render/hydration.d.ts",
        include_str!("../../../js/vendor/solid-js/types/render/hydration.d.ts"),
    ),
    (
        "solid-js/types/universal/index.d.ts",
        include_str!("../../../js/vendor/solid-js/types/universal/index.d.ts"),
    ),
    (
        "solid-js/types/universal/universal.d.ts",
        include_str!("../../../js/vendor/solid-js/types/universal/universal.d.ts"),
    ),
    // Solid's own JSX namespace describes the DOM: nsh's replaces it.
    (
        "solid-js/types/jsx.d.ts",
        "export type { JSX } from \"nsh\";\n",
    ),
];

/// Declarations of a TypeScript module, from its source (it must have the
/// explicit types `--isolatedDeclarations` asks for).
fn declarations(source: &str, lang: crate::embedded::Lang) -> String {
    let allocator = Allocator::default();
    let source_type = match lang {
        crate::embedded::Lang::Tsx => SourceType::tsx(),
        _ => SourceType::ts(),
    };
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let ret = IsolatedDeclarations::new(
        &allocator,
        IsolatedDeclarationsOptions {
            strip_internal: true,
        },
    )
    .build(&parsed.program);
    debug_assert!(
        ret.diagnostics.is_empty(),
        "the runtime needs explicit types: {:?}",
        ret.diagnostics
    );
    Codegen::new().build(&ret.program).code
}

/// The files of `.nsh/types`, as (path relative to it, contents): one
/// declaration file per embedded file, importing each other relatively as
/// the sources do: `nsh/list.d.ts` for `nsh/list`; a UI set's files under
/// its directory (`nsh/ui/mobile/navigation.d.ts`) and its module next to it
/// (`nsh/ui/mobile.d.ts`, found through the `nsh/*` path of tsconfig).
pub fn type_definitions() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = Vec::new();
    for f in crate::embedded::files() {
        let dir = match f.root {
            "runtime" => "nsh".to_owned(),
            root => format!("nsh/{root}"),
        };
        files.push((
            format!("{dir}/{}.d.ts", f.file()),
            declarations(f.module.source, f.module.lang),
        ));
        if f.root != "runtime"
            && f.file() == "index"
            && let Some((_, last)) = f.root.rsplit_once('/')
        {
            files.push((
                format!("{dir}.d.ts"),
                format!("export * from \"./{last}/index\";\n"),
            ));
        }
    }
    files.extend(
        [
            (
                "nsh/jsx-runtime.d.ts",
                "export type { JSX } from \"./types\";\n".to_owned(),
            ),
            (
                "nsh/globals.d.ts",
                include_str!("../../../js/runtime/globals.d.ts").to_owned(),
            ),
        ]
        .map(|(path, text)| (path.to_owned(), text)),
    );
    files.extend(
        SOLID_TYPES
            .iter()
            .map(|(path, text)| ((*path).to_owned(), (*text).to_owned())),
    );
    files
}

/// `nsh` for apps compiled by nsh (`"compiler": "nsh"`): the native
/// module's API (`nsh-js`'s `reactive.rs` and `dom.rs`).
const NATIVE_NSH: &str = r#"import type { JSX } from "./jsx-runtime";
export type { JSX };
export type * from "./types";
export type { YNode } from "./_core";

/** A value that changes over time, read by calling it (tracked). */
export interface Accessor<T> {
  (): T;
}

/** A signal: `count()` reads it, `count.set(1)` and `count.update((n) => n + 1)` write it. */
export interface Signal<T> extends Accessor<T> {
  set(value: T): void;
  update(fn: (value: T) => T): void;
}

export interface SignalOptions<T> {
  /** When a new value counts as a change (default `===`); `false`: always. */
  equals?: false | ((previous: T, next: T) => boolean);
}

/** A signal holding `value`. */
export declare function signal<T>(value: T, options?: SignalOptions<T>): Signal<T>;
export declare function signal<T = undefined>(): Signal<T | undefined>;

/** A value derived from signals, computed again when they change (and read). */
export declare function computed<T>(fn: (previous: T | undefined) => T, options?: SignalOptions<T>): Accessor<T>;

/** Runs `fn` after rendering, and again when what it read changes. */
export declare function effect(fn: () => void): void;

/** Runs `fn`, applying its writes together at the end. */
export declare function batch<T>(fn: () => T): T;

/** Runs `fn` without tracking what it reads. */
export declare function untracked<T>(fn: () => T): T;

/** Runs `fn` before the current effect or component runs again or goes away. */
export declare function cleanup(fn: () => void): void;

/** A value components provide to what they render: `Theme()` reads the nearest one. */
export interface Context<T> extends Accessor<T> {
  Provider: (props: { value: T; children?: JSX.Element }) => JSX.Element;
}

/** A context with a default value. */
export declare function context<T>(defaultValue: T): Context<T>;

/** A component: a function of its props rendering elements. */
export type Component<P = {}> = (props: P) => JSX.Element;

/** Renders `app` into the surface; returns a function that removes it. */
export declare function mount<P extends object>(app: Component<P>, props?: P): () => void;
"#;

/// The built-in elements of apps compiled by nsh and their props type
/// (from the runtime's declarations until they are generated from Rust).
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
];

/// `nsh/jsx-runtime` for an app compiled by nsh: the built-in elements and
/// the app's components named in kebab case (whose names win, as in the
/// compiler).
fn native_jsx(elements: &[crate::AppElement]) -> String {
    let mut out = String::from(
        "import type { ViewProps, TextProps, ImageProps, ScrollProps, InputProps, ControlProps, JSX as Base } from \"./types\";\n\n\
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

/// The files of `.nsh/types` for the app in `dir`: with nsh's compiler,
/// the native `nsh` and a JSX namespace with the app's `elements`;
/// otherwise [`type_definitions`].
pub fn type_definitions_for(
    dir: &std::path::Path,
    elements: &[crate::AppElement],
) -> Result<Vec<(String, String)>, crate::BundleError> {
    let mut files = type_definitions();
    if crate::compiler(dir)? == crate::Compiler::Nsh {
        for (path, contents) in [
            ("nsh/index.d.ts", NATIVE_NSH.to_owned()),
            ("nsh/jsx-runtime.d.ts", native_jsx(elements)),
            // Its Rust components and modules (navigation is not native
            // yet).
            (
                "nsh/ui/mobile.d.ts",
                "export * from \"./mobile/components.gen\";\n".to_owned(),
            ),
        ] {
            match files.iter_mut().find(|(p, _)| p == path) {
                Some(file) => file.1 = contents,
                None => files.push((path.to_owned(), contents)),
            }
        }
    }
    Ok(files)
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
    fn runtime_declarations_need_no_inference() {
        for f in crate::embedded::files() {
            let allocator = super::Allocator::default();
            let source_type = match f.module.lang {
                crate::embedded::Lang::Tsx => super::SourceType::tsx(),
                _ => super::SourceType::ts(),
            };
            let parsed = super::Parser::new(&allocator, f.module.source, source_type).parse();
            let ret = super::IsolatedDeclarations::new(
                &allocator,
                super::IsolatedDeclarationsOptions {
                    strip_internal: true,
                },
            )
            .build(&parsed.program);
            assert!(ret.diagnostics.is_empty(), "{:?}", ret.diagnostics);
        }
        let files = super::type_definitions();
        assert!(
            files
                .iter()
                .any(|(p, t)| p == "nsh/list.d.ts" && t.contains("export declare function List"))
        );
        assert!(
            files
                .iter()
                .any(|(p, t)| p == "nsh/ui/mobile.d.ts" && t.contains("./mobile/index"))
        );
        assert!(
            files
                .iter()
                .any(|(p, t)| p == "nsh/ui/mobile/navigation.d.ts"
                    && t.contains("export declare function Stack"))
        );
    }
}
