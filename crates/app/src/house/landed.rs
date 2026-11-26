use crate::house::desk::{Flying, Remind, Site};
use crate::house::flow::{origin_keeps, sync_done};
use crate::house::merge::{land as merge_in, Pass};
use crate::house::seller::{Page, Said, Tone};
use crate::house::shell::{Seat, Toast};
use crate::shell::Shell;
use crate::task::Kind;

#[derive(Clone)]
pub enum Landed {
    Sync(Option<Box<crate::house::sync::Tick>>, Option<eravon_chain::fault::Fault>),
    Orders(Box<Pass<crate::house::orders::Orders>>),
    Goods(Box<Pass<crate::house::seller::Goods>>),
    Cands(Box<Pass<crate::house::seller::Cands>>),
    Home(Box<Pass<crate::house::home::Home>>),
    Remind(Vec<(Remind, Option<String>)>),
    PayLink(Box<Pass<crate::house::paylink::PayLink>>),
    Bo(Box<Pass<crate::house::buyer_orders::BOrders>>),
    BHome(Box<Pass<crate::house::buyer_home::BHome>>),
    Wallet(Box<Pass<crate::house::wallet::Wallet>>),
    Settings(Box<Pass<crate::house::settings::Settings>>),
    Export(Box<Pass<crate::house::export::Export>>),
    FirstSeat(eravon_store::vault::Seat, Result<eravon_vault::onboard::Born, Said>),
    FirstPayout(Result<Result<usize, Said>, Said>),
    Direct(crate::house::action::Action, Said),
    History(crate::house::history::Page, Option<String>, Result<(String, eravon_link::identity::Records), Said>),
}

impl std::fmt::Debug for Landed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let k = match self {
            Landed::Sync(..) => "sync",
            Landed::Orders(_) => "orders",
            Landed::Goods(_) => "goods",
            Landed::Cands(_) => "cands",
            Landed::Home(_) => "home",
            Landed::Remind(_) => "remind",
            Landed::PayLink(_) => "paylink",
            Landed::Bo(_) => "bo",
            Landed::BHome(_) => "bhome",
            Landed::Wallet(_) => "wallet",
            Landed::Settings(_) => "settings",
            Landed::Export(_) => "export",
            Landed::History(..) => "history",
            Landed::FirstSeat(..) => "first-seat",
            Landed::FirstPayout(_) => "first-payout",
            Landed::Direct(..) => "direct",
        };
        write!(f, "Landed::{k}")
    }
}

fn toast_of(s: &Said) -> Toast {
    if s.tone == Tone::Bad {
        crate::house::sync::said_toast(s)
    } else {
        Toast { tone: s.tone, head: s.detail.clone(), next: String::new(), detail: None }
    }
}

