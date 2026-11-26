use eravon_literals::json::Out;
use eravon_vault as h2;

use crate::answer::ok;
use crate::deskops::{self, No};
use crate::fault::{Fault, R};
use crate::home::{self, Ctx};
use crate::shared::{self, Flags};

pub fn run(verb: &str, c: &Ctx, f: &Flags) -> Option<R<Out>> {
    Some(match verb {
        "vault-rename" => rename(c, f),
        "seat-set" => seat_set(c, f),
        "backup-export" => backup_export(c, f),
        "backup-restore" => backup_restore(c, f),
        "settings-endpoints" => endpoints(c, f),
        "settings-network" => network(c, f),
        "settings-zikaron" => zikaron(c, f),
        "settings-zikaron-link" => zikaron_link(c, f),
        "settings-zone" => zone(c, f),
        "settings-cli-path" => cli_path(c, f),
        "settings-proxy" => proxy(c, f),
        "settings-token-add" => token_add(c, f),
        "settings-token-remove" => token_remove(c, f),
        "settings-token-decimals" => token_decimals(c, f),
        "settings-company" => company(c, f),
        "settings-time-check" => time_check(c, f),
        "settings-past-cases" => past_cases(c, f),
        "note-handle" => note_handle(c, f),
        "export" => export(c, f, None),
        "seal" => seal(c, f),
        "direct-check" => direct_check(c, f),
        "direct-give-up" => direct_give_up(c, f),
        _ => return None,
    })
}

pub const VERBS: &[&str] = &[
    "vault-rename",
    "seat-set",
    "backup-export",
    "backup-restore",
    "settings-endpoints",
    "settings-network",
    "settings-zikaron",
    "settings-zikaron-link",
    "settings-zone",
    "settings-cli-path",
    "settings-proxy",
    "settings-token-add",
    "settings-token-remove",
    "settings-token-decimals",
    "settings-company",
    "settings-time-check",
    "settings-past-cases",
    "note-handle",
    "export",
    "seal",
    "direct-check",
    "direct-give-up",
];

fn need(f: &Flags, k: &str) -> R<String> {
    f.opt(k).ok_or_else(|| Fault::Arg(format!("要 {k}")))
}

fn on_off(f: &Flags, k: &str) -> R<bool> {
    match need(f, k)?.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        x => Err(Fault::Arg(format!("{k} 只收 true 或 false,给的是 {x:?}"))),
    }
}

fn no(n: No) -> Fault {
    n.fault()
}

fn vault(e: h2::fault::Fault) -> Fault {
    Fault::Down { layer: "H2", token: e.which().map_or_else(|| e.class().as_str(), |k| k.as_str()).to_string(), why: e.tail().to_string() }
}

fn rename(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let label = need(f, "--label")?;
    h2::accounts::rename(&*c.session(None)?, &who, &label).map_err(home::down)?;
    Ok(ok(vec![("address", Out::S(who)), ("label", Out::S(label.trim().to_string()))]))
}

pub(crate) fn seat_set(c: &Ctx, f: &Flags) -> R<Out> {
    let who = match (f.one("--address"), h2::accounts::active(&c.data, c.seat).map_err(home::down)?) {
        (Some(a), _) => a.to_string(),
        (None, Some(a)) => a,
        (None, None) => {
            let v = h2::vault::read(&c.data).map_err(home::down)?;
            h2::accounts::primary_of(&v, c.seat).map(|i| i.address.clone()).ok_or_else(|| Fault::State(format!("{} 这一席还没有当前钥:先跑 vault-init", c.seat.word())))?
        }
    };
    let chain = h2::store::birth_chain(f.opt_int("--chain")?)?;
    let born = h2::onboard::seat(&c.data, c.seat, &who, chain, c.now).map_err(home::down)?;
    Ok(ok(vec![("address", Out::S(who)), ("endpointCount", Out::N(born.endpoints as u64)), ("seat", Out::S(c.seat.word().to_string()))]))
}

