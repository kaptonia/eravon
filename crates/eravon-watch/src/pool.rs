use eravon_roster::book::PanelBook;

use crate::{Why, Wire, why};

#[derive(Clone, Debug)]
pub struct Panel {
    pub members: Vec<String>,
    pub amanah_hashes: Vec<[u8; 32]>,
}

fn book_of(w: &Wire<'_>) -> eravon_cands::wire::Wire {
    eravon_cands::wire::Wire { t: w.t.clone(), table: w.table.clone(), registries: Vec::new(), now: 0 }
}

fn words(e: &eravon_roster::book::Engaged) -> Result<Vec<[u8; 32]>, Why> {
    let mut hashes = Vec::with_capacity(e.amanah_hashes.len());
    for x in &e.amanah_hashes {
        hashes.push(eravon_standard::order::b32_of("判法词", x).map_err(|f| why(&f.name(), f.detail()))?);
    }
    Ok(hashes)
}

pub fn panel(w: &Wire<'_>, guild: &str, o: &eravon_standard::order::Order) -> Result<Panel, Why> {
    let cw = book_of(w);
    let book = eravon_goods::pool::LiveBook { w: &cw };
    let key = eravon_roster::pool::standard_code_hash();
    let root = book.root_of(guild).map_err(|e| why("E_WATCH_POOL", format!("行会 {guild} 在换钥簿上的根读不成:{e}")))?;
    let (head, _) = eravon_chain::read::height(w.t, w.table).map_err(|e| why(e.name(), e.detail()))?;
    let now = eravon_scan::scan::block_stamp(w.t, w.table, head).map_err(|e| why(&e.name(), e.detail()))?;
    let rec = eravon_roster::pool::frozen(&book, &root, key, now, head.saturating_add(1))
        .map_err(|e| why("E_WATCH_POOL", format!("行会 {guild} 的名单簿读不成:{e}")))?
        .ok_or_else(|| why("E_WATCH_POOL", format!("行会 {guild} 的名单簿上没有已登记、未过期的记录")))?;
    let e = book
        .engaged(&root, key, rec.index)
        .map_err(|e| why("E_WATCH_POOL", format!("行会 {guild} 第 {} 条名单的本体读不成:{e}", rec.index)))?
        .ok_or_else(|| why("E_WATCH_POOL", format!("行会 {guild} 的名单簿上没有第 {} 条名单的本体", rec.index)))?;
    let word = eravon_standard::order::hex_of(&o.offer.amanah_hash);
    let parties = [o.offer.seller_key.clone(), o.buyer_key.clone()];
    eravon_roster::pool::panel_ok(&rec, &e, now, &word, &parties)
        .map_err(|r| why("E_WATCH_POOL", format!("行会 {guild} 第 {} 条名单不能冻结:{}", rec.index, r.name())))?;
    Ok(Panel { amanah_hashes: words(&e)?, members: e.members })
}

pub fn panel_at(w: &Wire<'_>, root: &str, index: u64, panel_hash: &str) -> Result<Panel, Why> {
    let cw = book_of(w);
    let book = eravon_goods::pool::LiveBook { w: &cw };
    let key = eravon_roster::pool::standard_code_hash();
    let e = book
        .engaged(root, key, index)
        .map_err(|e| why("E_WATCH_POOL", format!("根 {root} 第 {index} 条名单的本体读不成:{e}")))?
        .ok_or_else(|| why("E_WATCH_POOL", format!("名单簿上根 {root} 之下没有第 {index} 条名单的本体")))?;
    if !eravon_roster::pool::panel_matches(&e, panel_hash) {
        return Err(why("E_WATCH_POOL", format!("根 {root} 第 {index} 条名单与案冻结的 panelHash {panel_hash} 不符")));
    }
    Ok(Panel { amanah_hashes: words(&e)?, members: e.members })
}

pub fn draw_due(w: &Wire<'_>, instance: &str) -> Result<Option<eravon_standard::call::Call>, Why> {
    let a1w = |f: eravon_standard::fault::Fault| why(&f.name(), f.detail());
    let Some((members, amanah_hashes)) = draw_lists(w, instance)? else { return Ok(None) };
    Ok(Some(eravon_standard::call::Action::Redraw { members, amanah_hashes }.call(instance).map_err(a1w)?))
}

