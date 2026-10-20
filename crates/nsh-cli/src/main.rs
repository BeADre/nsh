//! `nsh`: the command line with `nsh/base`.

fn main() -> std::process::ExitCode {
    nsh_cli::main_with(vec![nsh_base::registry()])
}
