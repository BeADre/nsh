//! `nsh`: the command line with nsh's UI sets.

fn main() -> std::process::ExitCode {
    nsh_cli::main_with(vec![nsh_ui::registry(), nsh_ui_mobile::registry()])
}