pub fn draw_lists(w: &Wire<'_>, instance: &str) -> Result<Option<(Vec<String>, Vec<[u8; 32]>)>, Why> {
    use eravon_standard::chain::status_col as col;
    let a1w = |f: eravon_standard::fault::Fault| why(&f.name(), f.detail());
    let st = eravon_standard::chain::status(w.t, w.table, instance).map_err(a1w)?.value;
    let at = |i: usize, what: &str| st.get(i).ok_or_else(|| why("E_WATCH_READ", format!("status 没有答 {what}")));
    if !at(col::PENDING_REDRAW, "pendingRedraw")?.boolean().map_err(a1w)? {
        return Ok(None);
    }
    if !redraws(at(col::SEATS, "seats")?.small().map_err(a1w)?) {
        return Ok(None);
    }
    let draw = at(col::DRAW_BLOCK, "drawBlock")?.small().map_err(a1w)?;
    let (head, _) = eravon_chain::read::height(w.t, w.table).map_err(|e| why(e.name(), e.detail()))?;
    if head <= draw {
        return Ok(None);
    }
    let index = at(col::PANEL_INDEX, "panelIndex")?.small().map_err(a1w)?;
    let generation = eravon_standard::chain::code_generation(w.t, w.table, instance, w.logic).map_err(a1w)?.value;
    let (members, amanah_hashes, logic) = match generation {
        eravon_literals::constants::Generation::Past(row) => {
            let hash = at(col::PANEL_HASH, "panelHash")?.word().map_err(a1w)?;
            let (m, h) = eravon_standard::chain::past_panel(w.t, w.table, instance, &row, index, &hash).map_err(a1w)?;
            (m, h, row.logic.to_string())
        }
        eravon_literals::constants::Generation::Current => {
            let root = eravon_standard::order::hex_of(&at(col::ROOT, "root")?.addr().map_err(a1w)?);
            let hash = eravon_standard::order::hex_of(&at(col::PANEL_HASH, "panelHash")?.word().map_err(a1w)?);
            let p = panel_at(w, &root, index, &hash)?;
            (p.members, p.amanah_hashes, w.logic.to_string())
        }
    };
    eravon_standard::chain::code_matches(w.t, w.table, instance, &logic).map_err(a1w)?;
    Ok(Some((members, amanah_hashes)))
}

fn redraws(_seats: u64) -> bool {
    true
}



#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Due {
    No,
    Seat(eravon_standard::chain::Outcome),
    Resigned,
    Draw,
    Past,
}

impl Due {
    pub fn moves(&self) -> bool {
        !matches!(self, Due::No)
    }

    pub fn ran_out(&self) -> bool {
        matches!(self, Due::Seat(_) | Due::Resigned | Due::Draw)
    }
}

pub const ADVANCE_NOT_DUE: &str = "E_DISPUTE_ADVANCE_NOT_DUE";
pub const ADVANCE_NOT_DUE_SAYS: &str = "期限未到,现在到期结清不会推进这一单";

