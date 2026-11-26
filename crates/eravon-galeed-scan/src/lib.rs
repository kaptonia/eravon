#![forbid(unsafe_code)]

pub mod basis;
pub mod txform;
pub mod verdict;

pub fn contained(calldata: &[u8], hash: &[u8; 32]) -> bool {
    galeed::kit::contained(calldata, hash)
}

pub fn carried(data: &[u8], hash: &[u8; 32]) -> Option<Vec<u8>> {
    galeed::kit::carried_anchor(data, hash)
}
