use super::*;
use eravon_ui::{card, states, table};

impl Win {
    pub(super) fn queue_page(&mut self, ui: &mut egui::Ui) {
        let Some(v) = self.shell.desk.sh.queue.clone() else {
            states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO);
            return;
        };
        let direct = v.open_direct.as_ref().and_then(|tx| v.direct.as_ref().and_then(|d| d.as_ref().ok()).and_then(|d| d.iter().find(|x| x.txs.contains(tx)).cloned()));
        match direct {
            Some(d) => self.queue_direct(ui, &d, v.said.as_ref()),
            None => self.queue_list(ui, &v),
        }
    }

    fn queue_list(&mut self, ui: &mut egui::Ui, v: &crate::house::queue::View) {
        use eravon_ui::mark::Mark as M;
        let lang = crate::lang::lang();
        let t = |k| crate::house::i18n::get(lang, k).0;
        enum Pick {
            Direct(String),
            Order(String),
        }
        let direct = v.direct.as_ref().and_then(|d| d.as_ref().ok()).cloned().unwrap_or_default();
        let orders = v.orders.as_ref().and_then(|o| o.as_ref().ok()).cloned().unwrap_or_default();
        let mut query = self.shell.desk.queue_query.clone();
        let w = ui.available_width();
        input::search(ui, &mut query, &t(crate::house::i18n::Key::QueueSearch), w);
        ui.add_space(eravon_ui::metrics::CARD_GAP);
        for (i, (name, detail)) in [v.direct.as_ref().and_then(|d| d.as_ref().err()), v.orders.as_ref().and_then(|o| o.as_ref().err())].into_iter().flatten().enumerate() {
            states::err_box(ui, &format!("queue-src-{i}"), &crate::house::refusal::line(lang, name, detail), "", &t(crate::house::i18n::Key::U3RawError), &format!("{name} · {detail}"));
        }
        let n = query.trim().to_lowercase();
        let hit = |parts: &[&str]| n.is_empty() || parts.iter().any(|p| p.to_lowercase().contains(&n));
        let mut picks: Vec<Pick> = Vec::new();
        let mut kept: Vec<(bool, usize)> = Vec::new();
        for (i, d) in direct.iter().enumerate() {
            if hit(&[&d.to, &d.from, &d.txs.join(" ")]) {
                kept.push((true, i));
                picks.push(Pick::Direct(d.last().to_string()));
            }
        }
        for (i, o) in orders.iter().enumerate() {
            if hit(&[&o.instance, &o.what, &o.tx]) {
                kept.push((false, i));
                picks.push(Pick::Order(o.instance.clone()));
            }
        }
        let row = |cells: Vec<table::Cell>| table::Row { cells, click: true, gone: false, on: false };
        let make = |i: usize| match kept[i] {
            (true, j) => {
                let d = &direct[j];
                row(vec![table::Cell::Tag(t(crate::house::i18n::Key::QueueSrcDirect)), table::Cell::Text(d.to.clone()), table::Cell::Text(d.last().to_string()), table::Cell::Mono(format!("#{}", d.at.nonce)), table::Cell::Mark(M::Busy), table::Cell::Chev])
            }
            (false, j) => {
                let o = &orders[j];
                row(vec![table::Cell::Tag(t(crate::house::i18n::Key::QueueSrcOrder)), table::Cell::Tag(o.what.clone()), table::Cell::Text(o.tx.clone()), table::Cell::Mono(crate::house::orders::order_label(&o.instance)), table::Cell::Mark(M::Busy), table::Cell::Chev])
            }
        };
        let cols = [table::TYPE, table::TYPE, table::col("", table::Col::Fr(eravon_ui::house::COL_REST)), table::col("", table::Col::Px(eravon_ui::house::COL_QUEUED_AT)), table::MARK, table::CHEV];
        let all = direct.len() + orders.len();
        let empty = t(if all == 0 { crate::house::i18n::Key::QueueEmpty } else { crate::house::i18n::Key::QueueNoMatch });
        let generation = super::listed_gen(self.shell.desk.rows_gen, &n);
        if let Some(i) = table::table_with(ui, "queue", table::Spec::new(&cols, generation).empty(&empty), kept.len(), make).clicked {
            match picks.get(i) {
                Some(Pick::Direct(tx)) => {
                    if let Some(x) = self.shell.desk.sh.queue.as_mut() {
                        x.open_direct = Some(tx.clone());
                        x.said = None;
                    }
                }
                Some(Pick::Order(instance)) => self.step(crate::house::action::Action::HomeOrder, &[("i", instance)]),
                None => {}
            }
        }
        self.shell.desk.queue_query = query;
    }

    fn queue_back(&mut self) {
        if let Some(x) = self.shell.desk.sh.queue.as_mut() {
            x.open_direct = None;
            x.said = None;
        }
    }

    fn queue_direct(&mut self, ui: &mut egui::Ui, d: &crate::house::queue::Direct, said: Option<&crate::house::seller::Said>) {
        let t = |k| crate::house::i18n::get(crate::lang::lang(), k).0;
        Self::hero_beside(ui, &t(crate::house::i18n::Key::QueueSrcDirect), &eravon_orders::table::short(&d.to), &[], |ui| {
            eravon_ui::mark::mark(ui, eravon_ui::mark::Mark::Busy);
        });
        let free = !self.shell.tasks.in_flight(crate::task::Kind::Wallet);
        let (mut check, mut give_up, mut back) = (false, false, false);
        let tx = d.last().to_string();
        card::section(ui, &t(crate::house::i18n::Key::GoodsBasics), "", |ui| {
            let copied = card::form(ui, |ui, f| {
                let mut c = f.info(ui, &t(crate::house::i18n::Key::QueueDirectTo), card::Show::Short(&d.to));
                if !d.from.is_empty() {
                    c = f.info(ui, &t(crate::house::i18n::Key::QueueDirectFrom), card::Show::Short(&d.from)).or(c);
                }
                f.info(ui, "nonce", card::Show::Mono(&d.at.nonce.to_string()));
                c = f.info(ui, &t(crate::house::i18n::Key::SendTx), card::Show::Short(&tx)).or(c);
                if d.txs.len() > 1 {
                    let tl = t(crate::house::i18n::Key::SendTx);
                    let (detail, widest) = detail_rows_of(ui, &[(&tl, kv::Val::Mono(d.txs.join("\n")))]);
                    c = f.details(ui, "queue-direct-txs", &t(crate::house::i18n::Key::SetEvidence), &detail, widest).or(c);
                }
                c
            });
            tell_copied(ui.ctx(), copied);
        });
        card::form(ui, |ui, f| {
            let sub = crate::house::i18n::filln(crate::house::i18n::Key::OrdersNoteSentWaiting, &[&eravon_orders::table::short(&tx)]);
            let _ = f.line(ui, Some(card::Lead::Mark(Mark::Busy)), &t(crate::house::i18n::Key::QueueWaiting), &sub, false, false, c(C::Ink), |ui| {
                check = key::show(ui, key::Key::new(&t(crate::house::i18n::Key::QueueDirectCheck), Role::Guide).enabled(free)).clicked();
            });
        });
        match said {
            Some(s) if s.tone == crate::house::seller::Tone::Bad => {
                let lang = crate::lang::lang();
                states::err_box(ui, "queue-direct-said", &crate::house::refusal::line(lang, &s.name, &s.detail), "", &t(crate::house::i18n::Key::U3RawError), &format!("{} · {}", s.name, s.detail));
            }
            Some(s) => states::okline(ui, if s.tone == crate::house::seller::Tone::Note { Mark::Busy } else { Mark::Ok }, &s.detail),
            None => {}
        }
        card::form(ui, |ui, f| {
            back = f.go(ui, &t(Key::ModeBack), "", true, "").clicked();
            give_up = free && f.danger(ui, &t(crate::house::i18n::Key::OrdersVoidKey)).clicked();
        });
        if back {
            self.queue_back();
        }
        if check {
            self.step(crate::house::action::Action::QueueDirectCheck, &[("tx", &tx)]);
        }
        if give_up {
            self.step(crate::house::action::Action::QueueDirectGiveUp, &[("tx", &tx)]);
        }
    }
}