pub fn advance_due(w: &Wire<'_>, instance: &str, case_id: &[u8; 32], seat: u64) -> Result<Due, Why> {
    use eravon_standard::chain::{seat_col, status_col as col};
    let a1w = |f: eravon_standard::fault::Fault| why(&f.name(), f.detail());
    let generation = eravon_standard::chain::code_generation(w.t, w.table, instance, w.logic).map_err(a1w)?.value;
    if generation.is_past() {
        return Ok(Due::Past);
    }
    let st = eravon_standard::chain::status(w.t, w.table, instance).map_err(a1w)?.value;
    let at = |i: usize, what: &str| st.get(i).ok_or_else(|| why("E_WATCH_READ", format!("status 没有答 {what}")));
    let phase = at(col::PHASE, "phase")?.small().map_err(a1w)?;
    if phase == PHASE_NONE {
        return Ok(Due::No);
    }
    let (head, _) = eravon_chain::clock::now(w.t, w.table).map_err(|e| why(e.name(), e.detail()))?;
    let seats = at(col::SEATS, "seats")?.small().map_err(a1w)?;
    if seat < seats {
        eravon_standard::chain::ahead_seat_and_advance(w.t, w.table, instance, case_id, seat);
        let s = eravon_standard::chain::seat_of(w.t, w.table, instance, case_id, seat).map_err(a1w)?.value;
        let small = |i: usize, what: &str| s.get(i).ok_or_else(|| why("E_WATCH_READ", format!("seatOf 没有答 {what}"))).and_then(|v| v.small().map_err(a1w));
        let (state, window_end) = (small(seat_col::STATE, "state")?, small(eravon_standard::sol::src_abi::I_GALEED_VENUE_T_SEAT_COL_WINDOW_END, "windowEnd")?);
        let o = eravon_standard::chain::advance_of(w.t, w.table, instance, case_id, seat).map_err(a1w)?.value;
        if o.state != state {
            return Ok(Due::Seat(o));
        }
        if resigned_lapsed(window_end, head.timestamp, state, phase, seat, seats) {
            return Ok(Due::Resigned);
        }
    }
    if at(col::PENDING_REDRAW, "pendingRedraw")?.boolean().map_err(a1w)? {
        let draw = at(col::DRAW_BLOCK, "drawBlock")?.small().map_err(a1w)?;
        let deadline = at(col::REDRAW_DEADLINE, "redrawDeadline")?.small().map_err(a1w)?;
        if draw_lapsed(head.block, head.timestamp, draw, deadline) {
            return Ok(Due::Draw);
        }
    }
    Ok(Due::No)
}

const PHASE_NONE: u64 = 0;
const PHASE_CLAIMED: u64 = 2;
const SEAT_RESIGNED: u64 = 3;
const DRAW_SEED_BLOCKS: u64 = 256;

fn resigned_lapsed(window_end: u64, now: u64, state: u64, phase: u64, seat: u64, seats: u64) -> bool {
    window_end != 0 && now > window_end && state == SEAT_RESIGNED && phase == PHASE_CLAIMED && seat.saturating_add(1) == seats
}

fn draw_lapsed(block: u64, now: u64, draw_block: u64, deadline: u64) -> bool {
    block > draw_block.saturating_add(1) && (now > deadline || block > draw_block.saturating_add(DRAW_SEED_BLOCKS))
}

#[cfg(test)]
mod due_tests {
    use super::*;

    #[test]
    fn t2_after_a_resignation_is_due_as_the_contract_has_it() {
        assert!(resigned_lapsed(100, 101, SEAT_RESIGNED, PHASE_CLAIMED, 0, 1));
        assert!(!resigned_lapsed(100, 100, SEAT_RESIGNED, PHASE_CLAIMED, 0, 1), "at windowEnd itself not yet");
        assert!(!resigned_lapsed(0, 101, SEAT_RESIGNED, PHASE_CLAIMED, 0, 1), "no window set");
        assert!(!resigned_lapsed(100, 101, 1, PHASE_CLAIMED, 0, 1), "a seated judge is advanceOf's (value 5)");
        assert!(!resigned_lapsed(100, 101, SEAT_RESIGNED, 3, 0, 1), "disputed");
        assert!(!resigned_lapsed(100, 101, SEAT_RESIGNED, PHASE_CLAIMED, 0, 2), "not the last seat");
        assert!(!resigned_lapsed(100, 101, SEAT_RESIGNED, PHASE_CLAIMED, u64::MAX, 0), "no seat folds back to none");
    }

    #[test]
    fn the_draw_clock_runs_out_as_the_contract_has_it() {
        assert!(!draw_lapsed(11, 999, 10, 500), "the block after the arming is excepted");
        assert!(draw_lapsed(12, 501, 10, 500), "deadline passed");
        assert!(!draw_lapsed(12, 500, 10, 500), "at the deadline itself not yet");
        assert!(draw_lapsed(267, 0, 10, 500), "seed lost");
        assert!(!draw_lapsed(266, 0, 10, 500), "seed still readable");
        assert!(!draw_lapsed(u64::MAX, 0, u64::MAX, 500), "no wrap past the last block");
    }
}
