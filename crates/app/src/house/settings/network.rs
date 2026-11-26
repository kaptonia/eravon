use super::*;

pub(super) fn wire(data: &str, me: &str) -> Result<(Auto, Table), Said> {
    let nodes = eravon_vault::store::endpoints(data, me).map_err(|e| bad(e.name(), e.detail()))?;
    if nodes.is_empty() {
        return Err(bad("E_S9_NO_ENDPOINTS", crate::house::i18n::t(crate::house::i18n::Key::SetNoEndpoints)));
    }
    let table = Table::new(nodes).map_err(|e| bad(e.name(), e.detail()))?;
    let t = Auto::web_roots().map_err(|e| bad(e.name(), e.detail()))?;
    Ok((t, table))
}

pub fn verbs(seat: Seat) -> Vec<&'static str> {
    eravon_cli::schema::verb_names(seat == Seat::Buyer)
}


pub fn zk_state(home: &str, manual: &str) -> ZkState {
    match eravon_zikaron::survey(home, Some(manual)) {
        Ok(s) => ZkState::Linked {
            at: s.link.at.clone(),
            layer: s.link.layer.name().to_string(),
            kits: s.kits.len(),
            absent: s.absent.map(|a| (a.name().to_string(), a.detail())),
        },
        Err(e) => zk_fold(e),
    }
}

pub fn zk_kits(home: &str, manual: &str, on: bool) -> Result<Vec<(String, String, u64)>, crate::house::seller::Said> {
    use crate::house::seller::Said;
    if !on {
        return Err(Said::bad(crate::house::orders::ZK_OFF, String::new()));
    }
    match eravon_zikaron::survey(home, Some(manual)) {
        Ok(s) => Ok(s.kits.into_iter().map(|k| (k.id, k.note_md, k.contents.len() as u64)).collect()),
        Err(e) => Err(match zk_fold(e) {
            ZkState::Absent { name, detail } => Said { tone: crate::house::seller::Tone::Note, name, detail },
            ZkState::Bad { name, detail } => Said::bad(&name, detail),
            ZkState::Linked { at, .. } => Said::bad("E_ZK_IO", at),
        }),
    }
}

pub fn zk_away(s: &crate::house::seller::Said) -> Option<String> {
    let absent = s.name == crate::house::orders::ZK_OFF || s.tone == crate::house::seller::Tone::Note;
    absent.then(|| {
        let (w, known) = crate::house::i18n::get_named(crate::lang::lang(), &format!("set.zkAbsent.{}", s.name));
        if known { format!("ZIKARON · {w}") } else { crate::house::i18n::t(crate::house::i18n::Key::OrdersZkOff) }
    })
}

pub(super) fn zk_fold(e: eravon_zikaron::fault::Fault) -> ZkState {
    match e.absent() {
        Some(a) => ZkState::Absent { name: a.name().to_string(), detail: a.detail() },
        None => ZkState::Bad { name: e.name().to_string(), detail: e.detail() },
    }
}


pub(super) fn zk_out(z: &ZkState) -> Out {
    let s = |x: &str| Out::S(x.to_string());
    match z {
        ZkState::Linked { at, layer, kits, absent } => {
            let mut m = vec![("at", s(at)), ("kits", Out::N(*kits as u64)), ("layer", s(layer)), ("state", s("linked"))];
            if let Some((n, d)) = absent {
                m.push(("absent", Out::O(vec![("detail", s(d)), ("name", s(n))])));
            }
            Out::O(m)
        }
        ZkState::Absent { name, detail } => Out::O(vec![("detail", s(detail)), ("name", s(name)), ("state", s("absent"))]),
        ZkState::Bad { name, detail } => Out::O(vec![("detail", s(detail)), ("name", s(name)), ("state", s("bad"))]),
    }
}

impl Settings {
    pub fn endpoint_specs(chain: u64, list: &[String]) -> Result<String, Said> {
        eravon_cli::deskops::home::endpoint_specs(chain, list).map_err(|n| bad(&n.name, n.detail()))
    }

    pub fn endpoints_set(&mut self, data: &str, env: &Env, got: Result<(), Said>, word: &str) {
        self.said = Some(match got {
            Ok(()) => ok(word),
            Err(e) => e,
        });
        self.me.clear();
        self.reload(data, env);
    }

    pub fn set_zikaron(&mut self, data: &str, env: &Env, dir: &str, now: u64, word: &str) {
        self.said = Some(match eravon_cli::deskops::network::set_zikaron(data, seam::WRITER, now, &self.me, dir) {
            Ok(()) => ok(word),
            Err(n) => bad(&n.name, n.detail()),
        });
        self.me.clear();
        self.reload(data, env);
    }

    pub fn set_past_cases(&mut self, data: &str, env: &Env, on: bool, now: u64, word: &str) {
        self.said = Some(match eravon_cli::deskops::network::set_past_cases(data, seam::WRITER, now, on) {
            Ok(()) => ok(word),
            Err(n) => bad(&n.name, n.detail()),
        });
        self.reload(data, env);
    }

    pub fn set_zk_link(&mut self, data: &str, env: &Env, on: bool, now: u64, word: &str) {
        self.said = Some(match eravon_cli::deskops::network::set_zikaron_link(data, seam::WRITER, now, &self.me, on) {
            Ok(()) => ok(word),
            Err(n) => bad(&n.name, n.detail()),
        });
        self.me.clear();
        self.reload(data, env);
    }
}

pub fn nodes_of(text: &str) -> Vec<String> {
    eravon_cli::deskops::home::nodes_of(text)
}

pub fn asked(s: &crate::settings::Settings) -> usize {
    let on = eravon_vault::store::chain_of(s);
    crate::chainx::nodes_of(s).into_iter().filter(|e| e.chain == on).count()
}
