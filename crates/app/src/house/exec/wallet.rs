use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::desk::Site;
use crate::house::shell::{Seat, Toast};
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    Some(match a.step {
        Step::WlReload => {
            if sh.desk.sh.wallet.is_none() {
                return Some(Err("先打开钱包页".to_string()));
            }
            let table = super::core::refresh(sh);
            sh.desk.tr.push(Step::WlReload, eravon_wallet::mark::FEATURE, table.clone());
            Ok(format!("wl-reload · {table}"))
        }
        Step::WlSend => send(sh, a),
        Step::WlFormerMove => former_move(sh, a),
        Step::WlAll => all(sh, a),
        Step::WlVaultSetup => vault_setup(sh, a),
        Step::WlVaultClose => match sh.desk.sh.wallet.as_mut() {
            Some(wl) => {
                wl.vault_set = None;
                Ok("wl-vault-close".into())
            }
            None => Err("先打开钱包页".to_string()),
        },
        Step::WlFund => fund(sh, a),
        Step::WlFundOwn => fund_own(sh, a),
        Step::WlSheet | Step::WlTx | Step::WlCopy => local(sh, a),
        _ => return None,
    })
}

fn local(sh: &mut Shell, a: &Act) -> Said {
    let wl = sh.desk.sh.wallet.as_mut().ok_or_else(|| "先打开钱包页".to_string())?;
    let say = match a.step {
        Step::WlSheet => {
            let open = a.opt("open") != Some("false");
            wl.sheet = open;
            if open && wl.said.as_ref().is_some_and(|s| s.tone == crate::house::seller::Tone::Bad) {
                wl.said = None;
            }
            if !open {
                sh.desk.wl_form = (0, String::new(), String::new());
                sh.desk.wl_again.clear();
            }
            if open { "open" } else { "false" }.to_string()
        }
        Step::WlTx => {
            let h = a.arg("hash")?;
            if wl.tx(&h).is_none() {
                return Err(format!("交易记录里没有 {h}"));
            }
            wl.open_tx = Some(h.clone());
            h
        }
        _ => {
            let item = a.arg("item")?;
            wl.copied = Some(match item.as_str() {
                "address" => wl.me.clone(),
                "hash" => wl.open_tx.clone().ok_or_else(|| "先点开一笔".to_string())?,
                x => return Err(format!("认不出这一项:{x}")),
            });
            item
        }
    };
    sh.desk.tr.push(a.step, eravon_wallet::mark::FEATURE, say.clone());
    Ok(say)
}

fn all(sh: &mut Shell, a: &Act) -> Said {
    if sh.desk.sh.wallet.is_none() {
        return Err("先打开钱包页".to_string());
    }
    let token = a.opt("token").filter(|x| !x.is_empty()).map(str::to_string);
    let to = a.opt("to").unwrap_or_default().trim().to_string();
    sh.desk.wl_all_fill = true;
    let go = sh.spawn_wallet_at(Site::Sheet("wallet-all"), crate::house::flow::Ask::Asked, move |_s, data, me, mut w, _t| {
        w.read_all(data, me, token.as_deref(), &to);
        w
    });
    if !go {
        sh.desk.wl_all_fill = false;
    }
    sh.started(go)?;
    sh.desk.tr.push(Step::WlAll, eravon_wallet::mark::FEATURE, String::new());
    Ok("wl-all".into())
}