pub(crate) fn land(sh: &mut Shell, kind: Kind, got: Landed, stale: bool, fl: Flying, failed: Option<eravon_chain::fault::Fault>) {
    sh.desk.rows_gen += 1;
    let Flying { site, act } = fl;
    let wrote = act || matches!(&got, Landed::Orders(o) if o.got.scan.is_some()) || matches!(&got, Landed::FirstSeat(..) | Landed::FirstPayout(_) | Landed::Direct(..));
    if wrote && !stale {
        crate::house::remeasure(sh);
    }
    match got {
        Landed::FirstSeat(seat, got) => crate::house::exec::first::land_seat(sh, seat, got),
        Landed::FirstPayout(got) => crate::house::exec::first::land_payout(sh, got),
        Landed::History(page, on, got) => crate::house::history::land(sh, page, on, got),
        Landed::Direct(step, said) => {
            sh.desk.tr.push(step, crate::house::mark::FEATURE, format!("{} · {}", said.name, said.detail));
            let tone = said.tone;
            sh.desk.sh.toast(Toast { tone, head: said.detail.clone(), next: String::new(), detail: (tone == Tone::Bad).then(|| said.name.clone()) });
            let me = sh.desk.addr.clone();
            let mut v = sh.desk.sh.queue.take().unwrap_or_default();
            v.direct = Some(crate::house::queue::direct_of(&sh.desk.session, &me));
            v.open_direct = v.open_direct.filter(|tx| matches!(&v.direct, Some(Ok(d)) if d.iter().any(|x| x.txs.contains(tx))));
            v.said = Some(said);
            sh.desk.sh.queue = Some(v);
            sh.site_done(site, tone != Tone::Bad);
            sh.desk.flush();
        }
        Landed::Sync(tick, fault) => land_sync(sh, stale, tick, fault.or(failed)),
        Landed::Remind(sent) => land_remind(sh, sent, failed),
        Landed::Home(h) => {
            if stale {
                sh.reenter_read(Kind::Home);
                return;
            }
            if let Some(e) = failed {
                sh.desk.sh.toast_read("home", crate::house::sync::fault_toast(&e));
            }
            match &h.got.toast {
                Some(t) => sh.desk.sh.toast_read("home", Toast { tone: Tone::Bad, head: t.head.clone(), next: t.next.clone(), detail: t.detail.clone() }),
                None => sh.desk.sh.read_ok("home"),
            }
            let me = h.got.me.clone();
            sh.desk.queue_remind(&me, &h.got.system);
            merge_in(&mut sh.desk.sh.seller.home, *h);
            sh.desk.flush();
        }
        Landed::BHome(h) => {
            if stale {
                sh.reenter_read(Kind::BHome);
                return;
            }
            let mut got = Some(*h);
            let mut whole = None;
            if let Some(e) = failed {
                sh.desk.sh.toast_read("bhome", crate::house::sync::fault_toast(&e));
                let me = sh.desk.addr.to_ascii_lowercase();
                let kept = sh.desk.sh.buyer_home.as_ref().is_some_and(|x| x.me.eq_ignore_ascii_case(&me) && x.notes.as_ref().is_some_and(|n| n.is_ok()));
                if kept {
                    sh.desk.flush();
                    return;
                }
                let why = Said::bad(e.name(), e.detail());
                whole = Some(crate::house::buyer_home::BHome { me, notes: Some(Err(why.clone())), activity: Some(Err(why.clone())), active: Some(Err(why.clone())), done: Some(Err(why.clone())), balance: Some(Err(why)), ..Default::default() });
                got = None;
            }
            if let Some(h) = whole {
                sh.desk.sh.read_ok("bhome");
                sh.desk.sh.home_asked = h.stopped;
                sh.desk.sh.buyer_home = Some(h);
            }
            if let Some(p) = got {
                match &p.got.toast {
                    Some(t) => sh.desk.sh.toast_read("bhome", crate::house::sync::said_toast(t)),
                    None => sh.desk.sh.read_ok("bhome"),
                }
                let anchor = p.got.anchor.clone();
                sh.desk.queue_remind(&anchor, &p.got.system);
                sh.desk.sh.home_asked = p.got.stopped;
                merge_in(&mut sh.desk.sh.buyer_home, p);
            }
            sh.desk.flush();
        }
        Landed::Cands(c) => {
            if stale {
                sh.land_stale(Kind::Cands, act, c.got.said.clone());
                return;
            }
            let mut c = *c;
            if crate::house::seller::Cands::behind(sh.desk.sh.seller.cands.as_ref(), c.from.as_ref(), &c.got) {
                sh.site_done(site, true);
                sh.desk.flush();
                return;
            }
            let read = c.read;
            let ok = c.got.said.as_ref().is_none_or(|s| s.tone == Tone::Ok);
            match origin_keeps(&mut c.got.said) {
                Some(s) => sh.desk.slot_toast("cands", read, toast_of(&s)),
                None => sh.desk.slot_ok("cands", read),
            }
            merge_in(&mut sh.desk.sh.seller.cands, c);
            sh.site_done(site, ok);
            if let Some(e) = failed {
                sh.desk.slot_toast("cands", read, crate::house::sync::fault_toast(&e));
            }
            sh.desk.flush();
        }
        Landed::Orders(o) => {
            if stale {
                sh.land_stale(Kind::Orders, act, o.got.said.clone());
                return;
            }
            let mut c = *o;
            let acted = Some(&c.got).filter(|_| act).and_then(|c| c.order.as_ref()).map(|o| o.instance.clone()).filter(|i| !i.is_empty());
            if let Some(i) = acted {
                sh.refresh_for(crate::house::refresh::Cause::Landed, Some(i));
            }
            let read = c.read;
            let ok = c.got.said.as_ref().is_none_or(|s| s.tone == Tone::Ok);
            match origin_keeps(&mut c.got.said) {
                Some(s) => sh.desk.slot_toast("orders", read, toast_of(&s)),
                None => sh.desk.slot_ok("orders", read),
            }
            merge_in(&mut sh.desk.sh.seller.orders, c);
            sh.site_done(site, ok);
            if let Some(e) = failed {
                sh.desk.slot_toast("orders", read, crate::house::sync::fault_toast(&e));
            }
            sh.desk.flush();
        }
        Landed::Goods(g) => {
            if stale {
                sh.desk.goods_after_leave = None;
                sh.land_stale(Kind::Goods, act, g.got.said.clone());
                return;
            }
            let mut g = *g;
            let read = g.read;
            let ok = failed.is_none();
            let kept = ok && g.got.said.as_ref().is_none_or(|s| s.tone != Tone::Bad);
            match origin_keeps(&mut g.got.said) {
                Some(s) => sh.desk.slot_toast("goods", read, toast_of(&s)),
                None => sh.desk.slot_ok("goods", read),
            }
            merge_in(&mut sh.desk.sh.seller.goods, g);
            if let Some(e) = failed {
                sh.desk.slot_toast("goods", read, crate::house::sync::fault_toast(&e));
            }
            if let Some(gd) = sh.desk.sh.seller.goods.as_mut() {
                gd.mark_dirty();
            }
            if site == Site::Sheet("goods-leave") {
                if let Some(to) = sh.desk.goods_after_leave.take() {
                    let saved = kept && sh.desk.sh.seller.goods.as_ref().and_then(|gd| gd.edit.as_ref()).is_some_and(|e| !e.dirty);
                    if saved {
                        if let Some(gd) = sh.desk.sh.seller.goods.as_mut() {
                            gd.edit = None;
                        }
                        crate::house::exec::follow(sh, to);
                    } else if let Some(e) = sh.desk.sh.seller.goods.as_mut().and_then(|gd| gd.edit.as_mut()) {
                        e.ask = None;
                    }
                }
            }
            sh.desk.flush();
        }
        Landed::PayLink(pl) => land_paylink(sh, *pl, stale, site, act, failed),
        Landed::Bo(b) => {
            if stale {
                sh.land_stale(Kind::Bo, act, b.got.said.clone());
                return;
            }
            let mut b = *b;
            let read = b.read;
            let acted = Some(&b.got).filter(|_| act).and_then(|b| b.open.as_ref()).map(|o| o.instance.clone());
            let drew = b.got.drew.take();
            if let Some(i) = acted.or(drew) {
                sh.refresh_for(crate::house::refresh::Cause::Landed, Some(i));
            }
            if b.got.rows.as_ref().is_some_and(|r| r.is_ok()) {
                sh.desk.slot_ok("bo", read);
            }
            let ok = b.got.said.as_ref().is_none_or(|s| s.tone != Tone::Bad);
            if site != Site::Page {
                if let Some(s) = origin_keeps(&mut b.got.said) {
                    sh.desk.slot_toast("bo", read, toast_of(&s));
                }
            }
            merge_in(&mut sh.desk.sh.buyer_orders, b);
            sh.site_done(site, ok);
            if let Some(e) = failed {
                sh.desk.slot_toast("bo", read, crate::house::sync::fault_toast(&e));
                let mut b = sh.desk.sh.buyer_orders.clone().unwrap_or_default();
                b.tell(Said::bad(e.name(), e.detail()));
                if b.rows.is_none() {
                    b.rows = Some(Err(Said::bad(e.name(), e.detail())));
                }
                b.me = sh.desk.addr.to_ascii_lowercase();
                sh.desk.sh.buyer_orders = Some(b);
            }
        }
        Landed::Wallet(w) => {
            if stale {
                sh.land_stale(Kind::Wallet, act, w.got.said.clone());
                return;
            }
            let mut p = *w;
            let read = p.read;
            let w = &mut p.got;
            if (act && w.said.as_ref().is_some_and(|s| s.tone == Tone::Ok)) || (site == Site::Sheet(crate::house::wallet::VAULT_SHEET) && w.vault_set.is_some()) {
                sh.refresh_for(crate::house::refresh::Cause::Landed, None);
            }
            let ok = w.said.as_ref().is_none_or(|s| s.tone == Tone::Ok);
            match origin_keeps(&mut w.said) {
                Some(s) => sh.desk.slot_toast("wallet", read, Toast { tone: s.tone, head: s.detail.clone(), next: crate::house::sync::next_of(&s), detail: Some(s.name.clone()) }),
                None => sh.desk.slot_ok("wallet", read),
            }
            merge_in(&mut sh.desk.sh.wallet, p);
            sh.site_done(site, ok);
            if let Some(e) = failed {
                sh.desk.slot_toast("wallet", read, crate::house::sync::fault_toast(&e));
                let mut w = sh.desk.sh.wallet.clone().unwrap_or_default();
                w.me = sh.desk.addr.to_ascii_lowercase();
                let why = Said::bad(e.name(), e.detail());
                w.balances = Some(Err(why.clone()));
                w.txs = Some(Err(why));
                sh.desk.sh.wallet = Some(w);
            }
            sh.desk.flush();
        }
        Landed::Export(e) => {
            if stale {
                sh.land_stale(Kind::Export, act, e.got.said.clone());
                return;
            }
            let mut p = *e;
            let read = p.read;
            let e = &mut p.got;
            let t = |k: crate::house::i18n::Key| crate::house::i18n::t(k);
            let ok = !matches!(e.done, Some(Err(_)));
            let done = e.done.as_ref().filter(|_| !read);
            match (done, origin_keeps(&mut e.said)) {
                (_, Some(s)) => sh.desk.slot_toast("export", read, Toast { tone: s.tone, head: s.detail.clone(), next: crate::house::sync::next_of(&s), detail: Some(s.name.clone()) }),
                (Some(Ok(o)), None) => sh.desk.slot_toast("export", read, Toast { tone: Tone::Ok, head: t(if o.tab == crate::house::export::Tab::Dossier { crate::house::i18n::Key::ExDossierSaid } else { crate::house::i18n::Key::ExDoneStatement }), next: String::new(), detail: Some(o.files.join("\n")) }),
                (Some(Err(x)), None) => sh.desk.slot_toast("export", read, Toast { tone: Tone::Bad, head: x.detail.clone(), next: t(crate::house::i18n::Key::ErrEVAULTBACKUPNOTLANDEDNext), detail: Some(x.name.clone()) }),
                (None, None) => sh.desk.slot_ok("export", read),
            }
            merge_in(&mut sh.desk.sh.export, p);
            sh.site_done(site, ok);
            if let Some(x) = failed {
                sh.desk.slot_toast("export", read, crate::house::sync::fault_toast(&x));
            }
            sh.desk.flush();
        }
        Landed::Settings(s) => {
            if stale {
                sh.land_stale(Kind::Settings, act, s.got.said.clone());
                return;
            }
            let p = *s;
            let read = p.read;
            let s = &p.got;
            let ok = s.said.as_ref().is_none_or(|x| x.tone == Tone::Ok);
            let took = act && crate::house::settings::took(s.said.as_ref());
            match &s.said {
                Some(x) => sh.desk.slot_toast("settings", read, toast_of(x)),
                None => sh.desk.slot_ok("settings", read),
            }
            let unlisted = sh.desk.sh.seller.settings.as_ref().is_none_or(|x| x.rotate.is_none());
            let resume = unlisted && matches!(&s.rotate, Some(Ok(v)) if v.opens_on_resume());
            if let (true, Some(crate::house::cards::SetCard::Rotate { keep, .. })) = (resume, sh.desk.set_card.as_mut()) {
                *keep = true;
            }
            merge_in(&mut sh.desk.sh.seller.settings, p);
            if ok && !matches!(sh.desk.set_card, Some(crate::house::cards::SetCard::Rotate { .. })) {
                sh.desk.set_card = None;
            }
            if let Some(e) = failed {
                sh.desk.sh.toast(crate::house::sync::fault_toast(&e));
            }
            if took {
                sh.moved();
            }
            let (was, was_seat) = (sh.desk.addr.clone(), sh.desk.addr_seat);
            let rotated = sh.desk.sh.seller.settings.as_ref().and_then(|x| Some((x.rotate.clone()?, x.section)));
            let card = sh.desk.set_card.clone();
            let moved = rotated.is_some() && crate::action::follow_register(sh) == crate::action::Followed::Moved;
            if moved {
                sh.seat_follow();
            }
            sh.refresh_addr();
            let mine = |r: &Result<crate::house::settings::RotView, crate::house::seller::Said>| r.as_ref().map_or(true, |v| v.new.is_empty() || eravon_vault::accounts::same_row(&sh.desk.data, &v.new, &sh.desk.addr));
            let rotated = rotated.filter(|(r, _)| mine(r));
            if moved && rotated.is_some() {
                sh.desk.set_card = card;
            }
            if let (Some((r, section)), Some(x)) = (rotated, sh.desk.sh.seller.settings.as_mut())
                && x.rotate.is_none()
            {
                x.rotate = Some(r);
                x.section = section;
            }
            if act && ok && sh.desk.addr.eq_ignore_ascii_case(&was) && sh.desk.addr_seat == was_seat {
                sh.refresh_for(crate::house::refresh::Cause::Switch, None);
            }
            sh.desk.flush();
        }
    }
    let _ = kind;
}

