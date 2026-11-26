use eravon_literals::json::{In, Out};

use super::{arg, said_of, Run};
use crate::action::{Action as Fam, Applied, ImportForm, RestoreHow};
use crate::house::action::Action;
use crate::nav::{Progress, Step};
use crate::secret::Secret;
use crate::shell::Shell;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum From {
    KeyFile,
    Backup,
}

#[derive(Default)]
pub(super) struct Wiz {
    want: Option<Step>,
    pin: Secret,
    first: Option<Secret>,
    label: String,
    confirming: bool,
    from: Option<From>,
    file: String,
    file_pass: Secret,
    says: String,
}

fn way_out(sh: &Shell) -> bool {
    let identity = sh.identities.as_ref().map(|r| !r.rows.is_empty()).unwrap_or(false);
    !sh.vault.absent() && identity
}

fn is_wizard(act: Action) -> bool {
    matches!(act, Action::First | Action::WizardExit | Action::SetPin | Action::FirstLabel | Action::Way | Action::Reveal | Action::Verify | Action::Answer | Action::Cells | Action::PickFile | Action::FirstFilePass | Action::Land | Action::Selfcheck)
}

impl Wiz {
    fn now(&self, sh: &Shell) -> Option<Step> {
        let pr = Progress::of(sh);
        self.want.map(|w| pr.gate(w))
    }

