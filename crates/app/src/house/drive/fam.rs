use eravon_literals::json::In;

use super::{arg, said_of, Run};
use crate::action::{Action as Fam, ImportForm, RestoreHow};
use crate::house::action::Action;
use crate::secret::Secret;
use crate::shell::Shell;

fn opt(st: &In, key: &str) -> String {
    st.get(key).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

fn secret(st: &In, key: &str) -> Result<Secret, String> {
    Ok(Secret::of(&arg(st, key)?))
}

pub(crate) fn id_of(addr: &str) -> Result<String, String> {
    eravon_vault::records::seat_of(addr).map(|(row, _)| row.id).map_err(|f| format!("{} · {}", f.said(), f.tail()))
}

fn on_off(v: &str) -> Result<bool, String> {
    match v {
        "on" | "true" => Ok(true),
        "off" | "false" => Ok(false),
        other => Err(format!("开关只收 on 或 off:{other}")),
    }
}

fn new_chain(sh: &mut Shell, st: &In) -> Result<(), String> {
    let c = opt(st, "chain");
    if c.is_empty() {
        return Ok(());
    }
    let n = eravon_cli::deskops::network::known(&c).map_err(|n| format!("{} · {}", n.name, n.detail()))?;
    sh.desk.new_chain = Some(n.standard.chain_id);
    Ok(())
}

pub(super) fn step(sh: &mut Shell, _r: &Run<'_>, act: Action, st: &In) -> Option<Result<String, String>> {
    match fam_of(sh, act, st)? {
        Err(e) => Some(Err(e)),
        Ok(None) => Some(Ok(format!("{} · 窗面的单,门上没有它的状态", act.name()))),
        Ok(Some(f)) => {
            let got = crate::action::apply_settled(sh, f);
            let (data, env) = (sh.desk.data.clone(), crate::house::settings::Env { home: sh.desk.sh.seller.field.home.clone() });
            if let Some(st) = sh.desk.sh.seller.settings.as_mut() {
                st.reload(&data, &env);
            }
            Some(Ok(format!("{} · {}", act.name(), said_of(&got))))
        }
    }
}

pub(crate) fn fam_of(sh: &mut Shell, act: Action, st: &In) -> Option<Result<Option<Fam>, String>> {
    let fam = (|| -> Result<Option<Fam>, String> {
        Ok(Some(match act {
            Action::SetSwitch => Fam::SwitchIdentity { id: id_of(&arg(st, "a")?)? },
            Action::SetRename => Fam::NameIdentity { id: id_of(&arg(st, "a")?)?, label: opt(st, "label") },
            Action::SetRemove => Fam::DeleteIdentity { id: id_of(&arg(st, "a")?)?, pin: secret(st, "pin")? },
            Action::SetAutolock => {
                let on = match opt(st, "on").as_str() {
                    "" => sh.machine.auto_lock,
                    v => on_off(v)?,
                };
                let secs = match opt(st, "minutes").as_str() {
                    "" => sh.machine.auto_lock_secs,
                    m => m.parse::<u64>().map_err(|_| format!("闲置时长要是整数:{m}"))? * 60,
                };
                Fam::SetAutoLock { on, secs }
            }
            Action::SetBackup => Fam::ExportBackup { pin: secret(st, "pin")?, password: secret(st, "pw")?, again: secret(st, "again")?, dir: arg(st, "dir")? },
            Action::SetBackupRestore => Fam::RestoreBackup { path: arg(st, "path")?, password: secret(st, "pw")?, how: RestoreHow::Settings { pin: secret(st, "pin")? } },
            Action::ReadCliPath => Fam::ReadCliPath,
            Action::SetCliPath => Fam::SetCliPath { on: on_off(&arg(st, "v")?)? },
            Action::SetCliDesktop => Fam::SetCliDesktop { on: on_off(&arg(st, "v")?)? },
            Action::SetCliAnchor => {
                let v = arg(st, "v")?;
                let to = crate::machine::CliAnchor::ALL.into_iter().find(|x| x.as_str() == v).ok_or_else(|| format!("v 只认 send 或 queue:{v}"))?;
                Fam::SetCliAnchor { to }
            }
            Action::SetCliAlone => match opt(st, "v").as_str() {
                "off" => Fam::CliAloneOff,
                _ => {
                    let lasts = match arg(st, "secs")?.as_str() {
                        "never" => crate::machine::Lasts::Never,
                        n => crate::machine::Lasts::Secs(n.parse::<u64>().map_err(|_| format!("有效期要是秒数或 never:{n}"))?),
                    };
                    Fam::SetCliAlone { lasts, pin: secret(st, "pin")? }
                }
            },
            Action::SetCliPassDrop => Fam::CliPassDrop { tag: arg(st, "tag")? },
            Action::SetRestMode => Fam::SetRestMode { on: on_off(&arg(st, "v")?)? },
            Action::SetRestAdd => Fam::AddRest { base: arg(st, "base")?, key: secret(st, "key")? },
            Action::SetRestRemove => Fam::RemoveRest { base: arg(st, "base")? },
            Action::SetNew => {
                sh.new_label = Some(opt(st, "label"));
                new_chain(sh, st)?;
                Fam::NewIdentity
            }
            Action::SetNewVerify => {
                let fresh = sh.new_words.as_ref().ok_or_else(|| "新钥的十二词没生成".to_string())?;
                let picks = fresh.picks;
                let words: Vec<Secret> = match st.get("w").and_then(|v| v.as_str()) {
                    Some(v) => v.split(',').map(|s| Secret::of(s.trim())).collect(),
                    None => {
                        let all = fresh.words();
                        picks.iter().map(|i| all.get(*i).cloned().unwrap_or_default()).collect()
                    }
                };
                let answers = picks.iter().copied().zip(words).collect();
                Fam::ConfirmIdentity { answers, label: sh.new_label.clone().unwrap_or_default() }
            }
            Action::SetNewClose => Fam::DropFresh,
            Action::SetImport => {
                new_chain(sh, st)?;
                Fam::ImportIdentity { form: ImportForm::Words(secret(st, "words")?), label: opt(st, "label") }
            }
            Action::SetRestore => {
                new_chain(sh, st)?;
                Fam::ImportIdentity { form: ImportForm::Keystore { path: arg(st, "path")?, password: secret(st, "fp")? }, label: opt(st, "label") }
            }
            Action::SetPasscode => Fam::ChangePin { old: secret(st, "cur")?, pin: secret(st, "next")?, again: secret(st, "again")? },
            Action::SetPhrase => Fam::RevealWords { pin: secret(st, "pin")? },
            Action::SetPhraseClose => Fam::HideWords,
            Action::SetExport => Fam::BackupKey { pin: secret(st, "pin")?, password: secret(st, "fp")?, again: secret(st, "again")?, dir: arg(st, "dir")? },
            Action::SetLang => {
                let v = arg(st, "v")?;
                Fam::SetLang { lang: crate::lang::Lang::ALL.into_iter().find(|l| l.as_str() == v).ok_or_else(|| format!("认不出这一种语言:{v}"))? }
            }
            Action::SetZone => {
                let v = arg(st, "v")?;
                Fam::SetZone { zone: crate::when::Zone::parse(&v).ok_or_else(|| format!("认不出这一种时区:{v}"))? }
            }
            Action::SetEvery => Fam::SetRefreshEvery { typed: arg(st, "v")? },
            Action::SetWay => Fam::SetProxy { choice: arg(st, "to")? },
            Action::SetLook => Fam::SetAppearance { appearance: arg(st, "v")? },
            Action::SetSwitchOpen | Action::SetKey | Action::SetKeyClose | Action::SetNewReveal | Action::SetNewAsk => return Ok(None),
            _ => return Err(String::new()),
        }))
    })();
    match fam {
        Err(e) if e.is_empty() => None,
        other => Some(other),
    }
}