fn land_sync(sh: &mut Shell, stale: bool, got: Option<Box<crate::house::sync::Tick>>, failed: Option<eravon_chain::fault::Fault>) {
    if stale {
        sh.refresh_for(crate::house::refresh::Cause::Key, None);
        return;
    }
    let fresh_book = got.as_ref().is_some_and(|t| t.pass.as_ref().is_some_and(|p| p.book.is_some()));
    let cause = sh.desk.refresh_cause.take();
    sh.desk.reshow_open = !matches!(cause, Some((crate::house::refresh::Cause::Landed, _)));
    if let (None, Some(e), Some((cause, only))) = (&got, &failed, cause) {
        crate::house::refresh::failed(&mut sh.desk.sh, cause, only, e);
    }
    let ok = crate::house::sync::land(&mut sh.desk.sh, got.map(|t| *t), failed.as_ref());
    sh.desk.sync_done_at = sync_done(ok, sh.desk.pace_now());
    if fresh_book {
        sh.reshow();
    }
    if let Some((cause, only)) = sh.desk.refresh_again.take() {
        sh.refresh_for(cause, only);
    }
}

fn land_remind(sh: &mut Shell, sent: Vec<(Remind, Option<String>)>, failed: Option<eravon_chain::fault::Fault>) {
    let mut bad: Vec<String> = Vec::new();
    let mut owners: Vec<(String, Vec<String>)> = Vec::new();
    for (r, e) in sent {
        match e {
            Some(e) => bad.push(e),
            None => match owners.iter_mut().find(|(o, _)| *o == r.owner) {
                Some((_, keys)) => keys.push(r.key),
                None => owners.push((r.owner, vec![r.key])),
            },
        }
    }
    let now = sh.desk.now();
    for (owner, keys) in owners {
        if let Err(e) = crate::house::home::delivered(&sh.desk.data, &owner, now, &keys) {
            bad.push(e.detail);
        }
    }
    bad.extend(failed.map(|e| e.detail()));
    if !bad.is_empty() {
        sh.desk.sh.toast(Toast { tone: Tone::Bad, head: crate::house::i18n::t(crate::house::i18n::Key::NotifyNotSent), next: String::new(), detail: Some(bad.join(";")) });
    }
}

