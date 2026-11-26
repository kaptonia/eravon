use eravon_store::vault::{IdentityKind, Seat};

use crate::vfault::{Fault, R};
use crate::session::Session;
use crate::{derive, keyfile, seam, vault};

pub fn add(
    s: &Session,
    bytes: &[u8],
    file_passcode: &str,
    seat: Seat,
    label: &str,
    now: u64,
) -> R<(String, IdentityKind)> {
    add_as(s, bytes, file_passcode, seat, label, true, now)
}

pub fn add_as(
    s: &Session,
    bytes: &[u8],
    file_passcode: &str,
    seat: Seat,
    label: &str,
    named: bool,
    now: u64,
) -> R<(String, IdentityKind)> {
    let _ = (s, bytes, file_passcode, seat, label, named, now);
    Err(crate::add::outside_key())
}

#[allow(clippy::too_many_arguments)]
pub fn rebuild(
    data: &str,
    bytes: &[u8],
    file_passcode: &str,
    new_passcode: &str,
    again: &str,
    seat: Seat,
    label: &str,
    _now: u64,
) -> R<(String, Option<usize>)> {
    crate::rules::twice(new_passcode, again).map_err(Fault::Rule)?;
    eravon_store::atomic::mkdirs(data)?;
    crate::records::bind(data)?;
    if crate::keybox::state()? == crate::keybox::State::Absent {
        let _ = (seat, label, bytes, file_passcode);
        return Err(crate::add::outside_key());
    }
    let got = keyfile::accept(bytes, file_passcode)?;
    crate::key::recover_with(&got.key, new_passcode).map_err(|e| match Fault::from(e) {
        Fault::Mnemonic(_) => Fault::NoIdentity(format!("这份密钥文件是 {} 的钥,不是这本密钥库主身份的:恢复只认主身份那一份", got.address)),
        e => e,
    })?;
    crate::local::after_open();
    keep_others(&got.address)?;
    crate::vault::lock_now();
    Ok((got.address, Some(0)))
}

fn keep_others(_kept: &str) -> R<()> {
    Ok(())
}


pub fn export(
    open: &Session,
    passcode: &str,
    address: &str,
    file_passcode: &str,
    again: &str,
) -> R<(String, Vec<u8>)> {
    crate::rules::file_passcode(file_passcode, again).map_err(Fault::Rule)?;
    keyfile::need_password(file_passcode)?;
    let data = open.data().to_string();
    vault::unlock(&data, passcode)?;
    secret_out(&data, address, file_passcode)
}


fn secret_out(data: &str, address: &str, file_passcode: &str) -> R<(String, Vec<u8>)> {
    let v = vault::read(data)?;
    crate::accounts::find(&v, address)?;
    let key = vault::open_key(data, address)?;
    let bytes = keyfile::write(&key, file_passcode)?;
    Ok((seam::keyfile_name(address), bytes))
}

pub fn export_to(open: &Session, passcode: &str, address: &str, file_passcode: &str, again: &str, dir: &str) -> R<String> {
    let (name, bytes) = export(open, passcode, address, file_passcode, again)?;
    let dir = dir.trim_end_matches('/');
    eravon_store::atomic::mkdirs(dir).map_err(Fault::from)?;
    let path = format!("{dir}/{name}");
    eravon_store::atomic::write_atomic(&path, &bytes).map_err(Fault::from)?;
    keyfile::landed(&path, address)?;
    let (row, _) = crate::records::seat_of(address)?;
    crate::register::change(|reg| crate::identity::mark(reg, &row.id, false, true, Some(&path)))?;
    Ok(path)
}

pub fn phrase(open: &Session, passcode: &str, address: &str) -> R<String> {
    let data = open.data().to_string();
    vault::unlock(&data, passcode)?;
    vault::open_phrase(&data, address)
}


pub fn file_name(address: &str) -> String {
    seam::keyfile_name(address)
}

pub fn address_of(key: &[u8; 32]) -> R<String> {
    derive::address(key)
}
