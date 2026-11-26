use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::endpoints::Table;
use crate::fault::R;
use crate::read;
use crate::seam;
use crate::task::Outcome;
use crate::transport::Transport;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tick {
    Block(u64),
    Changed { block: u64, from: String, to: String },
    Logs { block: u64, count: u64 },
}

pub fn watch_balance<T: Transport>(
    t: &T,
    table: &Table,
    addr: &str,
    blocks: u64,
    interval: Duration,
    with_logs: bool,
    tx: &Sender<Outcome<Tick>>,
) {
    if let Err(f) = run(t, table, addr, blocks, interval, with_logs, tx) {
        let _ = tx.send(Outcome::Failed(f));
        return;
    }
    let _ = tx.send(Outcome::Done);
}

fn run<T: Transport>(
    t: &T,
    table: &Table,
    addr: &str,
    blocks: u64,
    interval: Duration,
    with_logs: bool,
    tx: &Sender<Outcome<Tick>>,
) -> R<()> {
    let (baseline, _) = read::balance(t, table, addr, seam::TAG_LATEST)?;
    let (mut seen, _) = block_number(t, table)?;
    let mut ticks = 0u64;
    while ticks < blocks {
        let (height, _) = crate::action::after_a_beat(interval, || block_number(t, table))?;
        if height <= seen {
            continue;
        }
        ticks += 1;
        if tx.send(Outcome::Item(Tick::Block(height))).is_err() {
            return Ok(());
        }
        if with_logs {
            let (rows, _) = read::logs(t, table, addr, seen + 1, seam::Upto::Block(height))?;
            if !rows.is_empty() {
                let _ = tx.send(Outcome::Item(Tick::Logs {
                    block: height,
                    count: rows.len() as u64,
                }));
            }
        }
        seen = height;
        let (nowv, _) = read::balance(t, table, addr, seam::TAG_LATEST)?;
        let against = against(t, table, addr, &baseline)?;
        if nowv != against {
            let _ = tx.send(Outcome::Item(Tick::Changed {
                block: height,
                from: against,
                to: nowv,
            }));
            return Ok(());
        }
    }
    Ok(())
}

pub fn tick_once<T: Transport>(
    t: &T,
    table: &Table,
    addr: &str,
    was: Option<&str>,
    since: Option<u64>,
    with_logs: bool,
    tx: &Sender<Outcome<Tick>>,
) {
    if let Err(f) = once(t, table, addr, was, since, with_logs, tx) {
        let _ = tx.send(Outcome::Failed(f));
        return;
    }
    let _ = tx.send(Outcome::Done);
}

fn once<T: Transport>(
    t: &T,
    table: &Table,
    addr: &str,
    was: Option<&str>,
    since: Option<u64>,
    with_logs: bool,
    tx: &Sender<Outcome<Tick>>,
) -> R<()> {
    let (height, _) = block_number(t, table)?;
    if tx.send(Outcome::Item(Tick::Block(height))).is_err() {
        return Ok(());
    }
    if with_logs {
        if let Some(from) = since {
            if height > from {
                let (rows, _) = read::logs(t, table, addr, from + 1, seam::Upto::Block(height))?;
                if !rows.is_empty() {
                    let _ = tx.send(Outcome::Item(Tick::Logs {
                        block: height,
                        count: rows.len() as u64,
                    }));
                }
            }
        }
    }
    let (nowv, _) = read::balance(t, table, addr, seam::TAG_LATEST)?;
    let Some(was) = was else { return Ok(()) };
    let against = against(t, table, addr, was)?;
    if nowv != against {
        let _ = tx.send(Outcome::Item(Tick::Changed { block: height, from: against, to: nowv }));
    }
    Ok(())
}

fn against<T: Transport>(_t: &T, _table: &Table, _addr: &str, baseline: &str) -> R<String> {
    Ok(baseline.to_string())
}


fn block_number<T: Transport>(t: &T, table: &Table) -> R<(u64, ())> {
    let r = read::read_agreed(t, table, seam::M_BLOCK_NUMBER, vec![])?;
    Ok((seam::read_quantity(&read::as_str(&r)?)?, ()))
}
