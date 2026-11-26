#![forbid(unsafe_code)]

use eravon_cli::args::Args;
use eravon_cli::{out, verbs};

fn main() {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let a = Args::read(argv);
    out::emit(verbs::run(&a))
}
