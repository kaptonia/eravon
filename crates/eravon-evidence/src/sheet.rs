use crate::mark;

pub const SPEC: &str = galeed_evidence::SPEC;

pub type Manifest = galeed_evidence::Manifest;
pub type FileItem = galeed_evidence::FileItem;
pub type Unlisted = galeed_evidence::Unlisted;
pub const JUNK_SEGMENTS: [&str; 1] = galeed_evidence::JUNK_SEGMENTS;
pub type Zikaron = galeed_evidence::Zikaron;
pub type Fault = galeed_evidence::Fault;

pub fn check(b: &[u8]) -> Result<Manifest, Fault> {
    mark::mark(mark::FEATURE);
    galeed_evidence::check(b)
}


pub fn manifest(b: &[u8]) -> Result<[u8; 32], Fault> {
    mark::mark(mark::FEATURE);
    galeed_evidence::digest(b)
}


pub fn build(m: &Manifest) -> Result<Vec<u8>, eravon_galeed::Token> {
    mark::mark(mark::FEATURE);
    galeed_evidence::build(m)
}

pub fn file_view(b: &[u8]) -> (u64, [u8; 32]) {
    mark::mark(mark::FEATURE);
    galeed_evidence::file_view(b)
}

pub fn listed(name: &str, bytes: &[u8], media_type: &str) -> Result<FileItem, Unlisted> {
    mark::mark(mark::FEATURE);
    FileItem::listed(name, bytes, media_type)
}
