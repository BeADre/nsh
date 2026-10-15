//! Derive macros of nsh's Rust components (`nsh_core::ui`):
//!
//! - `#[derive(Props)]` on a component's props struct: decoding the props JS
//!   sends (`$title`), defaults, slots and children, and the TypeScript
//!   declarations of the generated glue (doc comments included).
//! - `#[derive(PropEnum)]` on an enum of unit variants: a string union in
//!   JS (`"primary" | "plain"`).
//! - `#[derive(Event)]` on a component's event enum: the `on<Event>`
//!   handlers and their data types.
//!
//! - `#[methods]` on an `impl` block of a component: its functions become
//!   methods of the component's ref in JS (`ref.setValue(5)`,
//!   `await ref.value()`).
//!
//! The code they generate names `::nsh_core::ui`. `style_helpers!` is
//! internal to `nsh-core`: the spacing and sizing methods of `Styled`.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, FnArg, GenericArgument, Ident, ImplItem, ItemImpl,
    LitStr, Pat, PathArguments, ReturnType, Token, Type, bracketed, parse::Parse,
    parse_macro_input, punctuated::Punctuated, spanned::Spanned,
};

/// The doc comment of an item, lines joined.
fn docs(attrs: &[Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &attr.meta
            && let Expr::Lit(lit) = &nv.value
            && let syn::Lit::Str(s) = &lit.lit
        {
            lines.push(s.value().trim().to_owned());
        }
    }
    lines.join(" ").trim().to_owned()
}

/// `accessibility_label` / `AccessibilityLabel` -> `accessibilityLabel`.
fn camel(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper = false;
    for (i, c) in name.chars().enumerate() {
        if c == '_' {
            upper = true;
        } else if i == 0 {
            out.push(c.to_ascii_lowercase());
        } else if upper {
            out.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// The options of `#[prop(...)]`.
#[derive(Default)]
struct PropAttr {
    /// `default` (`Some(None)`) or `default = expr`.
    default: Option<Option<Expr>>,
    rename: Option<String>,
}

fn prop_attr(attrs: &[Attribute]) -> syn::Result<PropAttr> {
    let mut out = PropAttr::default();
    for attr in attrs {
        if !attr.path().is_ident("prop") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                out.default = Some(if meta.input.peek(Token![=]) {
                    Some(meta.value()?.parse()?)
                } else {
                    None
                });
                Ok(())
            } else if meta.path.is_ident("rename") {
                out.rename = Some(meta.value()?.parse::<LitStr>()?.value());
                Ok(())
            } else {
                Err(meta.error("expected `default`, `default = …` or `rename = \"…\"`"))
            }
        })?;
    }
    Ok(out)
}

/// The last segment of a type path (`Slot`, `Option`) and its first
/// generic argument.
fn last_segment(ty: &Type) -> Option<(&Ident, Option<&Type>)> {
    let Type::Path(p) = ty else { return None };
    let seg = p.path.segments.last()?;
    let arg = match &seg.arguments {
        PathArguments::AngleBracketed(a) => a.args.iter().find_map(|a| match a {
            GenericArgument::Type(t) => Some(t),
            _ => None,
        }),
        _ => None,
    };
    Some((&seg.ident, arg))
}

enum Kind {
    Value,
    Slot,
    Children,
    Items,
}

