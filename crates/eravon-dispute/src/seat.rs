use eravon_standard as a1;
use eravon_watch::order::Order;
use eravon_watch::send::Post;

use a1::chain::status_col as col;

use crate::seat_col;
use crate::{Why, Wire, a1w, case_of, small, status, why};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Face {
    Open(u64),
    Ruling(u64),
    PendingRedraw(u64),
    Resigned(String),
    Closed(u64),
}

impl Face {
    pub fn say(&self) -> String {
        match self {
            Face::Open(_) | Face::Ruling(_) => String::new(),
            Face::PendingRedraw(_) => "待抽签".into(),
            Face::Resigned(r) => format!("已退款 · {r}"),
            Face::Closed(_) => String::new(),
        }
    }

    pub fn refundable(&self) -> bool {
        matches!(self, Face::Open(_) | Face::PendingRedraw(_))
    }

    pub fn disputed(&self) -> bool {
        matches!(self, Face::Open(3) | Face::Ruling(_))
    }
}

pub const RESIGN_HINT: &str = "更换判官或行会,重签商品链接";

fn reason(form: u64) -> String {
    let r = match form {
        0 => "判官拒单",
        1 => "判官在争议前辞任",
        2 => "判官在争议中辞任",
        3 => "没抽签或没有可用名单",
        _ => "判官辞任",
    };
    format!("{r};{RESIGN_HINT}")
}


fn resigned_form(w: &Wire<'_>, instance: &str, opened: u64) -> Result<Option<u64>, Why> {
    let (rows, _) =
        eravon_chain::read::logs(w.t, w.table, instance, eravon_chain::read::instance_from(opened), eravon_chain::seam::Upto::Latest)
            .map_err(|e| why(e.name(), e.detail()))?;
    let mut form = None;
    for row in &rows {
        let l = eravon_scan::log::Log::read(row).map_err(|e| why(&e.name(), e.detail()))?;
        let Ok(d) = a1::event::decode(&l.topics, &l.data) else {
            continue;
        };
        if d.name == "Resigned" {
            form = Some(d.at(1).map_err(a1w)?.small().map_err(a1w)?);
        }
    }
    Ok(form)
}

pub fn face(w: &Wire<'_>, o: &Order) -> Result<Face, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let st = status(w, &o.instance)?;
    let phase = small(&st, col::PHASE)?;
    if phase == 8 {
        let form = resigned_form(w, &o.instance, o.opened)?;
        return Ok(Face::Resigned(reason(form.unwrap_or(u64::MAX))));
    }
    if !crate::open_phase(phase) {
        return Ok(Face::Closed(phase));
    }
    let pending = st
        .get(col::PENDING_REDRAW)
        .ok_or_else(|| why("E_DISPUTE_READ", "status 没有答 pendingRedraw"))?
        .boolean()
        .map_err(a1w)?;
    if pending {
        return Ok(Face::PendingRedraw(small(&st, col::REDRAW_DEADLINE)?));
    }
    let seats = small(&st, col::SEATS)?;
    if phase == a1::precheck::PHASE_DISPUTED && seats > 0 {
        let seat = a1::chain::seat_of(w.t, w.table, &o.instance, &case_of(o)?, seats - 1).map_err(a1w)?.value;
        let (state, end) = (small(&seat, seat_col::STATE)?, small(&seat, seat_col::WINDOW_END)?);
        let now = a1::precheck::chain_now(w.t, w.table).map_err(a1w)?;
        if !crate::refund::refundable(phase, Some(state), end, now) {
            return Ok(Face::Ruling(end));
        }
    }
    Ok(Face::Open(phase))
}

pub fn draw_ready(w: &Wire<'_>, instance: &str) -> Result<(bool, u64, u64), Why> {
    let st = status(w, instance)?;
    let armed = st.get(col::PENDING_REDRAW).ok_or_else(|| why("E_DISPUTE_READ", "status 没有答 pendingRedraw"))?.boolean().map_err(a1w)?;
    let draw = small(&st, col::DRAW_BLOCK)?;
    let (head, _) = eravon_chain::read::height(w.t, w.table).map_err(|e| why(e.name(), e.detail()))?;
    Ok((armed && head > draw, draw, head))
}

pub fn redraw(w: &Wire<'_>, o: &Order, post: &mut dyn Post) -> Result<String, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let call = eravon_watch::pool::draw_due(w, &o.instance)?.ok_or_else(|| why("E_DISPUTE_DRAW_NOT_DUE", "没有待抽的签,或抽签那一块还没过"))?;
    post.post(&call)
}

pub fn due(f: &Face) -> bool {
    matches!(f, Face::PendingRedraw(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_forms_each_say_why() {
        let rs: Vec<String> = (0..4).map(reason).collect();
        for (i, r) in rs.iter().enumerate() {
            assert!(r.contains(RESIGN_HINT), "{i}");
            assert!(rs.iter().filter(|x| *x == r).count() == 1, "{i}");
        }
    }
}
