use eravon_watch::order::Order;
use eravon_watch::send::Post;

use crate::entries::{self, EntryView};
use crate::seat::{self, Face};
use crate::sweep::{self, Swept};
use crate::{Why, Wire, why};

#[derive(Clone, Debug)]
pub struct Tick {
    pub face: Face,
    pub redraw: Option<Result<String, Why>>,
    pub entries: Result<Vec<EntryView>, Why>,
    pub sweep: Option<Result<Swept, Why>>,
    pub proposal: Option<eravon_standard::proposal::Plan>,
}

fn locked(what: &str) -> Why {
    why("E_DISPUTE_LOCKED", format!("已锁定,未发送 {what};解锁后刷新"))
}

#[allow(clippy::too_many_arguments)]
pub fn tick(
    data: &str,
    writer: &str,
    now: u64,
    me: &str,
    w: &Wire<'_>,
    o: &Order,
    mut post: Option<&mut dyn Post>,
    company: bool,
) -> Result<Tick, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let face = seat::face(w, o)?;
    let past = eravon_watch::poll::past_held(data, me, o);
    let redraw = (!past && seat::due(&face)).then(|| match post.as_deref_mut() {
        None => Err(locked("redraw")),
        Some(p) => seat::redraw(w, o, p),
    });
    let entries = entries::read(data, writer, now, me, w, o, 0);
    let mut proposal = None;
    let sweep = (!past && matches!(face, Face::Closed(_))).then(|| {
        let fresh = eravon_watch::order::load(data, me, &o.id)?;
        if company {
            return match sweep::company(data, writer, now, me, w, &fresh)? {
                Ok(p) => {
                    proposal = Some(p);
                    Ok(Swept::Nothing(String::new()))
                }
                Err(s) => Ok(s),
            };
        }
        match post.as_deref_mut() {
            None => Err(locked("sweep")),
            Some(p) => sweep::judge(data, writer, now, me, w, &fresh, p),
        }
    });
    Ok(Tick {
        face,
        redraw,
        entries,
        sweep,
        proposal,
    })
}