fn backup_export(c: &Ctx, f: &Flags) -> R<Out> {
    let _open = c.session(None)?;
    let (pw, again) = (h2::secret::Secret::of(&need(f, "--file-passcode")?), h2::secret::Secret::of(&need(f, "--again")?));
    h2::backup::password_ok(&pw, &again).map_err(vault)?;
    let dir = h2::home::landing(&need(f, "--out")?).map_err(vault)?;
    let made = std::fs::create_dir_all(&dir).map_err(|e| Fault::Io(format!("{}:{e}", dir.display()))).and_then(|()| h2::backup::export(&dir, pw.expose(), c.now).map_err(vault));
    let done = made.inspect_err(|_| {
        let _ = h2::machine::update(|m| m.backup_failed = Some(c.now));
    })?;
    Ok(ok(vec![("file", Out::S(done.path.display().to_string())), ("count", Out::N(done.summary.records as u64))]))
}

fn backup_restore(c: &Ctx, f: &Flags) -> R<Out> {
    let pin = f.opt("--passcode").ok_or_else(|| Fault::Arg("要 --passcode".into()))?;
    let pw = need(f, "--file-passcode")?;
    let file = need(f, "--file")?;
    let _s = c.write()?;
    h2::keybox::unlock(&pin).map_err(vault)?;
    let done = h2::backup::restore(std::path::Path::new(&file), &pw, h2::backup::From::Settings(&pin)).map_err(vault)?;
    Ok(ok(vec![("file", Out::S(file)), ("count", Out::N(done.summary.records as u64))]))
}

fn endpoints(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let _open = c.session(None)?;
    let n = deskops::home::endpoints(&c.data, &who, f.opt_int("--chain")?, &need(f, "--nodes")?).map_err(no)?;
    Ok(ok(vec![("address", Out::S(who)), ("endpointCount", Out::N(n as u64))]))
}

fn network(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let _open = c.session(None)?;
    deskops::network::choose_by(&need(f, "--chain")?, |chain| h2::store::choose_network(&c.data, &who, chain).map_err(No::store)).map_err(no)?;
    let chain = h2::store::network(&c.data, &who)?;
    let n = h2::store::rpc_endpoints(&c.data, &who)?.len();
    Ok(ok(vec![("address", Out::S(who)), ("chainId", Out::N(chain)), ("endpointCount", Out::N(n as u64))]))
}

fn zikaron(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let dir = f.opt("--zikaron-dir").unwrap_or_default();
    deskops::network::set_zikaron(&c.data, &c.writer, c.now, &who, &dir).map_err(no)?;
    Ok(ok(vec![("address", Out::S(who)), ("dir", Out::S(dir.trim().to_string()))]))
}

fn zikaron_link(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let on = on_off(f, "--on")?;
    deskops::network::set_zikaron_link(&c.data, &c.writer, c.now, &who, on).map_err(no)?;
    Ok(ok(vec![("address", Out::S(who)), ("on", Out::B(on))]))
}

fn zone(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let _open = c.session(None)?;
    let z = deskops::home::zone(&c.data, &who, &need(f, "--zone")?).map_err(no)?;
    Ok(ok(vec![("address", Out::S(who)), ("zone", Out::S(z.as_str().to_string()))]))
}

fn cli_path(_c: &Ctx, f: &Flags) -> R<Out> {
    let on = on_off(f, "--on")?;
    let st = deskops::home::cli_path(on).map_err(no)?;
    Ok(ok(vec![("on", Out::B(on)), ("state", Out::S(format!("{st:?}")))]))
}

fn proxy(_c: &Ctx, f: &Flags) -> R<Out> {
    let written = deskops::home::proxy(&need(f, "--way")?).map_err(no)?;
    Ok(ok(vec![("way", Out::S(written))]))
}

fn token_add(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let (token, label) = (need(f, "--token")?, f.opt("--label").unwrap_or_default());
    let w = shared::wire(f, c, Some(&who));
    let probe = |at: &str| match &w {
        Ok(w) => eravon_standard::chain::token_probe(&w.t, &w.table, at),
        Err(e) => eravon_standard::chain::TokenProbe::Unreachable(format!("{}:{}", e.name(), e.detail())),
    };
    let added = match deskops::tokens::add(&c.data, &c.writer, c.now, &token, &label, None, &probe).map_err(no)? {
        deskops::tokens::Added::Manual(at) => match f.opt("--decimals") {
            Some(d) => deskops::tokens::add(&c.data, &c.writer, c.now, &at, &label, Some(&d), &probe).map_err(no)?,
            None => return Err(No::new("token-decimals-manual", vec![at]).fault()),
        },
        x => x,
    };
    let deskops::tokens::Added::Row(row) = added else { return Err(No::new("token-decimals-manual", vec![token]).fault()) };
    Ok(ok(vec![("decimals", row.decimals.map_or(Out::Null, |d| Out::N(u64::from(d)))), ("decimalsSource", Out::S(row.source)), ("label", Out::S(row.label)), ("token", Out::S(row.address))]))
}

