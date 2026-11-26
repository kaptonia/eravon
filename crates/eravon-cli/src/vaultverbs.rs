pub mod rotate;
mod rotateverb;

use eravon_literals::json::Out;
use eravon_store::vault::Seat;
use eravon_vault as h2;

use crate::fault::{Fault, R};
use crate::home::{self, Ctx};
use crate::answer::ok;
use crate::shared::Flags;

pub fn run(verb: &str, c: &Ctx, f: &Flags) -> Option<R<Out>> {
    Some(match verb {
        "vault-new-key" => new_key(c, f),
        "vault-import-words" => import_words(c, f),
        "vault-switch" => switch(c, f),
        "vault-rotate" => rotateverb::rotate(c, f),
        "vault-remove" => remove(c, f),
        "vault-reset" => reset(c, f),
        "vault-backup" => backup(c, f),
        "vault-restore" => restore(c, f),
        "vault-rebuild" => rebuild(c, f),
        "vault-phrase" => phrase(c, f),
        "vault-accounts" => accounts(c, f),
        "payout-set" => payout_set(c, f),
        "payout-show" => payout_show(c, f),
        "home-stand" => stand(c, f),
        _ => return None,
    })
}

pub const VERBS: &[&str] = &[
    "vault-accounts",
    "vault-backup",
    "vault-import-words",
    "vault-new-key",
    "vault-phrase",
    "vault-rebuild",
    "vault-remove",
    "vault-reset",
    "vault-restore",
    "vault-switch",
    "vault-rotate",
    "payout-set",
    "payout-show",
    "home-stand",
];

fn need(f: &Flags, k: &str) -> R<String> {
    f.opt(k).ok_or_else(|| Fault::Arg(format!("要 {k}")))
}

fn accounts(c: &Ctx, _f: &Flags) -> R<Out> {
    let v = c.vault()?;
    let active = h2::accounts::active(&c.data, c.seat).map_err(home::down)?;
    Ok(ok(vec![
        ("accounts", Out::A(h2::accounts::rows(&v, c.seat, active.as_deref()))),
        ("current", Out::S(active.unwrap_or_default())),
        ("seat", Out::S(c.seat.word().to_string())),
    ]))
}

fn new_key(c: &Ctx, f: &Flags) -> R<Out> {
    let named = f.opt("--label").is_some();
    let label = f.opt("--label").unwrap_or_else(|| "新钥".to_string());
    let s = c.write()?;
    let session = c.session(f.one("--passcode"))?;
    let (address, phrase) =
        h2::accounts::new_words_as(&session, c.seat, &label, named, c.now).map_err(home::down)?;
    let mut after = home::After::default();
    let mut out = after.born(&s, &c.data, &address, c.now);
    out.extend([
        ("address", Out::S(address)),
        ("label", Out::S(label)),
        ("phrase", Out::S(phrase)),
        ("seat", Out::S(c.seat.word().to_string())),
    ]);
    out.extend(after.rows());
    Ok(ok(out))
}

fn import_words(c: &Ctx, f: &Flags) -> R<Out> {
    let named = f.opt("--label").is_some();
    let label = f.opt("--label").unwrap_or_else(|| "导入的钥".to_string());
    let phrase = need(f, "--phrase")?;
    let s = c.write()?;
    let session = c.session(f.one("--passcode"))?;
    let address = h2::accounts::import_words_as(&session, c.seat, &label, named, &phrase, c.now)
        .map_err(home::down)?;
    let mut after = home::After::default();
    let born = after.born(&s, &c.data, &address, c.now);
    let mut out = vec![
        ("address", Out::S(address)),
        ("kind", Out::S("words".into())),
        ("label", Out::S(label)),
    ];
    out.extend(born);
    out.extend(after.rows());
    Ok(ok(out))
}

fn switch(c: &Ctx, f: &Flags) -> R<Out> {
    let to = need(f, "--address")?;
    let _s = c.write()?;
    let now = h2::accounts::switch(&c.data, c.seat, &to).map_err(home::down)?;
    Ok(ok(vec![
        ("current", Out::S(now.clone())),
        ("home", Out::S(h2::records::seat_of(&now).ok().and_then(|(r, s)| r.home(s)).map(|p| p.display().to_string()).unwrap_or_default())),
        ("seat", Out::S(c.seat.word().to_string())),
    ]))
}

fn remove(c: &Ctx, f: &Flags) -> R<Out> {
    let who = need(f, "--address")?;
    let _s = c.write()?;
    let session = c.session(Some(&need(f, "--passcode")?))?;
    let next = h2::accounts::remove(&session, &who).map_err(home::down)?;
    Ok(ok(vec![
        ("current", Out::S(next.unwrap_or_default())),
        ("removed", Out::S(who)),
        ("homeKept", Out::B(true)),
    ]))
}

fn reset(c: &Ctx, f: &Flags) -> R<Out> {
    let named = f.opt("--label").is_some();
    let label = f.opt("--label").unwrap_or_else(|| "新主钥".to_string());
    let s = c.write()?;
    let passcode = need(f, "--passcode")?;
    let mut session = c.session_mut()?;
    let r = crate::deskops::reset::run(&mut session, &s, &c.data, c.seat, &label, named, f.one("--phrase"), &passcode, c.now).map_err(home::down)?;
    drop(session);
    let mut after = home::After::default();
    for t in r.trouble {
        after.said(t);
    }
    let mut out = after.born_of(r.born);
    let identities = after.note(c.vault()).map_or(0, |v| v.identities.len() as u64);
    out.extend([
        ("address", Out::S(r.address)),
        ("identities", Out::N(identities)),
        ("phrase", Out::S(r.phrase.unwrap_or_default())),
    ]);
    out.extend(after.rows());
    Ok(ok(out))
}

