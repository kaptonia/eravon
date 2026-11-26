use eravon_literals::json::{In, Out};

use super::arg;
use crate::action::{Action as Fam, Applied};
use crate::house::action::Action;
use crate::secret::Secret;
use crate::shell::{CliAloneNow, Shell};

pub(super) fn step(shown: &mut Vec<Secret>, sh: &mut Shell, act: Action, st: &In) -> Option<Result<String, String>> {
    Some(match act {
        Action::SetCliPassAdd => {
            let note = st.get("note").and_then(|x| x.as_str()).unwrap_or_default().to_string();
            let pin = match arg(st, "pin") {
                Ok(p) => Secret::of(&p),
                Err(e) => return Some(Err(e)),
            };
            match crate::action::apply_settled(sh, Fam::CliPassAdd { note, pin }) {
                Applied::CliPassAdded { pass, row } => {
                    shown.push(pass);
                    Ok(format!("{} · {} · {}", act.name(), row.tag, row.note))
                }
                other => Ok(format!("{} · {}", act.name(), super::said_of(&other))),
            }
        }
        Action::SetCliPassRead => {
            sh.reread_credential();
            Ok(format!("{} · {}", act.name(), alone_word(sh.cli_alone_now())))
        }
        _ => return None,
    })
}

fn alone_word(now: CliAloneNow) -> &'static str {
    match now {
        CliAloneNow::Off => "off",
        CliAloneNow::Keyless => "keyless",
        CliAloneNow::Keyed(_) => "keyed",
    }
}

pub(super) fn rows(sh: &Shell, shown: &[Secret]) -> Out {
    let row = |r: &eravon_seal::credential::Row| Out::O(vec![("added", Out::N(r.added)), ("note", Out::S(r.note.clone())), ("tag", Out::S(r.tag.clone()))]);
    Out::O(vec![
        ("alone", Out::S(alone_word(sh.cli_alone_now()).to_string())),
        ("rows", Out::A(sh.passes.iter().map(row).collect())),
        ("shown", Out::A(shown.iter().map(|k| Out::S(k.expose().to_string())).collect())),
    ])
}
