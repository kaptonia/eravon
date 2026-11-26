use super::*;

pub(super) fn s(x: &str) -> Out {
    Out::S(x.to_string())
}

pub(super) fn row_out(r: &Row) -> Out {
    eravon_cli::deskops::orderpage::page_row(r, &crate::house::coin::rows())
}

pub use eravon_cli::deskops::orderpage::{acts, shown_acts};

impl Orders {
    pub(super) fn s2_rows(&self, m: &mut Vec<(&'static str, Out)>) {
        m.push(("me", s(&self.me)));
        match &self.table {
            Some(Ok(_)) => m.push(("list", Out::A(self.shown().into_iter().map(row_out).collect()))),
            Some(Err(e)) => m.push(("list", e.out())),
            None => {}
        }
        m.push(("foreign", Out::N(self.foreign)));
        m.push(("listTab", s(if self.done_tab { "done" } else { "active" })));
        m.push((
            "search",
            Out::O(vec![("from", s(&self.search.from)), ("on", Out::B(self.search.on)), ("q", s(&self.search.q)), ("to", s(&self.search.to))]),
        ));
        m.push(("pasteOpen", Out::B(self.paste_open)));
        if let Some(sel) = &self.sel {
            m.push(("sel", s(sel)));
            m.push(("tab", s(self.tab.word())));
        }
        if let Some(p) = &self.saved {
            m.push(("saved", Out::A(p.iter().map(|x| s(x)).collect())));
        }
        if let Some(j) = &self.judge {
            m.push(("judge", crate::house::judge_card::card_out(j)));
            m.push(("judgeOpen", Out::B(self.judge_open)));
        }
        if let Some(h) = crate::house::history::out(&self.history) {
            m.push(("history", h));
        }
        if let Some(i) = self.inner {
            m.push(("inner", s(i.word())));
            m.push(("innerTab", Out::N(u64::from(i.tab()))));
        }
        if let Some(b) = &self.bench {
            m.push(("bench", crate::house::case::bench_out(b)));
            if let (Some(crate::house::orders::Inner::Lapse), Ok(x)) = (self.inner, b) {
                if let Ok(q) = &x.quote {
                    m.push(("lapse", crate::house::case::lapse_out(&q.clocks, matches!(x.track, crate::house::case::Track::Guild(_)))));
                }
            }
        }
        if let Some(p) = &self.papers {
            m.push(("papers", crate::house::case::papers_out(p)));
        }
        m.push(("sealable", Out::B(self.sealable.is_some())));
        if let Some(x) = &self.sealed {
            m.push(("sealed", crate::house::judge_card::sealed_out(x)));
        }
        if let Some(c) = &self.complaint {
            m.push(("complaint", eravon_cli::deskops::complaint::complaint_out(c)));
        }
        let (Some(d), Some(r)) = (&self.detail, self.row()) else { return };
        m.push(("detail", eravon_cli::deskops::orderpage::detail_out(d, r, self.order.as_ref(), &crate::house::coin::rows(), self.terms_open)));
    }
}
