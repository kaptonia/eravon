use crate::house::seller::Said;
use eravon_literals::json::Out;
use crate::shell::Shell;
use eravon_link::identity::Records;

pub type Hist = Option<Result<(String, Records), Said>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    PayLink,
    Orders,
    Bo,
}

impl Page {
    pub fn step(self) -> crate::house::action::Action {
        match self {
            Page::PayLink => crate::house::action::Action::PlHistory,
            Page::Orders => crate::house::action::Action::OrderHistory,
            Page::Bo => crate::house::action::Action::BoHistory,
        }
    }
}

pub fn read(data: &str, me: &str, who: &str) -> Result<(String, Records), Said> {
    let who = who.trim().to_ascii_lowercase();
    let w = crate::house::orders::wire(data, me)?;
    let rs = eravon_scan::records::for_record(&w.t, &w.table, w.chain_id, &w.logic, &who).map_err(|e| Said::bad(&e.name(), e.detail()))?;
    Ok((who, rs))
}

pub fn say(got: &Result<(String, Records), Said>) -> String {
    match got {
        Ok((who, rs)) => {
            let h = eravon_link::identity::history(rs, who);
            format!("{} · 钥 {} · 文书 {} · 尝试 {}", who, h.keys.len(), h.documents.len(), h.attempts().count())
        }
        Err(s) => format!("{} · {}", s.name, s.detail),
    }
}

pub fn pending_of(rs: &Records, keys: &[String]) -> Option<String> {
    let last = keys.last()?;
    rs.book.declarations.iter().rev().find(|d| d.emitter.eq_ignore_ascii_case(last)).map(|d| d.to.to_ascii_lowercase()).filter(|to| !rs.book.accessions.iter().any(|x| x.emitter.eq_ignore_ascii_case(to)))
}

pub fn out(h: &Hist) -> Option<Out> {
    match h {
        Some(Ok((who, rs))) => {
            let x = eravon_link::identity::history(rs, who);
            let pending = pending_of(rs, &x.keys).unwrap_or_default();
            Some(Out::O(vec![("pending", Out::S(pending)), ("record", eravon_cli::history::answer(rs, &x)), ("who", Out::S(who.clone()))]))
        }
        Some(Err(s)) => Some(s.out()),
        None => None,
    }
}

fn open_on(sh: &Shell, page: Page) -> Option<String> {
    match page {
        Page::PayLink => sh.desk.sh.paylink.as_ref().map(|p| p.link.clone()),
        Page::Orders => sh.desk.sh.seller.orders.as_ref().and_then(|o| o.sel.clone()),
        Page::Bo => sh.desk.sh.buyer_orders.as_ref().and_then(|b| b.open.as_ref()).map(|o| o.instance.clone()),
    }
    .map(|x| x.to_ascii_lowercase())
}

pub fn ask(sh: &mut Shell, page: Page, me: &str, who: &str) -> Result<String, String> {
    let (data, me, who) = (sh.desk.data.clone(), me.to_string(), who.to_ascii_lowercase());
    let at = who.clone();
    let on = open_on(sh, page);
    let go = sh.house_spawn(crate::task::Kind::History, crate::house::desk::Site::Page, crate::house::flow::Ask::Asked, move |_| crate::house::landed::Landed::History(page, on, read(&data, &me, &at)));
    sh.started(go)?;
    if let Some(slot) = slot_of(sh, page) {
        *slot = None;
    }
    Ok(who)
}

fn slot_of(sh: &mut Shell, page: Page) -> Option<&mut Hist> {
    match page {
        Page::PayLink => sh.desk.sh.paylink.as_mut().map(|p| &mut p.history),
        Page::Orders => sh.desk.sh.seller.orders.as_mut().map(|o| &mut o.history),
        Page::Bo => sh.desk.sh.buyer_orders.as_mut().and_then(|b| b.open.as_mut()).map(|o| &mut o.history),
    }
}

pub fn land(sh: &mut Shell, page: Page, on: Option<String>, got: Result<(String, Records), Said>) {
    let mark = match page {
        Page::PayLink => crate::house::mark::FEATURE_P1,
        Page::Orders | Page::Bo => eravon_orders::mark::FEATURE,
    };
    if on.is_none() || open_on(sh, page) != on {
        sh.desk.tr.push(page.step(), mark, format!("未落 · {}", say(&got)));
        sh.desk.flush();
        return;
    }
    sh.desk.tr.push(page.step(), mark, say(&got));
    if let Some(slot) = slot_of(sh, page) {
        *slot = Some(got);
    }
    sh.desk.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_history_lands_only_where_what_it_was_asked_for_is_still_open() {
        let mut sh = crate::window::house::shell_headless();
        let (a, b) = ("eravon://open?e=a".to_string(), "eravon://open?e=b".to_string());
        sh.desk.sh.paylink = Some(crate::house::paylink::PayLink { link: a.clone(), ..Default::default() });
        let got = || Ok((format!("0x{}", "a1".repeat(20)), Records::default()));
        land(&mut sh, Page::PayLink, Some(a.clone()), got());
        assert!(sh.desk.sh.paylink.as_ref().is_some_and(|p| p.history.is_some()), "所问之链接仍开着:落");
        if let Some(p) = sh.desk.sh.paylink.as_mut() {
            p.history = None;
            p.link = b;
        }
        land(&mut sh, Page::PayLink, Some(a), got());
        assert!(sh.desk.sh.paylink.as_ref().is_some_and(|p| p.history.is_none()), "换开了别的链接:不落");
    }
}
