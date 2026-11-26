use eravon_literals::json::Out;

use crate::endpoints::Table;
use crate::fault::{Fault, R};
use crate::read::{self, Reading};
use crate::seam;
use crate::transport::Transport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Now {
    pub block: u64,
    pub timestamp: u64,
}

pub fn now<T: Transport>(t: &T, table: &Table) -> R<(Now, Reading)> {
    let r = read::read_agreed(
        t,
        table,
        seam::M_GET_BLOCK,
        vec![Out::S(seam::TAG_LATEST.to_string()), Out::B(false)],
    )?;
    let block = field(&r, "number")?;
    let timestamp = field(&r, "timestamp")?;
    Ok((Now { block, timestamp }, r))
}

fn field(r: &Reading, key: &str) -> R<u64> {
    let s = r
        .value
        .get(key)
        .and_then(|x| x.as_str())
        .ok_or_else(|| Fault::Endpoint(format!("块上没有 {key} 这一栏,或它不是文本")))?;
    seam::read_quantity(s)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub action: bool,
    pub stamp: bool,
}

pub fn window(end: u64, now: u64) -> Window {
    Window { action: now <= end, stamp: now > end }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_second_belongs_to_the_action() {
        assert_eq!(window(100, 99), Window { action: true, stamp: false });
        assert_eq!(window(100, 100), Window { action: true, stamp: false });
        assert_eq!(window(100, 101), Window { action: false, stamp: true });
    }

    #[test]
    fn the_two_answers_are_never_both_true_and_never_both_false() {
        for now in 0..5u64 {
            let w = window(2, now);
            assert_ne!(w.action, w.stamp);
        }
    }
}
