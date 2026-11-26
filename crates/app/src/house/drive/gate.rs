use eravon_literals::json::{In, Out};

use super::{arg, said_of, Run};
use crate::action::{Action as Fam, Applied};
use crate::house::action::Action;
use crate::house::gatecard::Card;
use crate::house::seam;
use crate::secret::Secret;
use crate::shell::Shell;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Route {
    #[default]
    Words,
    KeyFile,
    Backup,
}

impl Route {
    fn name(self) -> &'static str {
        match self {
            Route::Words => seam::ROUTE_WORDS,
            Route::KeyFile => seam::ROUTE_KEYFILE,
            Route::Backup => seam::ROUTE_BACKUP,
        }
    }

    fn of(name: &str) -> Option<Route> {
        [Route::Words, Route::Backup, Route::KeyFile].into_iter().find(|r| r.name() == name)
    }
}

#[derive(Default)]
pub(super) struct Gate {
    pin: Secret,
    first: Option<Secret>,
    words: Secret,
    file: String,
    file_pass: Secret,
    recover: bool,
    route: Option<Route>,
    reseal: bool,
    says: String,
    shake: bool,
}

pub(super) fn is_gate(act: Action) -> bool {
    matches!(
        act,
        Action::Pin | Action::Unlock | Action::Lock | Action::Forget | Action::Aside | Action::Words | Action::Recover | Action::RecoverCancel | Action::UseKey | Action::KeyFile | Action::Route | Action::Pick | Action::FilePass | Action::TakeOver
    )
}

impl Gate {
    fn card(&self, sh: &Shell) -> Card {
        Card::of(&sh.vault, self.recover, self.reseal, sh.vault_recoverable, sh.primary.as_ref().map(|(_, k)| *k))
    }

    fn route(&self, sh: &Shell) -> Route {
        self.route.unwrap_or(match sh.primary.as_ref().map(|(_, k)| *k) {
            Some(crate::keybox::PrimaryKind::KeyFile) => Route::KeyFile,
            _ => Route::Words,
        })
    }

    fn left(sh: &Shell) -> u64 {
        match &sh.vault {
            crate::shell::Vault::Read(crate::keybox::State::Locked { wrong }) => u64::from(eravon_literals::constants::PASSCODE_MAX_ATTEMPTS).saturating_sub(*wrong),
            crate::shell::Vault::Read(crate::keybox::State::LockedOut) => 0,
            _ => u64::from(eravon_literals::constants::PASSCODE_MAX_ATTEMPTS),
        }
    }

    pub(super) fn rows(&self, sh: &Shell) -> Out {
        let mut m = vec![
            ("attemptsLeft", Out::N(Gate::left(sh))),
            ("card", Out::S(self.card(sh).name().to_string())),
            ("filled", Out::N(self.pin.chars() as u64)),
            ("open", Out::B(sh.unlocked())),
            ("route", Out::S(self.route(sh).name().to_string())),
            ("says", Out::S(self.says.clone())),
            ("shake", Out::B(self.shake)),
            ("wordsFilled", Out::B(eravon_vault::words::filled(self.words.expose()))),
        ];
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }

    fn landed(&mut self, got: &Applied) -> bool {
        match got {
            Applied::Unlocked | Applied::Recovered | Applied::KeyboxReset | Applied::BackupRestored(_) => {
                *self = Gate::default();
                true
            }
            Applied::Trouble(f) => {
                self.first = None;
                self.shake = true;
                if f.which() == Some(crate::fault::Known::KdfBelowFloor) {
                    self.reseal = true;
                    self.shake = false;
                }
                self.says = f.said().to_string();
                false
            }
            _ => false,
        }
    }
}

