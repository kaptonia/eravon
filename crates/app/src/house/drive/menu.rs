use eravon_literals::json::{In, Out};

use super::{arg, said_of};
use crate::action::Action as Fam;
use crate::house::action::Action;
use crate::shell::Shell;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    address: String,
    label: String,
    kind: String,
    primary: bool,
    current: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Menu {
    rows: Vec<Row>,
    kb: Option<usize>,
    hover: Option<usize>,
    fresh: bool,
}

impl Menu {
    fn home(&self) -> Option<usize> {
        self.rows.iter().position(|r| r.current)
    }

    fn lens(&self) -> Option<usize> {
        self.hover.or(self.kb).or_else(|| self.home())
    }


    fn step(&mut self, down: bool) {
        let n = self.rows.len();
        if n == 0 {
            return;
        }
        let at = self.kb.or_else(|| self.home()).unwrap_or(0);
        self.kb = Some(if down { (at + 1) % n } else { (at + n - 1) % n });
        self.hover = None;
    }

    pub(super) fn rows(&self) -> Out {
        let rows = self
            .rows
            .iter()
            .map(|r| Out::O(vec![("address", Out::S(r.address.clone())), ("current", Out::B(r.current)), ("kind", Out::S(r.kind.clone())), ("label", Out::S(r.label.clone())), ("primary", Out::B(r.primary))]))
            .collect();
        let mut m = vec![("fresh", Out::B(self.fresh)), ("rows", Out::A(rows))];
        if let Some(l) = self.lens() {
            m.push(("lens", Out::N(l as u64)));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }
}

pub(super) fn step(menu: &mut Option<Menu>, sh: &mut Shell, act: Action, st: &In) -> Option<Result<String, String>> {
    if !matches!(act, Action::IdOpen | Action::IdClose | Action::IdMove | Action::IdHover | Action::IdEnter | Action::IdItem) {
        return None;
    }
    let lang = crate::lang::lang();
    Some((|| -> Result<String, String> {
        Ok(match act {
            Action::IdOpen => {
                let rows: Vec<Row> = crate::house::settings::keys(&sh.desk.data, sh.desk.seat())
                    .map_err(|e| format!("{}:{}", e.name, e.detail))?
                    .into_iter()
                    .map(|k| Row { address: k.address, label: k.label, kind: crate::house::i18n::get_named(lang, crate::house::settings::kind_key(&k.kind)).0, primary: k.primary, current: k.current })
                    .collect();
                let n = rows.len();
                *menu = Some(Menu { rows, kb: None, hover: None, fresh: true });
                format!("菜单 · {n} 把钥")
            }
            Action::IdClose => {
                *menu = None;
                "菜单已关".to_string()
            }
            Action::IdMove => {
                let m = menu.as_mut().ok_or_else(|| "菜单没开".to_string())?;
                m.fresh = false;
                m.step(arg(st, "d")? == "down");
                format!("镜头框 · {:?}", m.lens())
            }
            Action::IdHover => {
                let m = menu.as_mut().ok_or_else(|| "菜单没开".to_string())?;
                m.fresh = false;
                m.hover = st.get("i").and_then(|x| x.as_str()).and_then(|x| x.parse::<usize>().ok()).filter(|i| *i < m.rows.len());
                format!("镜头框 · {:?}", m.lens())
            }
            Action::IdEnter => {
                let m = menu.take().ok_or_else(|| "菜单没开".to_string())?;
                match m.lens().and_then(|i| m.rows.get(i)).filter(|r| !r.current).map(|r| r.address.clone()) {
                    None => "仍是正在使用的那一把".to_string(),
                    Some(a) => {
                        let (row, _) = eravon_vault::records::seat_of(&a).map_err(|f| format!("{} · {}", f.said(), f.tail()))?;
                        let got = crate::action::apply_settled(sh, Fam::SwitchIdentity { id: row.id });
                        format!("{} · {a} · {}", crate::house::i18n::get(lang, crate::house::i18n::Key::SetSwitched).0, said_of(&got))
                    }
                }
            }
            _ => {
                let i = arg(st, "i")?;
                if !crate::house::seam::ID_ITEMS.contains(&i.as_str()) {
                    return Err(format!("菜单上没有这一项:{i}"));
                }
                *menu = None;
                let got = crate::action::apply_settled(sh, Fam::Show(crate::shell::Page::Identity));
                format!("{i} · 身份页 · {}", said_of(&got))
            }
        })
    })())
}
