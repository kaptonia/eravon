use eravon_evidence as k6;
use eravon_galeed as k1;

use crate::fault::{Fault, R};
use crate::index::Kit;
use crate::seam;

pub use eravon_glue::read::UNPACK_HEAD;

pub type FileItem = k6::sheet::FileItem;
pub type Manifest = k6::sheet::Manifest;

fn prefix() -> &'static str {
    seam::kit_prefix()
}


pub fn items(kit: &Kit, media_type: &str) -> R<Vec<FileItem>> {
    if kit.sealed {
        return Err(Fault::Pack(format!("{} 是封给一把钥的加密记录包(索引行带 {}):只有收件人打得开,不收作证据", kit.path, seam::F_ONE_TIME)));
    }
    let t = tree(&kit.path)?;
    k6::listing::items_from_tree(t.root(), prefix(), media_type).map_err(|e| Fault::Pack(format!("{} 走不成清单项:{e:?}", kit.path)))
}

pub struct Tree {
    root: std::path::PathBuf,
    _unpacked: Option<eravon_glue::read::Unpacked>,
}

impl Tree {
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}

pub fn tree(path: &str) -> R<Tree> {
    let at = std::path::Path::new(path);
    if !eravon_glue::container::is_container_at(at) {
        return Ok(Tree { root: at.to_path_buf(), _unpacked: None });
    }
    let u = eravon_glue::read::unpack(at).map_err(|b| Fault::Pack(format!("{path} 是单档记录包而拆不开({}:{})", b.code(), b.subject())))?;
    Ok(Tree { root: u.dir.clone(), _unpacked: Some(u) })
}


pub fn manifest(
    subject_ref: k1::Value,
    party: &str,
    files: Vec<FileItem>,
    note_md: Option<String>,
) -> Result<(Manifest, Vec<u8>, [u8; 32]), k1::Token> {
    let m = k6::listing::from_items(subject_ref, party, files, note_md);
    let bytes = k6::sheet::build(&m)?;
    let digest = k6::listing::claim_parameter(&m)?;
    Ok((m, bytes, digest))
}
