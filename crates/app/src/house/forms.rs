use crate::action::Applied;
use crate::shell::Shell;

pub fn label_kept(label: &str) -> String {
    label.to_string()
}


pub fn fresh_kept(_shell: &mut Shell) -> Option<Applied> {
    None
}