/// `#[derive(Props)]`: a component's props. Every field is a prop named in
/// camel case (`accessibility_label` is `accessibilityLabel`):
///
/// - a field of a [`JsValue`] type; required in TypeScript unless it is an
///   `Option` or has `#[prop(default)]` / `#[prop(default = expr)]`.
///   Fields start with their default (`Default::default()` without one);
///   JS clearing a prop restores it.
/// - `Option<Slot>`: a named slot, JSX the app passes in that prop.
/// - `Children`: the JSX children.
/// - `Items`: virtualized rows (in JS `each`, a row renderer and `List`'s
///   data props), at most one, and not with `Children`.
///
/// `#[prop(rename = "…")]` sets the JS name. The struct's doc comment
/// documents the component in TypeScript, the fields' its props.
#[proc_macro_derive(Props, attributes(prop))]
pub fn derive_props(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    props(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn props(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`Props` is derived for structs",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(input.span(), "`Props` needs named fields"));
    };
    let doc = docs(&input.attrs);
    let ui = quote!(::nsh_core::ui);
    let mut specs = Vec::new();
    let mut initial = Vec::new();
    let mut set_arms = Vec::new();
    let mut slot_arms = Vec::new();
    let mut children = None;
    let mut items: Option<Ident> = None;
    for field in &fields.named {
        let ident = field.ident.as_ref().expect("named");
        let ty = &field.ty;
        let attr = prop_attr(&field.attrs)?;
        let js = attr
            .rename
            .clone()
            .unwrap_or_else(|| camel(&ident.to_string()));
        let field_doc = docs(&field.attrs);
        let kind = match last_segment(ty) {
            Some((id, Some(inner))) if id == "Option" => match last_segment(inner) {
                Some((s, _)) if s == "Slot" => Kind::Slot,
                _ => Kind::Value,
            },
            Some((id, _)) if id == "Children" => Kind::Children,
            Some((id, _)) if id == "Items" => Kind::Items,
            _ => Kind::Value,
        };
        let is_option = matches!(last_segment(ty), Some((id, _)) if id == "Option");
        let init = match (&attr.default, &kind) {
            (_, Kind::Items) => quote!(#ui::Items::__new(#js)),
            (Some(Some(expr)), _) => quote!(#expr),
            _ => quote!(::core::default::Default::default()),
        };
        initial.push(quote!(#ident: #init));
        match kind {
            Kind::Value => {
                let optional = is_option || attr.default.is_some();
                specs.push(quote! {
                    #ui::PropSpec {
                        name: #js,
                        ts: <#ty as #ui::JsValue>::ts_type(),
                        optional: #optional,
                        doc: #field_doc,
                        kind: #ui::PropKind::Value,
                    }
                });
                set_arms.push(quote! {
                    #js => {
                        let value: #ty = match value {
                            ::core::option::Option::Some(v) => <#ty as #ui::JsValue>::from_json(v)
                                .map_err(|e| #ui::PropError(::std::format!("`{}`: {}", #js, e)))?,
                            ::core::option::Option::None => #init,
                        };
                        if self.#ident == value {
                            ::core::result::Result::Ok(false)
                        } else {
                            self.#ident = value;
                            ::core::result::Result::Ok(true)
                        }
                    }
                });
            }
            Kind::Slot => {
                specs.push(quote! {
                    #ui::PropSpec {
                        name: #js,
                        ts: ::std::string::String::from("JSX.Element"),
                        optional: true,
                        doc: #field_doc,
                        kind: #ui::PropKind::Slot,
                    }
                });
                slot_arms.push(quote! {
                    #js => {
                        let changed = self.#ident != slot;
                        self.#ident = slot;
                        changed
                    }
                });
            }
            Kind::Items => {
                if items.is_some() || children.is_some() {
                    return Err(syn::Error::new(
                        field.span(),
                        "one `Items` field at most, and no `Children` with it (the row renderer is the children)",
                    ));
                }
                specs.push(quote! {
                    #ui::PropSpec {
                        name: #js,
                        ts: ::std::string::String::new(),
                        optional: false,
                        doc: #field_doc,
                        kind: #ui::PropKind::Items,
                    }
                });
                items = Some(ident.clone());
            }
            Kind::Children => {
                if children.is_some() || items.is_some() {
                    return Err(syn::Error::new(
                        field.span(),
                        "one `Children` field at most, and none with `Items`",
                    ));
                }
                specs.push(quote! {
                    #ui::PropSpec {
                        name: "children",
                        ts: ::std::string::String::from("JSX.Element"),
                        optional: true,
                        doc: #field_doc,
                        kind: #ui::PropKind::Children,
                    }
                });
                children = Some(ident.clone());
            }
        }
    }
    let set_children = match &children {
        Some(ident) => quote! {
            let changed = self.#ident != children;
            self.#ident = children;
            changed
        },
        None => quote!({
            let _ = children;
            false
        }),
    };
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #ui::Props for #name #ty_generics #where_clause {
            fn doc() -> &'static str {
                #doc
            }

            fn spec() -> ::std::vec::Vec<#ui::PropSpec> {
                ::std::vec![#(#specs),*]
            }

            fn initial() -> Self {
                Self { #(#initial),* }
            }

            fn set(
                &mut self,
                name: &str,
                value: ::core::option::Option<::nsh_core::JsonValue>,
            ) -> ::core::result::Result<bool, #ui::PropError> {
                match name {
                    #(#set_arms)*
                    _ => {
                        let _ = value;
                        ::core::result::Result::Err(#ui::PropError(::std::format!("unknown prop `{name}`")))
                    }
                }
            }

            fn set_slot(&mut self, name: &str, slot: ::core::option::Option<#ui::Slot>) -> bool {
                match name {
                    #(#slot_arms)*
                    _ => {
                        let _ = slot;
                        false
                    }
                }
            }

            fn set_children(&mut self, children: #ui::Children) -> bool {
                #set_children
            }
        }
    })
}

