// `nsh/ui/mobile`: the mobile UI. Its components and modules are written in
// Rust (`crates/nsh-ui-mobile`, one look on iOS and Android): their JS face is
// generated. Navigation stacks and modals hold the app's routes in Solid.

export * from "./components.gen";
export * from "./navigation";
