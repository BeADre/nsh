// `nsh/ui/mobile`: the mobile UI. Navigation stacks, modals, safe areas, the
// software keyboard, the status bar and system dialogs (natively backed, in
// this directory), and the components, written in Rust (`crates/nsh-ui-mobile`,
// one look on iOS and Android): their glue is generated from their specs.

export * from "./components.gen";
export * from "./keyboard";
export * from "./navigation";
export * from "./safe-area";
export * from "./system";
