use crate::trouble::Trouble;
use std::path::Path;

pub fn tmp_name() -> Result<String, Trouble> {
    let mut b = [0u8; 8];
    crate::trouble::fill_random(&mut b)?;
    Ok(eravon_glue::layout::tmp_file_name(&eravon::hexfmt::encode(&b)[2..]))
}

pub fn put_at(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), Trouble> {
    eravon_os::put_at(dir, name, &tmp_name()?, bytes).map_err(unput)
}

pub fn unput(u: eravon_os::Unput) -> Trouble {
    Trouble::io(&u.error, &u.at.display().to_string())
}