/// `#[derive(PropEnum)]`: an enum of unit variants as a string union in JS,
/// each variant in camel case (`SemiBold` is `"semiBold"`;
/// `#[prop(rename = "…")]` sets it).
#[proc_macro_derive(PropEnum, attributes(prop))]
pub fn derive_prop_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    prop_enum(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn prop_enum(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`PropEnum` is derived for enums",
        ));
    };
    let ui = quote!(::nsh_core::ui);
    let mut names = Vec::new();
    let mut idents = Vec::new();
    for v in &data.variants {
        if !matches!(v.fields, Fields::Unit) {
            return Err(syn::Error::new(
                v.span(),
                "`PropEnum` variants have no fields",
            ));
        }
        let attr = prop_attr(&v.attrs)?;
        names.push(attr.rename.unwrap_or_else(|| camel(&v.ident.to_string())));
        idents.push(&v.ident);
    }
    let ts = names
        .iter()
        .map(|n| format!("{n:?}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let expected = format!("one of {ts}");
    Ok(quote! {
        impl #ui::JsValue for #name {
            fn ts_type() -> ::std::string::String {
                ::std::string::String::from(#ts)
            }

            fn from_json(value: ::nsh_core::JsonValue) -> ::core::result::Result<Self, #ui::PropError> {
                match value.as_str() {
                    #(::core::option::Option::Some(#names) => ::core::result::Result::Ok(Self::#idents),)*
                    _ => ::core::result::Result::Err(#ui::PropError::expected(#expected, &value)),
                }
            }

            fn to_json(&self) -> ::nsh_core::JsonValue {
                ::nsh_core::JsonValue::String(::std::string::String::from(match self {
                    #(Self::#idents => #names,)*
                }))
            }
        }
    })
}

/// `#[derive(Event)]`: a component's events. Each variant is an event in
/// camel case (`Press` is `onPress` in JS): a unit variant carries no
/// data, a variant with named fields an object of them (camel case), a
/// variant with one unnamed field that value. Doc comments document the
/// handlers.
#[proc_macro_derive(Event)]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    event(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn event(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`Event` is derived for enums",
        ));
    };
    let ui = quote!(::nsh_core::ui);
    let mut specs = Vec::new();
    let mut name_arms = Vec::new();
    let mut data_arms = Vec::new();
    for v in &data.variants {
        let ident = &v.ident;
        let js = camel(&ident.to_string());
        let doc = docs(&v.attrs);
        match &v.fields {
            Fields::Unit => {
                specs.push(quote! {
                    #ui::EventSpec { name: #js, data: ::std::string::String::from("void"), doc: #doc }
                });
                name_arms.push(quote!(Self::#ident => #js));
                data_arms.push(quote!(Self::#ident => ::nsh_core::JsonValue::Null));
            }
            Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                let ty = &f.unnamed[0].ty;
                specs.push(quote! {
                    #ui::EventSpec { name: #js, data: <#ty as #ui::JsValue>::ts_type(), doc: #doc }
                });
                name_arms.push(quote!(Self::#ident(..) => #js));
                data_arms.push(quote!(Self::#ident(v) => #ui::JsValue::to_json(v)));
            }
            Fields::Named(f) => {
                let idents: Vec<_> = f
                    .named
                    .iter()
                    .map(|f| f.ident.clone().expect("named"))
                    .collect();
                let keys: Vec<_> = idents.iter().map(|i| camel(&i.to_string())).collect();
                let tys: Vec<_> = f.named.iter().map(|f| &f.ty).collect();
                specs.push(quote! {
                    #ui::EventSpec {
                        name: #js,
                        data: {
                            let fields: ::std::vec::Vec<::std::string::String> = ::std::vec![
                                #(::std::format!("{}: {}", #keys, <#tys as #ui::JsValue>::ts_type())),*
                            ];
                            ::std::format!("{{ {} }}", fields.join("; "))
                        },
                        doc: #doc,
                    }
                });
                name_arms.push(quote!(Self::#ident { .. } => #js));
                data_arms.push(quote! {
                    Self::#ident { #(#idents),* } => {
                        let mut map = ::nsh_core::ui::__private::JsonMap::new();
                        #(map.insert(::std::string::String::from(#keys), #ui::JsValue::to_json(#idents));)*
                        ::nsh_core::JsonValue::Object(map)
                    }
                });
            }
            Fields::Unnamed(f) => {
                return Err(syn::Error::new(
                    f.span(),
                    "an event variant has at most one unnamed field",
                ));
            }
        }
    }
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #ui::Event for #name #ty_generics #where_clause {
            fn spec() -> ::std::vec::Vec<#ui::EventSpec> {
                ::std::vec![#(#specs),*]
            }

            fn name(&self) -> &'static str {
                match self {
                    #(#name_arms,)*
                }
            }

            fn data(&self) -> ::nsh_core::JsonValue {
                match self {
                    #(#data_arms,)*
                }
            }
        }
    })
}

/// One `name => [Key, ...]` line of `style_helpers!`.
struct Helper {
    doc: String,
    name: Ident,
    keys: Vec<Ident>,
}

impl Parse for Helper {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let name: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;
        let content;
        bracketed!(content in input);
        let keys = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?
            .into_iter()
            .collect();
        Ok(Helper {
            doc: docs(&attrs),
            name,
            keys,
        })
    }
}

/// Tailwind's spacing scale: method suffix and points (4pt steps).
const STEPS: &[(&str, f32)] = &[
    ("0", 0.0),
    ("0p5", 2.0),
    ("1", 4.0),
    ("1p5", 6.0),
    ("2", 8.0),
    ("2p5", 10.0),
    ("3", 12.0),
    ("3p5", 14.0),
    ("4", 16.0),
    ("5", 20.0),
    ("6", 24.0),
    ("7", 28.0),
    ("8", 32.0),
    ("9", 36.0),
    ("10", 40.0),
    ("11", 44.0),
    ("12", 48.0),
    ("14", 56.0),
    ("16", 64.0),
    ("20", 80.0),
    ("24", 96.0),
    ("28", 112.0),
    ("32", 128.0),
    ("36", 144.0),
    ("40", 160.0),
    ("44", 176.0),
    ("48", 192.0),
    ("52", 208.0),
    ("56", 224.0),
    ("60", 240.0),
    ("64", 256.0),
    ("72", 288.0),
    ("80", 320.0),
    ("96", 384.0),
];

/// Internal to `nsh-core`: `name => [StyleKey, ...]` lines become a method
/// `name(length)` and one method per spacing step (`p_4()` is 16pt) of
/// the `Styled` trait.
#[proc_macro]
pub fn style_helpers(input: TokenStream) -> TokenStream {
    let helpers = parse_macro_input!(input with Punctuated::<Helper, Token![,]>::parse_terminated);
    let mut out = TokenStream2::new();
    for h in helpers {
        let Helper { doc, name, keys } = h;
        out.extend(quote! {
            #[doc = #doc]
            fn #name(mut self, length: impl ::core::convert::Into<crate::ui::Length>) -> Self {
                self.style().lengths(&[#(crate::style::StyleKey::#keys),*], length.into());
                self
            }
        });
        for (suffix, points) in STEPS {
            let method = format_ident!("{}_{}", name, suffix, span = Span::call_site());
            let step_doc = format!("{}: {points}pt.", doc.trim_end_matches('.'));
            out.extend(quote! {
                #[doc = #step_doc]
                fn #method(mut self) -> Self {
                    self.style().lengths(
                        &[#(crate::style::StyleKey::#keys),*],
                        crate::ui::Length::Points(#points),
                    );
                    self
                }
            });
        }
    }
    out.into()
}

/// Whether `ty` is `&mut Context<..>` (a method's context parameter).
fn is_context(ty: &Type) -> bool {
    match ty {
        Type::Reference(r) => {
            r.mutability.is_some()
                && matches!(last_segment(&r.elem), Some((id, _)) if id == "Context")
        }
        _ => false,
    }
}

/// `#[methods]` on an `impl` block of a component: each function becomes a
/// method of the component's ref in JS, named in camel case (`set_value`
/// is `setValue`), documented by its doc comment. A function takes `&self`
/// or `&mut self`, then [`JsValue`] parameters, and optionally a last
/// `cx: &mut Context<Self>`; it returns nothing (JS calls it and moves on;
/// the call is ordered with the props set before it) or a [`JsValue`] (JS
/// gets a `Promise`). The block's functions are JS's: keep helpers in
/// another `impl`. The component lists them with
/// `const METHODS: &'static [Method<Self>] = Self::JS_METHODS;`.
#[proc_macro_attribute]
pub fn methods(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(Span::call_site(), "`#[methods]` takes no arguments")
            .into_compile_error()
            .into();
    }
    let item = parse_macro_input!(item as ItemImpl);
    methods_impl(item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn methods_impl(item: ItemImpl) -> syn::Result<TokenStream2> {
    let ui = quote!(::nsh_core::ui);
    let self_ty = &item.self_ty;
    let (impl_generics, _, where_clause) = item.generics.split_for_impl();
    let mut entries = Vec::new();
    for member in &item.items {
        let ImplItem::Fn(f) = member else { continue };
        let ident = &f.sig.ident;
        let js = camel(&ident.to_string());
        let doc = docs(&f.attrs);
        let mut inputs = f.sig.inputs.iter();
        match inputs.next() {
            Some(FnArg::Receiver(r)) if r.reference.is_some() => {}
            _ => {
                return Err(syn::Error::new(
                    f.sig.span(),
                    "a JS method takes `&self` or `&mut self`",
                ));
            }
        }
        let mut params = Vec::new();
        let mut decode = Vec::new();
        let mut call_args = Vec::new();
        let mut takes_cx = false;
        let typed: Vec<_> = inputs
            .map(|arg| match arg {
                FnArg::Typed(t) => Ok(t),
                FnArg::Receiver(r) => Err(syn::Error::new(r.span(), "one receiver")),
            })
            .collect::<syn::Result<_>>()?;
        for (i, arg) in typed.iter().enumerate() {
            let ty = &arg.ty;
            if is_context(ty) {
                if i + 1 != typed.len() {
                    return Err(syn::Error::new(
                        arg.span(),
                        "the context is the last parameter",
                    ));
                }
                takes_cx = true;
                continue;
            }
            let Pat::Ident(name) = &*arg.pat else {
                return Err(syn::Error::new(
                    arg.pat.span(),
                    "a JS method's parameters are named",
                ));
            };
            let name = camel(&name.ident.to_string());
            let local = format_ident!("__arg{}", i);
            params.push(quote!((#name, <#ty as #ui::JsValue>::ts_type)));
            decode.push(quote! {
                let #local: #ty = <#ty as #ui::JsValue>::from_json(
                    args.next().unwrap_or(::nsh_core::JsonValue::Null),
                )
                .map_err(|e| #ui::PropError(::std::format!("`{}`: {}", #name, e)))?;
            });
            call_args.push(quote!(#local));
        }
        if takes_cx {
            call_args.push(quote!(cx));
        }
        let (returns, result) = match &f.sig.output {
            ReturnType::Default => (
                quote!(::core::option::Option::None),
                quote! {
                    this.#ident(#(#call_args),*);
                    ::core::result::Result::Ok(::nsh_core::JsonValue::Null)
                },
            ),
            ReturnType::Type(_, ty) => (
                quote!(::core::option::Option::Some(<#ty as #ui::JsValue>::ts_type)),
                quote! {
                    let value: #ty = this.#ident(#(#call_args),*);
                    ::core::result::Result::Ok(#ui::JsValue::to_json(&value))
                },
            ),
        };
        let unused_cx = (!takes_cx).then(|| quote!(let _ = cx;));
        let args_iter = if decode.is_empty() {
            quote!(let _ = args;)
        } else {
            quote!(let mut args = args.into_iter();)
        };
        entries.push(quote! {
            #ui::Method {
                name: #js,
                doc: #doc,
                params: &[#(#params),*],
                returns: #returns,
                call: |this, args, cx| {
                    #unused_cx
                    #args_iter
                    #(#decode)*
                    #result
                },
            }
        });
    }
    Ok(quote! {
        #item

        impl #impl_generics #self_ty #where_clause {
            /// The methods JS calls on the component's ref (`#[methods]`).
            #[doc(hidden)]
            pub const JS_METHODS: &'static [#ui::Method<Self>] = &[#(#entries),*];
        }
    })
}