fn vault_setup(sh: &mut Shell, a: &Act) -> Said {
    if sh.desk.seat() != Seat::Seller {
        return Err("金库是卖家席的".to_string());
    }
    if sh.desk.sh.wallet.is_none() {
        return Err("先打开钱包页".to_string());
    }
    let (token, amount) = (a.arg("token")?, a.arg("amount")?.trim().to_string());
    let say = format!("{token} · {amount}");
    let mut dir = String::new();
    let mut why = None;
    crate::house::orders::set_export_dir(&mut dir, &mut why, &sh.desk.sh.seller.field.home, &sh.desk.data);
    let now = sh.desk.now();
    let own = crate::house::wallet::own_vault(&sh.desk.data, &sh.desk.addr);
    if own {
        if let Some(w) = sh.write_why(crate::house::shell::Write::Chain) {
            let e = crate::house::seller::Said::bad("E_APP_WRITE", w);
            let line = format!("{} · {}", e.name, e.detail);
            sh.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: e.detail.clone(), next: String::new(), detail: Some(e.name.clone()) });
            return Ok(line);
        }
        sh.spawn_wallet_at(Site::Sheet(crate::house::wallet::VAULT_SEND), true, move |s, data, me, mut w, t| {
            let t = *t;
            w.vault_setup(s, data, me, &dir, now, &token, &amount, &move || t.live());
            w
        });
        sh.desk.tr.push(Step::WlVaultSetup, eravon_goods::mark::FEATURE, say.clone());
        return Ok(say);
    }
    let go = sh.spawn_wallet_at(Site::Sheet(crate::house::wallet::VAULT_SHEET), crate::house::flow::Ask::Asked, move |s, data, me, mut w, _t| {
        w.vault_setup(s, data, me, &dir, now, &token, &amount, &|| true);
        w
    });
    sh.started(go)?;
    sh.desk.tr.push(Step::WlVaultSetup, eravon_goods::mark::FEATURE, say.clone());
    Ok(say)
}

fn fund(sh: &mut Shell, a: &Act) -> Said {
    if sh.desk.seat() != Seat::Seller {
        return Err("金库是卖家席的".to_string());
    }
    let v = a.arg("v")?;
    if sh.desk.sh.seller.pay.is_none() {
        sh.open_pay();
    }
    let (data, addr, now) = (sh.desk.data.clone(), sh.desk.addr.clone(), sh.desk.now());
    let pay = sh.desk.sh.seller.pay.as_mut().ok_or_else(|| "读不到付款方式设置".to_string())?;
    pay.fund = v.trim().to_string();
    pay.save_opening(&data, &addr, now, &crate::house::i18n::t(crate::house::i18n::Key::VaultSavedSaid), true);
    let said = pay.said.clone();
    let bad = said.as_ref().is_some_and(|s| s.tone == crate::house::seller::Tone::Bad);
    if let Some(s) = &said {
        sh.desk.sh.toast(Toast { tone: s.tone, head: s.detail.clone(), next: String::new(), detail: bad.then(|| s.name.clone()) });
    }
    if !bad {
        sh.load_wallet();
    }
    sh.desk.tr.push(Step::WlFund, eravon_payways::mark::FEATURE, v.clone());
    Ok(said.map(|s| s.detail).unwrap_or_default())
}

fn fund_own(sh: &mut Shell, a: &Act) -> Said {
    if sh.desk.seat() != Seat::Seller {
        return Err("金库是卖家席的".to_string());
    }
    let on = match a.arg("on")?.as_str() {
        "true" => true,
        "false" => false,
        x => return Err(format!("on 只收 true 或 false,给的是 {x:?}")),
    };
    if sh.desk.sh.seller.pay.is_none() {
        sh.open_pay();
    }
    let (data, addr, now) = (sh.desk.data.clone(), sh.desk.addr.clone(), sh.desk.now());
    let pay = sh.desk.sh.seller.pay.as_mut().ok_or_else(|| "读不到付款方式设置".to_string())?;
    pay.save_own(&data, &addr, now, on);
    let said = pay.said.clone();
    let kept = pay.ways.as_ref().map(|w| w.kept.clone()).unwrap_or_default();
    let bad = said.as_ref().is_some_and(|s| s.tone == crate::house::seller::Tone::Bad);
    use crate::house::i18n::{t, Key};
    let toast = match (&said, bad, on) {
        (Some(s), true, _) => Toast { tone: s.tone, head: s.detail.clone(), next: String::new(), detail: Some(s.name.clone()) },
        (_, _, true) => Toast { tone: crate::house::seller::Tone::Ok, head: t(Key::WlOwnOnSaid), next: String::new(), detail: None },
        (_, _, false) if !kept.is_empty() => Toast { tone: crate::house::seller::Tone::Ok, head: t(Key::WlOwnOffSaid), next: String::new(), detail: None },
        _ => Toast { tone: crate::house::seller::Tone::Note, head: t(Key::WlOwnOffBare), next: t(Key::WlOwnOffBareNext), detail: None },
    };
    let head = toast.head.clone();
    sh.desk.sh.toast(toast);
    if !bad {
        sh.load_wallet();
    }
    sh.desk.tr.push(Step::WlFundOwn, eravon_payways::mark::FEATURE, on.to_string());
    Ok(head)
}