pub(super) fn step(g: &mut Gate, sh: &mut Shell, r: &Run<'_>, act: Action, st: &In) -> Option<Result<String, String>> {
    if !is_gate(act) {
        return None;
    }
    let word = |k: crate::house::i18n::Key| crate::house::i18n::get(r.lang, k).0;
    Some((|| -> Result<String, String> {
        Ok(match act {
            Action::Pin => {
                for c in arg(st, "d")?.chars() {
                    if g.pin.chars() < crate::keybox::PIN_LEN {
                        g.pin.push(c);
                    }
                }
                g.shake = false;
                format!("键入 {} 位", g.pin.chars())
            }
            Action::Unlock => {
                let card = g.card(sh);
                let pin = std::mem::take(&mut g.pin);
                let fam = match card {
                    Card::Passcode => Some(Fam::Unlock { pin }),
                    Card::Reseal => Some(Fam::Reseal { pin }),
                    Card::Reset => Some(Fam::ResetEmptyKeybox),
                    Card::Recover | Card::KeyFile | Card::Dead => match g.first.take() {
                        None => {
                            g.first = Some(pin);
                            return Ok(format!("{} · 第一遍", card.name()));
                        }
                        Some(first) => Some(match g.route(sh) {
                            Route::Words => Fam::RecoverWords { words: g.words.clone(), pin: first, again: pin },
                            Route::KeyFile => Fam::RecoverKeystore { path: g.file.clone(), password: g.file_pass.clone(), pin: first, again: pin },
                            Route::Backup => Fam::RestoreBackup { path: g.file.clone(), password: g.file_pass.clone(), how: crate::action::RestoreHow::Locked { pin: first, again: pin } },
                        }),
                    },
                    Card::None | Card::Damaged => None,
                };
                let Some(fam) = fam else { return Ok(format!("{} · 拒 · 门上没有可交的卡", card.name())) };
                let got = crate::action::apply_settled(sh, fam);
                let ok = g.landed(&got);
                format!("{} · {} · 还可再试 {} 次", card.name(), if ok { "过" } else { "拒" }, Gate::left(sh))
            }
            Action::Lock => {
                let got = crate::action::apply_settled(sh, Fam::Lock);
                *g = Gate::default();
                format!("{} · 门 {} · {}", word(crate::house::i18n::Key::PinLockedTitle), g.card(sh).name(), said_of(&got))
            }
            Action::Aside => {
                let got = crate::action::apply_settled(sh, Fam::SetMachineAside);
                *g = Gate::default();
                format!("门 {} · {}", g.card(sh).name(), said_of(&got))
            }
            Action::Forget => {
                g.recover = true;
                g.says.clear();
                word(crate::house::i18n::Key::PinForgot)
            }
            Action::KeyFile => {
                g.recover = true;
                g.route = Some(Route::KeyFile);
                format!("{} · 卡 {}", word(crate::house::i18n::Key::GateFromKeyFile), g.card(sh).name())
            }
            Action::Route => {
                let name = arg(st, "r")?;
                let route = Route::of(&name).ok_or_else(|| format!("不认得的恢复路 {name}"))?;
                g.recover = true;
                g.route = Some(route);
                format!("{} · 卡 {}", route.name(), g.card(sh).name())
            }
            Action::RecoverCancel | Action::Recover => {
                g.recover = act == Action::Recover;
                g.first = None;
                format!("卡 {}", g.card(sh).name())
            }
            Action::Pick => {
                let to = arg(st, "to")?;
                if std::path::Path::new(&to).is_dir() {
                    format!("进 {to}")
                } else {
                    g.file = to.clone();
                    format!("选中 {to}")
                }
            }
            Action::FilePass => {
                let p = Secret::of(&arg(st, "p")?);
                if p.overflowed() {
                    return Err(format!("文件口令超过 {} 字", crate::secret::CAP));
                }
                g.file_pass = p;
                format!("{} · 文件口令已填", g.route(sh).name())
            }
            Action::Words => {
                g.words = Secret::of(&arg(st, "w")?);
                format!("十二格 · 满 {}", eravon_vault::words::filled(g.words.expose()))
            }
            Action::UseKey => {
                let who = arg(st, "who")?;
                match sh.desk.session.key_for(&who) {
                    Ok(_) => format!("{} · 过", word(crate::house::i18n::Key::GateUseKey)),
                    Err(e) => {
                        sh.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: crate::house::said_of(&e).detail, next: word(crate::house::i18n::Key::ErrEVAULTLOCKEDNext), detail: Some(e.name().to_string()) });
                        format!("{} · {}", word(crate::house::i18n::Key::GateUseKey), e.name())
                    }
                }
            }
            _ => {
                let got = crate::action::apply_settled(sh, Fam::TakeWriter);
                format!("{} · {}", word(crate::house::i18n::Key::SetTakeWriter), said_of(&got))
            }
        })
    })())
}
