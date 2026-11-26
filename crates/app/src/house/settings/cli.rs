use eravon_os::cli_path::{self, State};

fn beside() -> Result<std::path::PathBuf, State> {
    cli_path::beside_this_program().ok_or_else(|| State::Unsupported("这个程序的位置读不出来".into()))
}

pub fn cli_read() -> State {
    match beside() {
        Ok(cli) => cli_path::state(&cli),
        Err(s) => s,
    }
}

pub fn cli_word(s: &State) -> (&'static str, String) {
    match s {
        State::On => ("on", String::new()),
        State::Off => ("off", String::new()),
        State::Taken(what) => ("taken", what.clone()),
        State::Provided => ("provided", String::new()),
        State::Unsupported(why) => ("unsupported", why.clone()),
    }
}
