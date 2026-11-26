use super::*;

pub(super) fn vault_init(c: &Ctx, f: &Flags) -> R<Out> {
    if f.one("--key").is_some() {
        return Err(home::down(h2::add::outside_key()));
    }
    if h2::vault::exists(&c.data) {
        return crate::deskverbs::seat_set(c, f);
    }
    let label = f.opt("--label").unwrap_or_else(|| "买家钥".to_string());
    let chain = h2::store::birth_chain(f.opt_int("--chain")?)?;
    let s = c.write()?;
    let passcode = f.opt("--passcode").ok_or_else(|| Fault::Arg("要 --passcode".into()))?;
    let phrase = f.opt("--phrase").ok_or_else(|| {
        Fault::Arg("要 --phrase(十二个词);生成并摆出来那一路在桌面应用的首启向导里".into())
    })?;
    let mut born = h2::vault::create(
        &passcode,
        Seat::Buyer,
        &label,
        Some(&phrase),
        c.now,
    )
    .map_err(home::down)?;
    let g = eravon_store::settings::Global { buyer_ready: true, last_writer: c.writer.clone(), ..Default::default() };
    let aside = born.write(&c.data).map_err(home::down)?;
    c.open_now(&passcode)?;
    h2::accounts::switch(&c.data, Seat::Buyer, &born.address).map_err(home::down)?;
    s.init(&g)?;
    s.init_identity(&born.address, c.now)?;
    h2::store::change_home(&c.data, &born.address, |h| h.chain_id = Some(chain))?;
    let (params, ms) = born.sealed_at;
    Ok(ok(vec![
        ("address", Out::S(born.address.clone())),
        ("identities", Out::N(h2::vault::count(&c.data).map_err(home::down)? as u64)),
        ("kdfLogN", Out::N(u64::from(params.log_n))),
        ("kdfMs", Out::N(ms)),
        ("seat", Out::S(seam::SEAT_BUYER.to_string())),
    ].into_iter().chain(aside.map(|e| ("trouble", Out::A(vec![Out::S(format!("{}:{}", e.name(), e.detail()))]))))
    .collect()))
}

pub(super) fn vault_import(c: &Ctx, f: &Flags) -> R<Out> {
    vault_init(c, f)
}

pub(super) fn seat_unlock(c: &Ctx, f: &Flags) -> R<Out> {
    let session = c.session(f.one("--passcode"))?;
    let v = session.vault().map_err(home::down)?;
    let opened: Vec<String> = v
        .identities
        .iter()
        .filter(|i| session.key_for(&i.address).is_ok())
        .map(|i| i.address.clone())
        .collect();
    let door = if session.is_open() { h2::seam::GATE_OPEN } else { h2::seam::GATE_LOCKED };
    Ok(ok(vec![
        ("door", Out::S(door.into())),
        ("open", Out::B(session.is_open())),
        ("opened", strs(&opened)),
        ("wrongCount", Out::N(u64::from(h2::vault::wrong_count(&c.data).map_err(home::down)?))),
    ]))
}

pub(super) fn seat_lock(c: &Ctx, _f: &Flags) -> R<Out> {
    let mut session = c.session_mut()?;
    session.lock();
    let (door, wrong) = h2::vault::gate(&c.data).map_err(home::down)?;
    Ok(ok(vec![
        ("door", Out::S(door.to_string())),
        ("open", Out::B(session.is_open())),
        ("wrongCount", Out::N(u64::from(wrong))),
    ]))
}

pub(super) fn seat_derive(c: &Ctx, f: &Flags) -> R<Out> {
    let role = f.opt("--role").ok_or_else(|| Fault::Arg("要 --role".into()))?;
    let session = c.session(f.one("--passcode"))?;
    let who = c.who(f.one("--address"))?;
    let (path, address) = h2::add::derive_role(&session, &who, &role, 0).map_err(home::down)?;
    Ok(ok(vec![("address", Out::S(address)), ("path", Out::S(path)), ("role", Out::S(role))]))
}

pub(super) fn seat_passcode(c: &Ctx, f: &Flags) -> R<Out> {
    let old = f.opt("--passcode").ok_or_else(|| Fault::Arg("要 --passcode".into()))?;
    let new = f.opt("--new-passcode").ok_or_else(|| Fault::Arg("要 --new-passcode".into()))?;
    let again = f.opt("--again").ok_or_else(|| Fault::Arg("要 --again".into()))?;
    let _s = c.write()?;
    h2::vault::passcode(&c.data, &old, &new, &again).map_err(home::down)?;
    Ok(ok(vec![("changed", Out::B(true)), ("wrongCount", Out::N(0))]))
}

pub(super) fn seat_recover(c: &Ctx, f: &Flags) -> R<Out> {
    let phrase = f.opt("--phrase").ok_or_else(|| Fault::Arg("要 --phrase(十二个词)".into()))?;
    let new = f.opt("--new-passcode").ok_or_else(|| Fault::Arg("要 --new-passcode".into()))?;
    let again = f.opt("--again").ok_or_else(|| Fault::Arg("要 --again".into()))?;
    let _s = c.write()?;
    h2::vault::recover(&c.data, &phrase, &new, &again).map_err(home::down)?;
    Ok(ok(vec![("recovered", Out::B(true)), ("wrongCount", Out::N(0))]))
}

pub(super) fn seat_list(c: &Ctx, _f: &Flags) -> R<Out> {
    let n = h2::vault::count(&c.data).map_err(home::down)?;
    let v = match h2::vault::read(&c.data) {
        Ok(v) => v,
        Err(h2::vfault::Fault::Locked) => return Ok(ok(vec![("count", Out::N(n as u64)), ("locked", Out::B(true))])),
        Err(e) => return Err(home::down(e)),
    };
    Ok(ok(vec![
        ("count", Out::N(v.identities.len() as u64)),
        (
            "identities",
            Out::A(
                v.identities
                    .iter()
                    .map(|i| {
                        Out::O(vec![
                            ("address", Out::S(i.address.clone())),
                            ("kind", Out::S(i.kind.word().to_string())),
                            ("label", Out::S(i.label.clone())),
                            ("primary", Out::B(i.primary)),
                            ("seat", Out::S(i.seat.word().to_string())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("locked", Out::B(false)),
    ]))
}