    pub(super) fn rows(&self, sh: &Shell) -> Out {
        let pr = Progress::of(sh);
        let name = |s: Step| s.name().to_string();
        let done: Vec<Out> = Step::walk(pr.own.on).into_iter().filter(|s| pr.done(*s)).map(|s| Out::S(name(s))).collect();
        let mut m = vec![
            ("canExit", Out::B(way_out(sh))),
            ("confirming", Out::B(self.confirming)),
            ("done", Out::A(done)),
            ("filled", Out::N(self.pin.chars() as u64)),
            ("label", Out::S(self.label.clone())),
            ("next", Out::S(pr.first_open().map(name).unwrap_or_default())),
            ("open", Out::B(self.want.is_some())),
            ("says", Out::S(self.says.clone())),
            ("secondPass", Out::B(self.first.is_some())),
            ("step", Out::S(self.now(sh).map(name).unwrap_or_default())),
            ("newWords", Out::B(sh.new_words.is_some())),
        ];
        if let Some(f) = &sh.desk.first {
            m.push(("seat", Out::O(f.rows())));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }

    fn landed(&mut self, got: &Applied) -> String {
        let said = said_of(got);
        self.says = match got {
            Applied::Trouble(f) => f.said().to_string(),
            _ => String::new(),
        };
        said
    }
}

pub(super) fn step(w: &mut Wiz, sh: &mut Shell, r: &Run<'_>, act: Action, st: &In) -> Option<Result<String, String>> {
    if !is_wizard(act) {
        return None;
    }
    let _ = r;
    Some((|| -> Result<String, String> {
        let on_key = matches!(act, Action::FirstLabel | Action::Way | Action::Reveal | Action::Verify | Action::Answer | Action::Cells | Action::PickFile | Action::FirstFilePass);
        let drawn = Progress::of(sh).gate(Step::Key);
        if on_key && drawn != Step::Key {
            return Err(format!("向导这一步在 {}:身份那一步还画不出来", drawn.name()));
        }
        Ok(match act {
            Action::First => {
                let pr = Progress::of(sh);
                let want = match st.get("at").and_then(|x| x.as_str()) {
                    Some(n) => *Step::ALL.iter().find(|s| s.name() == n).ok_or_else(|| format!("向导里没有这一步:{n}"))?,
                    None => pr.first_open().unwrap_or(Step::Pin),
                };
                w.want = Some(want);
                sh.desk.wizard_up = Some(pr.gate(want).name());
                format!("向导 · 步 {}", pr.gate(want).name())
            }
            Action::WizardExit => {
                if !way_out(sh) {
                    return Err("还没有口令与身份,向导没有退出口".to_string());
                }
                if sh.new_words.is_some() {
                    if let crate::action::Applied::Trouble(f) = crate::action::apply_settled(sh, Fam::DropFresh) {
                        return Err(format!("{} · {}", f.said(), f.tail()));
                    }
                }
                *w = Wiz::default();
                sh.desk.wizard_up = None;
                "退出向导".to_string()
            }
            Action::SetPin => {
                for c in arg(st, "d")?.chars() {
                    if w.pin.chars() < crate::keybox::PIN_LEN {
                        w.pin.push(c);
                    }
                }
                if w.pin.chars() < crate::keybox::PIN_LEN {
                    return Ok(format!("键入 {} 位", w.pin.chars()));
                }
                let pin = std::mem::take(&mut w.pin);
                match w.first.take() {
                    None => {
                        w.first = Some(pin);
                        "口令 · 第一遍".to_string()
                    }
                    Some(first) => {
                        let got = crate::action::apply_settled(sh, Fam::SetPin { pin: first, again: pin });
                        format!("口令 · 第二遍 · {}", w.landed(&got))
                    }
                }
            }
            Action::FirstLabel => {
                w.label = st.get("v").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                format!("备注 · {}", w.label)
            }
            Action::Way => {
                let way = arg(st, "w")?;
                match way.as_str() {
                    "gen" => {
                        w.confirming = false;
                        let got = crate::action::apply_settled(sh, Fam::NewIdentity);
                        format!("新钥 · {}", w.landed(&got))
                    }
                    "keyFile" => {
                        w.from = Some(From::KeyFile);
                        "钥档".to_string()
                    }
                    "backup" => {
                        w.from = Some(From::Backup);
                        "整机备份".to_string()
                    }
                    other => return Err(format!("身份那一步没有这一式:{other}")),
                }
            }
            Action::Reveal => format!("十二词 · {}", if sh.new_words.is_some() { "在" } else { "没有" }),
            Action::Verify => {
                let fresh = sh.new_words.as_ref().ok_or_else(|| "新钥的十二词没生成".to_string())?;
                w.confirming = true;
                format!("核对 · 问第 {:?} 位", fresh.picks.map(|x| x + 1))
            }
            Action::Answer => {
                let fresh = sh.new_words.as_ref().ok_or_else(|| "新钥的十二词没生成".to_string())?;
                let picks = fresh.picks;
                let all = fresh.words();
                let answers = picks
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (*p, st.get(&format!("a{i}")).and_then(|x| x.as_str()).map(Secret::of).unwrap_or_else(|| all.get(*p).cloned().unwrap_or_default())))
                    .collect();
                let got = crate::action::apply_settled(sh, Fam::ConfirmIdentity { answers, label: w.label.clone() });
                w.confirming = false;
                format!("核对 · {}", w.landed(&got))
            }
            Action::Cells => {
                let got = crate::action::apply_settled(sh, Fam::ImportIdentity { form: ImportForm::Words(Secret::of(&arg(st, "w")?)), label: w.label.clone() });
                format!("导入 · {}", w.landed(&got))
            }
            Action::PickFile => {
                w.file = arg(st, "to")?;
                format!("选中 {}", w.file)
            }
            Action::FirstFilePass => {
                w.file_pass = Secret::of(&arg(st, "p")?);
                let (path, password) = (w.file.clone(), w.file_pass.clone());
                let fam = match w.from.ok_or_else(|| "先选钥档或整机备份".to_string())? {
                    From::KeyFile => Fam::ImportIdentity { form: ImportForm::Keystore { path, password }, label: w.label.clone() },
                    From::Backup => Fam::RestoreBackup { path, password, how: RestoreHow::FirstRun },
                };
                let got = crate::action::apply_settled(sh, fam);
                format!("文件口令 · {}", w.landed(&got))
            }
            Action::Selfcheck => eravon_vault::onboard::self_check(&sh.desk.data, sh.desk.seat().store()).map_err(|e| format!("{}:{}", e.name(), e.detail()))?,
            _ => return Err("落地一步已退场:向导的每一步各自落地".to_string()),
        })
    })())
}