fn token_remove(c: &Ctx, f: &Flags) -> R<Out> {
    let row = deskops::tokens::remove(&c.data, &c.writer, c.now, &need(f, "--token")?).map_err(no)?;
    Ok(ok(vec![("label", Out::S(row.label)), ("token", Out::S(row.address))]))
}

fn token_decimals(c: &Ctx, f: &Flags) -> R<Out> {
    let token = need(f, "--token")?;
    let d = deskops::tokens::decimals(&c.data, &c.writer, c.now, &token, &need(f, "--decimals")?).map_err(no)?;
    Ok(ok(vec![("decimals", Out::N(u64::from(d))), ("token", Out::S(token.to_ascii_lowercase()))]))
}

fn company(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let on = on_off(f, "--on")?;
    let address = f.opt("--company").unwrap_or_default();
    let mut m = vec![("address", Out::S(who.clone())), ("on", Out::B(on))];
    if on {
        let open = deskops::company::gate(&address, || shared::wire(f, c, Some(&who)).map(|w| (w.t, w.table)).map_err(|e| No::new(e.name(), vec![e.detail()]))).map_err(no)?;
        m.push(("agreement", Out::S(open.agreement)));
        m.push(("calendarBound", Out::N(open.bound)));
        m.push(("company", Out::S(address.trim().to_ascii_lowercase())));
    }
    deskops::company::set(&c.data, &c.writer, c.now, &who, on, &address).map_err(no)?;
    Ok(ok(m))
}

fn time_check(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let on = on_off(f, "--on")?;
    deskops::company::time_check(&c.data, &c.writer, c.now, &who, on).map_err(no)?;
    Ok(ok(vec![("address", Out::S(who)), ("on", Out::B(on))]))
}

fn past_cases(c: &Ctx, f: &Flags) -> R<Out> {
    let on = on_off(f, "--on")?;
    deskops::network::set_past_cases(&c.data, &c.writer, c.now, on).map_err(no)?;
    Ok(ok(vec![("on", Out::B(on))]))
}

fn note_handle(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let key = need(f, "--reminder")?;
    eravon_home::notes::handle(&c.data, &c.writer, c.now, &who, &key).map_err(|(n, d)| No::new(&n, vec![d]).fault())?;
    Ok(ok(vec![("address", Out::S(who)), ("reminder", Out::S(key))]))
}

pub(crate) fn export(c: &Ctx, f: &Flags, kind: Option<deskops::export::Kind>) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let kind = match kind {
        Some(k) => k,
        None => deskops::export::Kind::of(&need(f, "--tab")?).ok_or_else(|| Fault::Arg("--tab 只有 dossier 与 statement".into()))?,
    };
    let (instance, dir) = (need(f, "--instance")?, need(f, "--out")?);
    let buyer = c.seat == eravon_store::vault::Seat::Buyer;
    let w = shared::wire(f, c, Some(&who))?;
    let (owners, keys) = deskops::export::owners_keys(&*c.session(None)?, &c.data, &who, buyer).map_err(no)?;
    let record = deskops::export::pick(&c.data, &owners, &keys, buyer, &w, &instance).map_err(no)?;
    let registries = f.many("--registry");
    let job = deskops::export::Job { kind, data: c.data.clone(), writer: c.writer.clone(), me: who.to_ascii_lowercase(), buyer, owners, record, dir, registries, now: c.now };
    let o = deskops::export::run(&job, &w, &|| true).map_err(no)?;
    let Out::O(mut m) = o.out() else { unreachable!("the result block is an object") };
    m.push(("instance", Out::S(instance.to_ascii_lowercase())));
    Ok(ok(m))
}