fn former_move(sh: &mut Shell, a: &Act) -> Said {
    if sh.desk.sh.wallet.is_none() {
        return Err("先打开钱包页".to_string());
    }
    let from = a.arg("a")?.trim().to_ascii_lowercase();
    if let Some(w) = sh.write_why(crate::house::shell::Write::Chain) {
        let e = crate::house::seller::Said::bad("E_APP_WRITE", w);
        let line = format!("{} · {}", e.name, e.detail);
        sh.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: e.detail.clone(), next: String::new(), detail: Some(e.name.clone()) });
        return Ok(line);
    }
    let said = from.clone();
    sh.spawn_wallet_at(Site::Page, true, move |s, data, me, mut w, t| {
        let t = *t;
        w.move_former(s, data, me, &from, &move || t.live());
        w
    });
    sh.desk.tr.push(Step::WlFormerMove, eravon_wallet::mark::FEATURE, said.clone());
    Ok(said)
}

fn send(sh: &mut Shell, a: &Act) -> Said {
    if sh.desk.sh.wallet.is_none() {
        return Err("先打开钱包页".to_string());
    }
    let token = a.opt("token").filter(|x| !x.is_empty()).map(str::to_string);
    let amount = match a.arg("amount")?.as_str() {
        "all" => eravon_wallet::send::Amount::All,
        x => eravon_wallet::send::Amount::Of(x.to_string()),
    };
    let to = a.arg("to")?.trim().to_string();
    let seat = sh.desk.seat();
    let again = (seat == Seat::Seller).then(|| a.opt("again").unwrap_or_default().trim().to_string());
    let me = crate::house::shell::current_key(&sh.desk.data, seat).unwrap_or_default();
    let why = sh.write_why(crate::house::shell::Write::Chain).map(|w| crate::house::seller::Said::bad("E_APP_WRITE", w));
    let asked = sh.desk.sh.wallet.as_ref().map_or(Ok(()), |w| crate::house::asked(w.send_plan(&me, token.as_deref(), &amount, &to, again.as_deref()))).and_then(|()| why.map_or(Ok(()), Err));
    if let Err(e) = asked {
        let line = format!("{} · {}", e.name, e.detail);
        sh.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: e.detail.clone(), next: String::new(), detail: Some(e.name.clone()) });
        if let Some(w) = sh.desk.sh.wallet.as_mut() {
            w.said = Some(e);
        }
        return Ok(line);
    }
    sh.desk.retry_on(crate::task::Kind::Wallet, a.clone());
    let say = if matches!(amount, eravon_wallet::send::Amount::All) { "all".to_string() } else { a.opt("amount").unwrap_or_default().to_string() };
    sh.spawn_wallet_at(Site::Sheet("wallet-send"), true, move |s, data, me, mut w, t| {
        let t = *t;
        w.transfer(s, data, me, token.as_deref(), &amount, &to, again.as_deref(), &move || t.live());
        w
    });
    sh.desk.tr.push(Step::WlSend, eravon_wallet::mark::FEATURE, say.clone());
    Ok(say)
}