fn land_paylink(sh: &mut Shell, pass: Pass<crate::house::paylink::PayLink>, stale: bool, site: Site, act: bool, failed: Option<eravon_chain::fault::Fault>) {
    let pl = &pass.got;
    if stale {
        fn ok_of<T>(r: &Result<T, Said>) -> Said {
            match r {
                Ok(_) => Said::ok("ok"),
                Err(e) => e.clone(),
            }
        }
        let said = [pl.paid.as_ref().map(ok_of), pl.company.as_ref().map(ok_of), pl.co_open.as_ref().map(ok_of), pl.import.said.clone()]
            .into_iter()
            .flatten()
            .find(|s| s.tone != Tone::Ok || s.name == "ok");
        if said.as_ref().is_some_and(|s| s.tone == Tone::Ok) && sh.unlocked() {
            merge_in(&mut sh.desk.sh.paylink, pass);
        }
        sh.land_stale(Kind::PayLink, act, said);
        return;
    }
    let wrote = |p: &crate::house::paylink::PayLink| matches!(p.paid, Some(Ok(_))) || matches!(p.co_open, Some(Ok(_)));
    let was = |p: &crate::house::paylink::PayLink| (p.paid.clone(), p.co_open.clone());
    if act && wrote(pl) && sh.desk.sh.paylink.as_ref().is_none_or(|x| !crate::house::merge::same(&was(x), &was(pl))) {
        sh.refresh_for(crate::house::refresh::Cause::Landed, None);
    }
    let errs = |x: Option<&crate::house::paylink::PayLink>| -> [Option<Said>; 3] {
        match x {
            Some(p) => [p.paid.as_ref().and_then(|r| r.as_ref().err().cloned()), p.company.as_ref().and_then(|r| r.as_ref().err().cloned()), p.co_open.as_ref().and_then(|r| r.as_ref().err().cloned())],
            None => [None, None, None],
        }
    };
    let (old, new) = (errs(sh.desk.sh.paylink.as_ref()), errs(Some(pl)));
    for (o, n) in old.iter().zip(new.iter()) {
        if let Some(s) = n.as_ref().filter(|s| o.as_ref() != Some(*s)) {
            sh.desk.sh.toast(Toast { tone: s.tone, head: s.detail.clone(), next: crate::house::sync::next_of(s), detail: Some(s.name.clone()) });
        }
    }
    let fresh = pl.import.done.is_some() && sh.desk.sh.paylink.as_ref().is_some_and(|x| !crate::house::merge::same(&x.import.done, &pl.import.done));
    if fresh {
        sh.desk.sh.toast(Toast { tone: Tone::Ok, head: crate::house::i18n::t(crate::house::i18n::Key::PlImportedSaid), next: String::new(), detail: None });
    }
    let settle = matches!(&pl.opened, Some(Err(e)) if e.name == crate::house::paylink::NOT_LISTING) && sh.desk.seat() == Seat::Buyer;
    let link = pl.link.clone();
    merge_in(&mut sh.desk.sh.paylink, pass);
    if settle {
        if let Some(p) = sh.desk.sh.paylink.as_mut() {
            p.opened = None;
        }
        sh.go_page_now(Some(Page::Orders));
        sh.settle_from_link(link);
    }
    let ok = new.iter().all(Option::is_none);
    sh.site_done(site, ok);
    if fresh && sh.desk.seat() == Seat::Buyer && !sh.covered() {
        sh.go_page_now(Some(Page::Orders));
    }
    if let Some(e) = failed {
        sh.desk.sh.toast(crate::house::sync::fault_toast(&e));
    }
}