fn seal(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let (instance, dir) = (need(f, "--instance")?, need(f, "--out")?);
    let buyer = c.seat == eravon_store::vault::Seat::Buyer;
    let w = shared::wire(f, c, Some(&who))?;
    let dw = eravon_dispute::Wire { t: &w.t, table: &w.table, chain_id: w.chain_id, logic: &w.logic, registries: &w.registries };
    let p6 = |(token, why): (String, String)| Fault::Down { layer: "P6", token, why };
    let s6 = |(token, why): (String, String)| Fault::Down { layer: "S6", token, why };
    let (case_id, _, _, block) = eravon_dispute::settle::opened(&dw, &instance).map_err(p6)?;
    let view = eravon_dispute::view::case(&dw, &instance, &case_id, block).map_err(p6)?;
    let (hash, given) = shared::evidence_arg(f)?;
    let manifest = match given {
        Some(b) => b,
        None => {
            let (owners, _) = deskops::export::owners_keys(&*c.session(None)?, &c.data, &who, buyer).map_err(no)?;
            deskops::seal::kept(&c.data, &owners, &instance, &hash).map_err(no)?
        }
    };
    let cw = eravon_cands::wire::Wire::on(&shared::route(f, c, Some(&who)), &w.registries, c.now).map_err(|a| Fault::Down { layer: "S3", token: a.name().to_string(), why: a.detail() })?;
    let seated = deskops::seal::read(&cw, &view.judge, "").map_err(no)?;
    if let (Some(u), false) = (&seated.unread, matches!(seated.pick, deskops::seal::Pick::One(_))) {
        return Err(s6((eravon_dispute::seal::E_SEAL_RECIPIENT.to_string(), format!("判官 {} 的账本读不出({u}):挑不出收件钥,不导出,也不发明文", seated.address))));
    }
    let kit = f.opt("--kit");
    let (home, machine) = shared::evidence_places(c, f, &who, kit.is_some())?;
    let drop = f.opt("--manifest-dir").map(|p| shared::whole_place(&p));
    let origins = eravon_dispute::seal::origins(&home, machine.as_deref(), kit.as_deref(), drop.as_deref()).map_err(s6)?;
    let sealed = eravon_dispute::seal::export(&origins, &manifest, std::slice::from_ref(&seated.pick), &shared::whole_place(&dir)).map_err(s6)?;
    Ok(ok(vec![
        ("dir", Out::S(sealed.dir)),
        ("files", Out::A(sealed.files.into_iter().map(Out::S).collect())),
        ("instance", Out::S(instance.to_ascii_lowercase())),
        ("judge", Out::O(vec![("address", Out::S(seated.address.clone())), ("name", Out::S(seated.name.clone().unwrap_or_default())), ("pick", deskops::seal::pick_out(&seated.pick))])),
        ("manifest", Out::S(eravon_standard::order::hex_of(&hash))),
    ]))
}

fn direct_check(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let key = c.key(&who, f.one("--passcode"))?;
    let d = deskops::direct::find(&who, &key, &need(f, "--tx")?).map_err(no)?;
    let w = shared::wire(f, c, Some(&who))?;
    Ok(ok(match deskops::direct::check(&w, &d, &key).map_err(no)? {
        deskops::direct::Checked::Done { tx, block, status } => vec![("block", Out::N(block)), ("state", Out::S(if status == 1 { "included" } else { "reverted" }.into())), ("tx", Out::S(tx))],
        deskops::direct::Checked::Void(tx) => vec![("state", Out::S("void".into())), ("tx", Out::S(tx))],
        deskops::direct::Checked::Pending(tx) => vec![("state", Out::S("submitted".into())), ("tx", Out::S(tx))],
    }))
}

fn direct_give_up(c: &Ctx, f: &Flags) -> R<Out> {
    let who = c.who(f.one("--address"))?;
    let key = c.key(&who, f.one("--passcode"))?;
    let d = deskops::direct::find(&who, &key, &need(f, "--tx")?).map_err(no)?;
    let w = shared::wire(f, c, Some(&who))?;
    deskops::direct::give_up(&w, &d, &key).map_err(no)?;
    Ok(ok(vec![("state", Out::S("void".into())), ("tx", Out::S(d.last().to_string()))]))
}
