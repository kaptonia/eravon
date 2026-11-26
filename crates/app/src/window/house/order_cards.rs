use super::*;
use crate::house::action::Action as Step;

impl Win {
    pub(super) fn paste_card(&mut self, ctx: &egui::Context) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some(os) = self.shell.desk.sh.seller.orders.as_ref().filter(|o| o.paste_open) else { return };
        let mut text = os.paste.clone();
        let (_, send, close) = Self::sheet_with(ctx, "orders-paste", eravon_ui::metrics::MODAL_W, |ui, keys| {
            Self::sheet_head(ui, &t(crate::house::i18n::Key::OrdersPasteKey), "");
            input::area_hint(ui, &mut text, 3, &t(crate::house::i18n::Key::OrdersPasteHint));
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::OrdersPaste), Role::Primary, !text.trim().is_empty())));
        });
        if let Some(o) = self.shell.desk.sh.seller.orders.as_mut() {
            o.paste = text.clone();
        }
        if send {
            let v = text.clone();
            self.step(Step::OrderPaste, &[("v", &v)]);
        } else if close {
            self.step(Step::OrderPasteClose, &[]);
        }
    }

    pub(super) fn refund_card(&mut self, ctx: &egui::Context) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let said = self.shell.desk.sh.seller.orders.as_ref().and_then(|o| o.said.clone());
        let Some(c) = self.shell.desk.sh.seller.orders.as_ref().and_then(|o| o.card.clone()) else { return };
        let order = self.shell.desk.sh.seller.orders.as_ref().and_then(|o| o.order.clone());
        let m = |x: &str| crate::house::orders::money_as(lang, order.as_ref(), x, crate::house::coin::Form::Short);
        let full = |x: &str| format!("{} · {x}", crate::house::orders::money(lang, order.as_ref(), x));
        let lf_back = !c.lf_back.trim_start_matches('0').is_empty();
        let (_, send, close) = Self::sheet_with(ctx, "orders-refund", eravon_ui::metrics::MODAL_W, |ui, keys| {
            Self::sheet_head(ui, &t(crate::house::i18n::Key::OrdersCardTitle), "");
            Self::sheet_said(ui, "refund_card-said", said.as_ref());
            let (co, co_err) = self.order_route();
            if let Some(e) = &co_err {
                said_note(ui, "order_cards-said-3", lang, &e.name, &e.detail);
            } else if co {
                states::note_box(ui, &t(crate::house::i18n::Key::OrdersCompanyDeliver));
            }
            let mut rows = vec![
                (t(crate::house::i18n::Key::ExPrice), kv::Val::Mono(m(&c.amount))),
                (t(crate::house::i18n::Key::OrdersCardFee), kv::Val::Mono(m(&c.judge_fee))),
                (t(crate::house::i18n::Key::OrdersCardBuyer), kv::Val::Mono(m(&c.to_buyer))),
                (t(crate::house::i18n::Key::OrdersCardBond), kv::Val::Mono(m(&c.bond_back))),
            ];
            if lf_back {
                rows.push((t(crate::house::i18n::Key::OrdersCardLf), kv::Val::Mono(m(&c.lf_back))));
            }
            let refs: Vec<(&str, kv::Val)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            kv::kv(ui, &refs);
            if lf_back {
                states::note_box(ui, &t(crate::house::i18n::Key::OrdersCardDisputeNote));
            }
            if let Some(o) = &order {
                eravon_ui::fold::fold(ui, "orders-refund-det", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                    let mut det = vec![
                        (t(crate::house::i18n::Key::ExPrice), kv::Val::Mono(full(&c.amount))),
                        (t(crate::house::i18n::Key::OrdersCardFee), kv::Val::Mono(full(&c.judge_fee))),
                        (t(crate::house::i18n::Key::OrdersCardBuyer), kv::Val::Mono(full(&c.to_buyer))),
                        (t(crate::house::i18n::Key::OrdersCardBond), kv::Val::Mono(full(&c.bond_back))),
                    ];
                    if lf_back {
                        det.push((t(crate::house::i18n::Key::OrdersCardLf), kv::Val::Mono(full(&c.lf_back))));
                    }
                    det.push((t(crate::house::i18n::Key::SetContractAddr), kv::Val::Mono(o.instance.clone())));
                    let refs: Vec<(&str, kv::Val)> = det.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                    kv::kv(ui, &refs);
                });
            }
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&if co { t(crate::house::i18n::Key::OrdersCompanyExportKey) } else { t(crate::house::i18n::Key::OrdersCardTitle) }, Role::Commit, co_err.is_none())));
        });
        if send {
            self.step(Step::RefundSend, &[]);
        } else if close {
            self.step(Step::RefundClose, &[]);
        }
    }

    pub(super) fn evidence_card(&mut self, ctx: &egui::Context) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let said = self.shell.desk.sh.seller.orders.as_ref().and_then(|o| o.said.clone());
        let Some(m) = self.shell.desk.sh.seller.orders.as_ref().and_then(|o| o.evidence.clone()) else { return };
        let (mut kit, mut picked) = (None::<Option<String>>, None::<String>);
        let mut note = m.note.clone();
        let (co, co_err) = self.order_route();
        let (_, send, close, taken) = self.sheet_take(ctx, "orders-evidence", eravon_ui::metrics::MODAL_WIDE_W, crate::platform::Pick::FileOrFolder, |ui, keys| {
            Self::sheet_head(ui, &t(crate::house::i18n::Key::OrdersEvidenceKey), "");
            Self::sheet_said(ui, "evidence_card-said", said.as_ref());
            if let Some(e) = &co_err {
                said_note(ui, "order_cards-said-4", lang, &e.name, &e.detail);
            } else if co {
                states::note_box(ui, &t(crate::house::i18n::Key::OrdersCompanyDeliver));
            }
            card::group_title(ui, &t(crate::house::i18n::Key::OrdersKits));
            match &m.kits {
                Ok(ks) => {
                    let mut labels: Vec<String> = ks.iter().map(|(id, n, c)| format!("{id} · {n} · {}", crate::house::i18n::filln(crate::house::i18n::Key::BoFilesN, &[&c.to_string()]))).collect();
                    labels.push(t(crate::house::i18n::Key::OrdersKitNone));
                    let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                    let at = match &m.kit {
                        Some(id) => ks.iter().position(|(k, _, _)| k == id),
                        None => Some(ks.len()),
                    };
                    for (i, l) in refs.iter().enumerate() {
                        if eravon_ui::house::choice_line(ui, egui::Id::new(("order-kit", i)), at == Some(i), l).clicked() {
                            kit = Some(ks.get(i).map(|(id, _, _)| id.clone()));
                        }
                    }
                }
                Err(s) => match crate::house::settings::zk_away(s) {
                    Some(line) => states::note_box(ui, &line),
                    None => said_note(ui, "order_cards-said-5", lang, &s.name, &s.detail),
                },
            }
            chosen_row(ui, &m.path, lang);
            states::hint(ui, &t(crate::house::i18n::Key::OrdersDeliverOffline));
            if m.reading {
                eravon_ui::states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO);
            }
            match &m.sheet {
                Some(Ok((h, files))) => {
                    for (i, (n, b)) in files.iter().enumerate() {
                        eravon_ui::drop::file_row(ui, &format!("f{i}"), &eravon_ui::width::file_name(n), &size_say(*b as u64), false, "");
                    }
                    eravon_ui::fold::fold(ui, "orders-evidence-det", &t(crate::house::i18n::Key::ToastDetail), |ui| {
                        kv::kv(ui, &[(t(crate::house::i18n::Key::OrdersManifest).as_str(), kv::Val::Mono(h.clone()))]);
                    });
                }
                Some(Err(s)) => states::err_box(ui, "orders-evidence-err", &crate::house::refusal::line(lang, &s.name, &s.detail), &t(crate::house::i18n::Key::SaidNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
                None => {}
            }
            field(ui, &t(crate::house::i18n::Key::OrdersEvidenceNote), None, |ui| {
                input::line(ui, &mut note, "");
            });
            let go = if co { t(crate::house::i18n::Key::OrdersCompanyExportKey) } else { t(crate::house::i18n::Key::OrdersEvidenceKey) };
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&go, Role::Commit, co_err.is_none() && matches!(m.sheet, Some(Ok(_))))));
        });
        let addr = self.shell.desk.addr.clone();
        if let Some(os) = self.shell.desk.sh.seller.orders.as_mut() {
            if os.evidence.as_ref().is_some_and(|x| x.note != note) {
                os.evidence_note(&addr, &note);
            }
        }
        picked = taken.or(picked);
        if let Some(k) = kit {
            self.step(Step::EvidenceKit, &[("id", &k.unwrap_or_default())]);
        } else if let Some(p) = picked {
            self.step(Step::EvidencePick, &[("path", &p)]);
        } else if send {
            self.step(Step::EvidenceSend, &[]);
        } else if close {
            self.step(Step::EvidenceClose, &[]);
            self.shell.desk.flush();
        }
    }

    pub(super) fn settle_card(&mut self, ctx: &egui::Context) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let said = self.shell.desk.sh.seller.orders.as_ref().and_then(|o| o.said.clone());
        let Some(m) = self.shell.desk.sh.seller.orders.as_ref().and_then(|o| o.settle.clone()) else {
            self.shell.desk.settle_form = None;
            return;
        };
        let (mut pct, mut seg, mut free) = self.shell.desk.settle_form.clone().unwrap_or_else(|| {
            let seg = SETTLE_DAYS.iter().position(|d| d.to_string() == m.days.trim());
            (bps_to_pct(&m.bps), seg, if seg.is_none() { m.days.clone() } else { String::new() })
        });
        let second = matches!(&m.offer, Some(Ok(_)));
        let (offered, hit, back) = Self::sheet_pick(ctx, "orders-settle", eravon_ui::metrics::MODAL_WIDE_W, |ui, keys| {
            Self::sheet_head(ui, &t(crate::house::i18n::Key::OrdersSettleKey), &t(if second { crate::house::i18n::Key::OrdersSettleStep2 } else { crate::house::i18n::Key::OrdersSettleStep1 }));
            Self::sheet_said(ui, "settle_card-said", said.as_ref());
            let (co, co_err) = self.order_route();
            if let Some(e) = &co_err {
                said_note(ui, "order_cards-said-6", lang, &e.name, &e.detail);
            } else if co {
                states::note_box(ui, &t(crate::house::i18n::Key::OrdersCompanySettle));
            }
            match &m.offer {
                Some(Ok(v)) => {
                    if let Err(s) = &v.preview {
                        said_hint(ui, "order_cards-said-1", lang, &s.name, &s.detail, true);
                    }
                    let can_look = v.preview.is_ok();
                    if let Ok(l) = &v.preview {
                        kv::kv(ui, &[(t(crate::house::i18n::Key::OrdersSettleLink).as_str(), kv::Val::Mono(l.clone()))]);
                    }
                    Self::sheet_keys_n(keys, "", &[(&t(crate::house::i18n::Key::ToastClose), Role::Primary, true), (&t(crate::house::i18n::Key::GoodsCopyLink), Role::Secondary, can_look)]);
                    true
                }
                other => {
                    field(ui, &t(crate::house::i18n::Key::OrdersBps), None, |ui| {
                        input::line(ui, &mut pct, "60");
                    });
                    if !pct.trim().is_empty() && pct_to_bps(&pct).is_none() {
                        states::hint(ui, &t(crate::house::i18n::Key::OrdersPctBad));
                    }
                    field(ui, &t(crate::house::i18n::Key::OrdersDays), None, |ui| {
                        let labels = [t(crate::house::i18n::Key::OrdersDay1), t(crate::house::i18n::Key::OrdersDay3), t(crate::house::i18n::Key::OrdersDay7), t(crate::house::i18n::Key::OrdersDayFree)];
                        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
                        let cells: Vec<eravon_ui::seg::Cell> = refs.iter().map(|w| eravon_ui::seg::Cell { text: w, count: None }).collect();
                        let pick = ui.allocate_ui(egui::vec2(eravon_ui::house::FORM_SEG_W, eravon_ui::tokens::SEG_H), |ui| eravon_ui::seg::seg_fill(ui, "settle-days", &cells, seg.unwrap_or(3), eravon_ui::tokens::SEG_H)).inner;
                        if let Some(i) = pick {
                            seg = (i < 3).then_some(i);
                        }
                        if seg.is_none() {
                            input::line(ui, &mut free, &t(crate::house::i18n::Key::OrdersDaysHint));
                        }
                    });
                    if let Some(Err(s)) = other {
                        said_note(ui, "order_cards-said-7", lang, &s.name, &s.detail);
                    }
                    let days_now = seg.map_or_else(|| free.trim().to_string(), |i| SETTLE_DAYS[i].to_string());
                    let ready = pct_to_bps(&pct).is_some() && !days_now.is_empty();
                    let go = if co { t(crate::house::i18n::Key::OrdersCompanyExportKey) } else { t(crate::house::i18n::Key::OrdersMakeOffer) };
                    Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&go, Role::Commit, ready && co_err.is_none())));
                    false
                }
            }
        });
        let (make, look, close) = if offered { (false, hit == Some(1), hit == Some(0) || back) } else { (hit == Some(0), false, back) };
        let bps = pct_to_bps(&pct).unwrap_or_default();
        let days = seg.map_or_else(|| free.trim().to_string(), |i| SETTLE_DAYS[i].to_string());
        self.shell.desk.settle_form = Some((pct, seg, free));
        if let Some(x) = self.shell.desk.sh.seller.orders.as_mut().and_then(|o| o.settle.as_mut()) {
            x.bps = bps.clone();
            x.days = days.clone();
        }
        if make {
            self.step(Step::SettleMake, &[("bps", &bps), ("days", &days)]);
        } else if look {
            let url = self.shell.desk.sh.seller.orders.as_mut().and_then(|o| o.settle_preview());
            if let Some(u) = &url {
                ctx.copy_text(u.clone());
                tell_copied(ctx, Some(u.clone()));
            }
            self.shell.desk.tr.push(crate::house::action::Action::SettlePreview, eravon_dispute::mark::FEATURE, url.unwrap_or_default());
            self.shell.desk.flush();
        } else if close {
            self.step(Step::SettleClose, &[]);
        }
    }

    pub(super) fn deliver_card(&mut self, ctx: &egui::Context) {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let said = self.shell.desk.sh.seller.orders.as_ref().and_then(|o| o.said.clone());
        let Some(m) = self.shell.desk.sh.seller.orders.as_ref().and_then(|o| o.deliver.clone()) else { return };
        let mut picked = None::<String>;
        let (mut note, mut method) = (m.note.clone(), m.method.clone());
        let (co, co_err) = self.order_route();
        let (_, send, close, taken) = self.sheet_take(ctx, "orders-deliver", eravon_ui::metrics::MODAL_WIDE_W, crate::platform::Pick::FileOrFolder, |ui, keys| {
            Self::sheet_head(ui, &t(crate::house::i18n::Key::OrdersDeliverKey), "");
            Self::sheet_said(ui, "deliver_card-said", said.as_ref());

            if let Some(e) = &co_err {
                said_note(ui, "order_cards-said-8", lang, &e.name, &e.detail);
            } else if co {
                states::note_box(ui, &t(crate::house::i18n::Key::OrdersCompanyDeliver));
            }
            chosen_row(ui, &m.path, lang);
            states::hint(ui, &t(crate::house::i18n::Key::OrdersDeliverOffline));
            if m.reading {
                eravon_ui::states::skeleton_lines(ui, &eravon_ui::house::SKELETON_TWO);
            }
            match &m.sheet {
                Some(Ok((h, files))) => {
                    for (i, (n, b)) in files.iter().enumerate() {
                        eravon_ui::drop::file_row(ui, &format!("f{i}"), &eravon_ui::width::file_name(n), &size_say(*b as u64), false, "");
                    }
                    if !files.is_empty() {
                        states::hint(ui, &crate::house::i18n::filln(crate::house::i18n::Key::OrdersDeliverNow, &[&files.len().to_string()]));
                    }
                    details_list(ui, "orders-deliver-det", &[(&t(crate::house::i18n::Key::OrdersManifest), kv::Val::Mono(h.clone()))]);
                }
                Some(Err(s)) => said_hint(ui, "order_cards-said-2", lang, &s.name, &s.detail, true),
                None => {}
            }
            field(ui, &t(crate::house::i18n::Key::OrdersNote), None, |ui| {
                input::line(ui, &mut note, "");
            });
            field(ui, &t(crate::house::i18n::Key::OrdersMethod), None, |ui| {
                input::line(ui, &mut method, "");
            });
            if !m.key_on {
                states::note_box(ui, &crate::house::i18n::library(lang, &m.key_note));
            }
            let ready = m.key_on && co_err.is_none() && matches!(m.sheet, Some(Ok(_)));
            let go = if co { t(crate::house::i18n::Key::OrdersCompanyExportKey) } else { t(crate::house::i18n::Key::OrdersDeliverKey) };
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&go, Role::Commit, ready)));
        });
        let addr = self.shell.desk.addr.clone();
        if let Some(os) = self.shell.desk.sh.seller.orders.as_mut() {
            if let Some(x) = os.deliver.as_mut() {
                x.method = method;
            }
            if os.deliver.as_ref().is_some_and(|x| x.note != note) {
                os.deliver_note(&addr, &note);
            }
        }
        picked = taken.or(picked);
        if let Some(p) = picked {
            self.step(Step::DeliverPick, &[("path", &p)]);
        } else if send {
            self.step(Step::DeliverSend, &[]);
        } else if close {
            self.step(Step::DeliverClose, &[]);
        }
    }
}

const SETTLE_DAYS: [u32; 3] = [1, 3, 7];

fn pct_to_bps(s: &str) -> Option<String> {
    let s = s.trim();
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    if int.is_empty() || frac.len() > 2 || !int.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let bps = int.parse::<u64>().ok()?.checked_mul(100)? + format!("{frac:0<2}").parse::<u64>().ok()?;
    (bps <= 10_000).then(|| bps.to_string())
}

fn bps_to_pct(s: &str) -> String {
    match s.trim().parse::<u64>() {
        Ok(b) if b % 100 == 0 => (b / 100).to_string(),
        Ok(b) => format!("{}.{:02}", b / 100, b % 100).trim_end_matches('0').to_string(),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod pct {
    #[test]
    fn percent_turns_into_bps_and_back() {
        assert_eq!(super::pct_to_bps("12.5").as_deref(), Some("1250"));
        assert_eq!(super::pct_to_bps("100").as_deref(), Some("10000"));
        assert_eq!(super::pct_to_bps("100.01"), None);
        assert_eq!(super::pct_to_bps("1.234"), None);
        assert_eq!(super::bps_to_pct("1250"), "12.5");
        assert_eq!(super::bps_to_pct("10000"), "100");
    }
}