fn backup(c: &Ctx, f: &Flags) -> R<Out> {
    let out_dir = need(f, "--out")?;
    let passcode = need(f, "--passcode")?;
    let file_pass = need(f, "--file-passcode")?;
    let again = need(f, "--again")?;
    let who = c.who(f.one("--address"))?;
    let session = c.session(None)?;
    let path = h2::restore::export_to(&session, &passcode, &who, &file_pass, &again, &out_dir).map_err(home::down)?;
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    Ok(ok(vec![
        ("address", Out::S(who)),
        ("bytes", Out::N(bytes)),
        ("file", Out::S(path)),
        ("landed", Out::B(true)),
    ]))
}

fn restore(c: &Ctx, f: &Flags) -> R<Out> {
    let path = need(f, "--file")?;
    let file_pass = need(f, "--file-passcode")?;
    let named = f.opt("--label").is_some();
    let label = f.opt("--label").unwrap_or_else(|| "还原的钥".to_string());
    let bytes = home::read_bounded(&path)?;
    let s = c.write()?;
    let session = c.session(Some(&need(f, "--passcode")?))?;
    let (address, kind) = h2::restore::add_as(&session, &bytes, &file_pass, c.seat, &label, named, c.now)
        .map_err(home::down)?;
    let mut after = home::After::default();
    let born = after.born(&s, &c.data, &address, c.now);
    let mut out = vec![
        ("address", Out::S(address)),
        ("kind", Out::S(kind.word().to_string())),
        ("label", Out::S(label)),
    ];
    out.extend(born);
    out.extend(after.rows());
    Ok(ok(out))
}

fn rebuild(c: &Ctx, f: &Flags) -> R<Out> {
    let path = need(f, "--file")?;
    let file_pass = need(f, "--file-passcode")?;
    let next = need(f, "--new-passcode")?;
    let again = need(f, "--again")?;
    let label = f.opt("--label").unwrap_or_else(|| "还原的主钥".to_string());
    let bytes = home::read_bounded(&path)?;
    let s = c.write()?;
    let (address, dropped) = h2::restore::rebuild(
        &c.data,
        &bytes,
        &file_pass,
        &next,
        &again,
        c.seat,
        &label,
        c.now,
    )
    .map_err(home::down)?;
    let mut after = home::After::default();
    after.note(c.open_now(&next));
    let g = after.note(eravon_vault::store::global_to_change(&c.data).map_err(Fault::from));
    if let Some(mut g) = g {
        match c.seat {
            Seat::Seller => g.seller_ready = true,
            Seat::Buyer => g.buyer_ready = true,
        }
        g.last_writer = c.writer.clone();
        after.note(eravon_vault::store::write_global(&c.data, &g).map_err(Fault::from));
    }
    let mut out = after.born(&s, &c.data, &address, c.now);
    out.extend([
        ("address", Out::S(address)),
        ("otherKeysCounted", Out::B(dropped.is_some())),
        ("otherKeysLost", Out::N(dropped.unwrap_or(0) as u64)),
        ("primary", Out::B(true)),
    ]);
    out.extend(after.rows());
    Ok(ok(out))
}

fn phrase(c: &Ctx, f: &Flags) -> R<Out> {
    let passcode = need(f, "--passcode")?;
    let who = c.who(f.one("--address"))?;
    let session = c.session(None)?;
    let words = h2::restore::phrase(&session, &passcode, &who).map_err(home::down)?;
    Ok(ok(vec![
        ("address", Out::S(who)),
        ("phrase", Out::S(words)),
        ("words", Out::N(h2::seam::WORDS as u64)),
    ]))
}

fn payout_set(c: &Ctx, f: &Flags) -> R<Out> {
    let addr = need(f, "--address")?;
    let again = need(f, "--again")?;
    let who = c.who(f.one("--who"))?;
    let s = c.write()?;
    let got = h2::payout::set(&c.data, &who, &addr, &again).map_err(home::down)?;
    drop(s);
    let n = eravon_goods::goods::note_payout(&c.data, &c.writer, &who, c.now)
        .map_err(|e| Fault::Down { layer: "S1", token: e.name().to_string(), why: e.detail() })?;
    Ok(ok(vec![
        ("key", Out::S(who)),
        ("payout", Out::S(got)),
        ("resignsPending", Out::B(n > 0)),
    ]))
}

fn payout_show(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--who"))?;
    let got = h2::payout::get(&c.data, &who).map_err(home::down)?;
    Ok(ok(vec![("key", Out::S(who)), ("payout", Out::S(got))]))
}

fn stand(c: &Ctx, _f: &Flags) -> R<Out> {
    let st = eravon_store::mirror::stand(&c.data)?;
    let mut rows = st.rows();
    rows.push(("data", Out::S(c.data.clone())));
    rows.sort_by(|a, b| a.0.cmp(b.0));
    Ok(ok(rows))
}
