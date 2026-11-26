use super::*;
use crate::house::action::Action as Step;

impl Win {
    pub(super) fn cands_page(&mut self, ui: &mut egui::Ui) {
        use eravon_cands::table::Side;
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        if self.shell.page_in_flight(crate::task::Kind::Cands) && self.shell.desk.sh.seller.cands.is_none() {
            Self::home_skeleton(ui, 0);
            return;
        }
        let Some(c) = self.shell.desk.sh.seller.cands.clone() else { return };
        if c.detail.is_some() {
            self.cands_detail(ui, &c);
            return;
        }
        let mut open: Option<String> = None;
        match &c.rows {
            Err(s) => states::err_box(ui, "cands", &t(crate::house::i18n::Key::CandsTableFailed), &t(crate::house::i18n::Key::HomeUnsyncedNext), &t(crate::house::i18n::Key::U3RawError), &raw_of(&s.name, &s.detail)),
            Ok(rows) => {
                let mine: Vec<_> = rows.iter().filter(|b| b.row.kind == c.tab.word() || Side::of(&b.row.kind).is_none()).collect();
                let heads = [t(if c.tab == Side::Judge { crate::house::i18n::Key::PlCands } else { crate::house::i18n::Key::CandsGuild }), t(crate::house::i18n::Key::SetZkState)];
                let cols = [table::col(&heads[0], eravon_ui::house::REST), table::col(&heads[1], table::Col::Auto { min: eravon_ui::house::COL_NARROW, max: f32::MAX }), table::CHEV];
                let make = |i: usize| {
                        let b = mine[i];
                        if c.unread {
                            let text = match &b.flaw {
                                Some(f) => format!("{} · {} · {f}", b.row.label, short(&b.row.address)),
                                None => format!("{} · {}", b.row.label, short(&b.row.address)),
                            };
                            let pill = (crate::house::i18n::get(lang, crate::house::i18n::Key::SetNotRead).0, eravon_ui::palette::Tone::Grey);
                            return table::Row { cells: vec![table::Cell::Text(text), table::Cell::Pill(pill.0, pill.1), table::Cell::Chev], click: true, gone: false, on: false };
                        }
                        let mut sub = match (&b.state, c.tab) {
                            (Ok(s), Side::Judge) => format!(
                                "{} · {}",
                                law_word(lang, s.version, s.unread.as_ref()),
                                if s.accepting { t(crate::house::i18n::Key::CandsAccepting) } else { t(crate::house::i18n::Key::CandsClosedPill) }
                            ),
                            (Ok(s), Side::Guild) => format!(
                                "{} {} · {}",
                                s.members,
                                t(crate::house::i18n::Key::CandsMembers),
                                if s.pool { t(crate::house::i18n::Key::GoodsGuildPool) } else { t(crate::house::i18n::Key::GoodsGuildSet) }
                            ),
                            (Err(a), _) => crate::house::i18n::absent_detail(lang, a),
                        };
                        if let Ok(s) = &b.state
                            && !s.current.is_empty()
                            && !s.current.eq_ignore_ascii_case(&b.row.address)
                        {
                            sub = format!("{sub} · {}", crate::fill!(crate::house::i18n::Key::CandsMovedTo; short(&s.current)));
                        }
                        if let Ok(s) = &b.state
                            && !s.resign_items.is_empty()
                        {
                            sub = format!("{sub} · {}", resign_line(lang, &s.resign_items));
                        }
                        if let Some(f) = &b.flaw {
                            sub = format!("{f} · {sub}");
                        }
                        let pill = cand_pill(b, lang);
                        table::Row { cells: vec![table::Cell::Text(format!("{} · {} · {}", b.row.label.clone(), short(&b.row.address), sub)), table::Cell::Pill(pill.0, pill.1), table::Cell::Chev], click: true, gone: false, on: false }
                };
                let none = t(crate::house::i18n::Key::HomeNone);
                if let Some(i) = table::table_with(ui, "cands", table::Spec::new(&cols, self.shell.desk.rows_gen).head(true).empty(&none), mine.len(), make).clicked {
                    open = Some(mine[i].row.address.clone());
                }
            }
        }
        if let Some(a) = open {
            self.step(Step::CandOpen, &[("a", &a)]);
        }
        self.add_sheet(ui.ctx());
    }

    pub(super) fn add_sheet(&mut self, ctx: &egui::Context) {
        use eravon_cands::table::Side;
        let Some((side, mut input_v, mut label)) = self.shell.desk.add_form.clone() else { return };
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let title = if side == Side::Judge { t(crate::house::i18n::Key::CandsAddJudge) } else { t(crate::house::i18n::Key::CandsAddGuild) };
        let what = if side == Side::Judge { t(crate::house::i18n::Key::Address) } else { t(crate::house::i18n::Key::CandsPointer) };
        let said = self.shell.desk.sh.seller.cands.as_ref().and_then(|c| c.said.clone());
        let (_, go, back) = Self::sheet_with(ctx, "cand-add", eravon_ui::metrics::MODAL_W, |ui, keys| {
            Self::sheet_head(ui, &title, "");
            ui.add_space(eravon_ui::metrics::CARD_GAP);
            field(ui, &what, None, |ui| {
                let hint = if side == Side::Judge { "0x…".to_string() } else { eravon_cands::table::pointer_hint() };
                input::line(ui, &mut input_v, &hint);
            });
            field(ui, &t(crate::house::i18n::Key::IdLabel), None, |ui| {
                input::line(ui, &mut label, "");
            });
            Self::sheet_said(ui, "cand-add-said", said.as_ref());
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::ReadNetAdd), Role::Primary, !input_v.trim().is_empty() && !label.trim().is_empty())));
        });
        if back {
            self.shell.desk.add_form = None;
            return;
        }
        self.shell.desk.add_form = Some((side, input_v.clone(), label.clone()));
        if go {
            self.step(Step::CandAdd, &[("kind", side.word()), ("a", &input_v), ("label", &label)]);
        }
    }

    pub(super) fn cands_detail(&mut self, ui: &mut egui::Ui, c: &crate::house::seller::Cands) {
        use eravon_cands::view::Detail;
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let Some((row, got, warn)) = &c.detail else { return };
        if let Some(tok) = &c.lapse {
            if let Ok((clocks, guild)) = c.lapse_of(tok) {
                super::casepages::lapse_page(ui, &row.label, guild, &clocks);
            }
            return;
        }
        let judge = matches!(got, Ok(Detail::Judge(_))) || row.kind == "judge";
        let labels: Vec<String> = if judge {
            [crate::house::i18n::Key::CandsTabOverview, crate::house::i18n::Key::CandsLaw, crate::house::i18n::Key::CandsTabQuotes, crate::house::i18n::Key::CandsTabRecords].iter().map(|k| crate::house::i18n::get(lang, *k).0).collect()
        } else {
            [crate::house::i18n::Key::CandsTabOverview, crate::house::i18n::Key::CandsLaw, crate::house::i18n::Key::CandsTabQuotes, crate::house::i18n::Key::CandsTabMembers].iter().map(|k| crate::house::i18n::get(lang, *k).0).collect()
        };
        let refs: Vec<&str> = labels.iter().map(|x| x.as_str()).collect();
        {
            let (sub, pill) = match got {
                Ok(Detail::Judge(j)) => (
                    format!("{} · {}", t(crate::house::i18n::Key::PlCands), law_word(lang, j.version, j.law().err().as_ref())),
                    if j.accepting { (t(crate::house::i18n::Key::CandsAccepting), eravon_ui::palette::Tone::Ok) } else { (t(crate::house::i18n::Key::CandsPaused), eravon_ui::palette::Tone::Grey) },
                ),
                Ok(Detail::Guild(g)) => (format!("{} · {} {}", t(crate::house::i18n::Key::CandsGuild), g.members.len(), t(crate::house::i18n::Key::CandsMembers)), if g.unpaced() { (t(crate::house::i18n::Key::CandsUnpaced), eravon_ui::palette::Tone::Warn) } else { (t(crate::house::i18n::Key::HomeChoosable), eravon_ui::palette::Tone::Ok) }),
                Err(a) => (t(if row.kind == "guild" { crate::house::i18n::Key::CandsGuild } else { crate::house::i18n::Key::PlCands }), (crate::house::i18n::get_named(lang, &format!("cands.absent.{}", a.class())).0, eravon_ui::palette::Tone::Bad)),
            };
            Self::hero(ui, &row.label, &sub, &[hero_pill(&pill.0, pill.1)]);
        }
        if got.is_ok() {
            if let Some(i) = eravon_ui::seg::tabs(ui, &&refs.join("\u{1f}"), &refs, c.dtab) {
                self.step(Step::CandTab, &[("i", &i.to_string())]);
            }
        }
        if let Some(w) = warn {
            states::banner(ui, states::Banner::Warn, w, |_| ());
        }
        let mut member_open: Option<String> = None;
        let mut url: Option<(&'static str, Option<String>)> = None;
        let mut lapse: Option<String> = None;
        let (mut rename, mut remove) = (false, false);
        match got {
            Err(a) => {
                let (head, next) = match a {
                    eravon_cands::fault::Absent::Succeeded { .. } => (crate::house::i18n::absent_detail(lang, a), crate::house::refusal::next(lang, a.name()).unwrap_or_default()),
                    _ => (crate::house::i18n::get_named(lang, &format!("cands.absent.{}", a.class())).0, t(crate::house::i18n::Key::CandsAbsentNext)),
                };
                states::err_box(ui, "cand-absent", &head, &next, &t(crate::house::i18n::Key::U3RawError), &format!("{} · {}", a.name(), a.detail()));
                let addr_label = t(if row.kind == "guild" { crate::house::i18n::Key::CandsPointer } else { crate::house::i18n::Key::Address });
                let succ_label = t(crate::house::i18n::Key::CandsSuccessor);
                let mut rows: Vec<(&str, kv::Val)> = vec![(&addr_label, kv::Val::Mono(row.address.clone()))];
                if let eravon_cands::fault::Absent::Succeeded { to, block } = a {
                    rows.push((&succ_label, kv::Val::Mono(format!("{to} · {}", crate::house::i18n::filln(crate::house::i18n::Key::CandsAtBlock, &[&block.to_string()])))));
                }
                details_card(ui, "cand-absent-raw", &rows);
                card::form(ui, |ui, f| {
                    rename = f.go(ui, &t(crate::house::i18n::Key::CandsRename), "", true, "").clicked();
                    remove = f.danger(ui, &t(crate::house::i18n::Key::CandsRemoveDots)).clicked();
                });
            }
            Ok(Detail::Judge(j)) => match c.dtab {
                0 => {
                    let avail = match &j.ledger {
                        Ok(l) => {
                            let head = t(if j.offers() { crate::house::i18n::Key::CandsAvailOk } else { crate::house::i18n::Key::CandsAvailBad });
                            audit_say(&l.label, lang).map_or(head.clone(), |w| format!("{head} · {w}"))
                        }
                        Err(e) => format!("{} · {e}", t(crate::house::i18n::Key::CandsAvailBad)),
                    };
                    let latest = j.records.as_ref().map(|r| match r {
                        Ok(list) => list.last().map_or(t(crate::house::i18n::Key::BoRecordNone), |r| format!("{} · {}", date_of(r.at), r.label)),
                        Err(_) => t(crate::house::i18n::Key::CandsRecordsDown),
                    });
                    card::section(ui, &t(crate::house::i18n::Key::GoodsBasics), "", |ui| {
                        let copied = card::form(ui, |ui, f| {
                            rename = f.go(ui, &t(crate::house::i18n::Key::IdLabel), &row.label, true, "").clicked();
                            f.info(ui, &t(crate::house::i18n::Key::GoodsName), card::Show::Text(j.name.as_deref().unwrap_or("·")));
                            let copied = f.info(ui, &format!("{} · {}", t(crate::house::i18n::Key::Address), code_word(&c.codes, &j.address, lang)), card::Show::Short(&j.address));
                            if let Some(h) = &j.home {
                                if super::casepages::link_row(ui, f, &t(crate::house::i18n::Key::CandsHome), h) {
                                    url = Some(("judge-home", None));
                                }
                            }
                            let law = match j.law() {
                                Ok(v) => crate::house::i18n::filln(crate::house::i18n::Key::CandsVersion, &[&v.to_string()]),
                                Err(u) => format!("{} · {}", t(crate::house::i18n::Key::CandsLawUnread), unread_word(lang, &u)),
                            };
                            f.info(ui, &t(crate::house::i18n::Key::CandsLaw), card::Show::Text(&format!("{law} · {}", crate::house::i18n::filln(crate::house::i18n::Key::CandsPublished, &[&date_of(j.published)]))));
                            f.info(ui, &t(crate::house::i18n::Key::CandsAccept), card::Show::Text(&if j.accepting { t(crate::house::i18n::Key::CandsAccepting) } else { t(crate::house::i18n::Key::CandsPaused) }));
                            f.info(ui, &t(crate::house::i18n::Key::CandsLedger), card::Show::Text(&avail));
                            if let Some(l) = &latest {
                                f.info(ui, &t(crate::house::i18n::Key::CandsLatest), card::Show::Text(l));
                            }
                            copied
                        });
                        tell_copied(ui.ctx(), copied);
                    });
                    if !j.trail.is_empty() || !j.attempts.is_empty() {
                        let hops = std::iter::once(j.listed.clone()).chain(j.trail.iter().filter(|a| !a.eq_ignore_ascii_case(&j.listed)).cloned()).chain(std::iter::once(j.address.clone())).map(|a| format!("{a} · {}", code_word(&c.codes, &a, lang))).collect::<Vec<_>>().join("\n");
                        let (tl, al) = (t(crate::house::i18n::Key::CandsTrail), t(crate::house::i18n::Key::CandsAttempts));
                        let mut rows: Vec<(&str, kv::Val)> = Vec::new();
                        if !j.trail.is_empty() {
                            rows.push((&tl, kv::Val::Mono(hops)));
                        }
                        if !j.attempts.is_empty() {
                            rows.push((&al, kv::Val::Mono(attempts_line(&j.attempts, &c.codes, lang))));
                        }
                        let says: Vec<String> = [(!j.trail.is_empty(), crate::house::i18n::Key::CandsTrailSay), (!j.attempts.is_empty(), crate::house::i18n::Key::CandsAttemptsSay)].into_iter().filter(|(on, _)| *on).map(|(_, k)| t(k)).collect();
                        card::section(ui, &t(crate::house::i18n::Key::CandsTrail), &says.join(" "), |ui| details_list(ui, "judge-trail", &rows));
                    }
                    if let Some(n) = j.note_md.as_ref().filter(|n| !n.trim().is_empty()) {
                        card::section(ui, &t(crate::house::i18n::Key::JudgeNote), "", |ui| super::casepages::text_box(ui, "cand-judge-note", n, eravon_ui::house::TEXT_BOX_MAX_H));
                    }
                    beyond_rows(ui, &j.beyond, lang);
                    if j.needs_resign {
                        states::banner(ui, states::Banner::Warn, &resign_line(lang, &j.resign_items), |_| ());
                    }
                    card::form(ui, |ui, f| remove = f.danger(ui, &t(crate::house::i18n::Key::CandsRemoveDots)).clicked());
                }
                1 => match &j.text {
                    Some(x) => {
                        card::section(ui, &law_word(lang, j.version, j.law().err().as_ref()), "", |ui| super::casepages::text_whole(ui, x));
                        details_card(ui, "judge-word", &[("amanahHash", kv::Val::Mono(j.amanah.clone()))]);
                    }
                    None => states::empty(ui, Glyph::Ledger, &t(crate::house::i18n::Key::CandsNoLawText)),
                },
                2 => lapse = self.quote_tables(ui, &j.quotes, &j.decimals),
                _ if j.records.is_none() => {}
                _ => match &j.records {
                    None => {}
                    Some(Err(e)) => states::err_box(ui, "records", &t(crate::house::i18n::Key::CandsRecordsDown), e, "", ""),
                    Some(Ok(list)) if list.is_empty() => states::empty(ui, Glyph::Receipt, &t(crate::house::i18n::Key::BoRecordNone)),
                    Some(Ok(list)) => {
                        card::form(ui, |ui, f| {
                            for r in list.iter().rev() {
                                f.info(ui, &date_of(r.at), card::Show::Text(&format!("{} · {}{}", short(&r.instance), r.label, r.red.as_ref().map_or(String::new(), |x| format!(" · {x}")))));
                            }
                        });
                    }
                },
            },
            Ok(Detail::Guild(g)) => match c.dtab {
                0 => {
                    let home = g.home.clone();
                    card::section(ui, &t(crate::house::i18n::Key::GoodsBasics), "", |ui| {
                        let copied = card::form(ui, |ui, f| {
                            rename = f.go(ui, &t(crate::house::i18n::Key::IdLabel), &row.label, true, "").clicked();
                            f.info(ui, &t(crate::house::i18n::Key::GoodsName), card::Show::Text(if g.card_unread() { g.card } else { g.name.as_deref().unwrap_or("·") }));
                            let copied = f.info(ui, &format!("{} · {}", t(crate::house::i18n::Key::Address), code_word(&c.codes, &g.address, lang)), card::Show::Short(&g.address));
                            if let Some(h) = &home {
                                if super::casepages::link_row(ui, f, &t(crate::house::i18n::Key::CandsHome), h) {
                                    url = Some(("guild-home", None));
                                }
                            }
                            f.info(ui, &t(crate::house::i18n::Key::CandsWay), card::Show::Text(&if g.pool { t(crate::house::i18n::Key::CandsPoolWay) } else { t(crate::house::i18n::Key::CandsSetWay) }));
                            f.info(ui, &t(crate::house::i18n::Key::CandsPanel), card::Show::Text(&format!("{} {}", g.members.len(), t(crate::house::i18n::Key::CandsMembers))));
                            match &g.terms {
                                Some(x) => f.info(ui, &t(crate::house::i18n::Key::CandsGuildLaw), card::Show::Text(&format!("{} · {}", x.version.map_or_else(|| t(crate::house::i18n::Key::CandsUnreadNoVersion), |v| crate::house::i18n::filln(crate::house::i18n::Key::CandsVersion, &[&v.to_string()])), crate::house::i18n::filln(crate::house::i18n::Key::CandsPublished, &[&date_of(g.published)])))),
                                None => f.info(ui, &t(crate::house::i18n::Key::CandsGuildLaw), card::Show::Empty(&t(crate::house::i18n::Key::CandsNoGuildLaw))),
                            };
                            let pl = t(crate::house::i18n::Key::CandsPointer);
                            let (detail, widest) = detail_rows_of(ui, &[(&pl, kv::Val::Mono(g.pointer.clone()))]);
                            f.details(ui, "guild-overview-raw", &t(crate::house::i18n::Key::SetEvidence), &detail, widest).or(copied)
                        });
                        tell_copied(ui.ctx(), copied);
                    });
                    if let Some(w) = g.words.as_ref().filter(|w| !w.trim().is_empty()) {
                        card::section(ui, &t(crate::house::i18n::Key::CandsGuildWords), "", |ui| super::casepages::text_box(ui, "cand-guild-words", w, eravon_ui::house::TEXT_BOX_MAX_H));
                    }
                    beyond_rows(ui, &g.beyond, lang);
                    if g.needs_resign {
                        states::banner(ui, states::Banner::Warn, &resign_line(lang, &g.resign_items), |_| ());
                    }
                    card::form(ui, |ui, f| remove = f.danger(ui, &t(crate::house::i18n::Key::CandsRemoveDots)).clicked());
                }
                1 => match &g.terms {
                    None => states::empty(ui, Glyph::Ledger, &t(crate::house::i18n::Key::CandsNoGuildLaw)),
                    Some(x) => {
                        match &x.text {
                            Ok(body) => card::section(ui, &x.version.map_or_else(|| format!("{} · {}", t(crate::house::i18n::Key::CandsGuildLaw), t(crate::house::i18n::Key::CandsUnreadNoVersion)), |v| format!("{} v{v}", t(crate::house::i18n::Key::CandsGuildLaw))), "", |ui| super::casepages::text_whole(ui, body)),
                            Err(e) => states::err_box(ui, "guild-law", &t(crate::house::i18n::Key::CandsLawDown), e, "", ""),
                        }
                        details_card(ui, "guild-sha", &[("sha256", kv::Val::Mono(x.sha256.clone()))]);
                    }
                },
                2 => lapse = self.quote_tables(ui, &g.quotes, &g.decimals),
                _ => {
                    let make = |i: usize| {
                            let m = &g.members[i];
                            let at = |a: &str| format!("{a} · {}", code_word(&c.codes, a, lang));
                            let addr = if m.resolved.eq_ignore_ascii_case(&m.listed) { at(&m.listed) } else { format!("{} → {}", at(&m.listed), at(&m.resolved)) };
                            let text = match m.name.as_ref().filter(|n| !n.trim().is_empty()) {
                                Some(n) => format!("{n} · {addr}"),
                                None => addr,
                            };
                            table::Row { cells: vec![table::Cell::Text(text), table::Cell::Chev], click: true, gone: false, on: false }
                    };
                    if let Some(i) = table::table_with(ui, "guild-members", table::Spec::new(&[table::col("", eravon_ui::house::REST), table::CHEV], self.shell.desk.rows_gen), g.members.len(), make).clicked {
                        let m = &g.members[i];
                        member_open = Some(m.resolved.clone());
                    }
                    for (i, m) in g.members.iter().enumerate() {
                        if m.home.is_none() && m.note_md.as_ref().is_none_or(|n| n.trim().is_empty()) && m.attempts.is_empty() && m.beyond.is_empty() {
                            continue;
                        }
                        let head = m.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| short(&m.listed));
                        card::form(ui, |ui, f| {
                            f.fold(ui, &format!("guild-member-{i}"), &head, "", |ui, f| {
                                if let Some(h) = &m.home {
                                    if super::casepages::link_row(ui, f, &t(crate::house::i18n::Key::CandsHome), h) {
                                        url = Some(("judge-home", Some(m.listed.clone())));
                                    }
                                }
                                if !m.attempts.is_empty() {
                                    f.info(ui, &t(crate::house::i18n::Key::CandsAttempts), card::Show::Mono(&attempts_line(&m.attempts, &c.codes, lang)));
                                }
                                if let Some(n) = m.note_md.as_ref().filter(|n| !n.trim().is_empty()) {
                                    f.free(ui, |ui| md_view(ui, n));
                                }
                                for (k, v) in &m.beyond {
                                    f.info(ui, k, card::Show::Mono(v));
                                }
                            });
                        });
                    }
                }
            },
        }
        if let Some(a) = member_open {
            self.step(Step::CandOpen, &[("a", &a), ("member", "true")]);
        }
        if let Some((k, a)) = url {
            self.open_foreign_of(k, a.as_deref());
        }
        if let Some(tok) = lapse {
            self.step(Step::LapseOpen, &[("a", &tok)]);
        }
        if rename {
            self.shell.desk.rename = Some(row.label.clone());
        }
        if remove {
            self.shell.desk.confirm_remove = true;
        }
        self.rename_sheet(ui.ctx(), &row.address.clone());
        self.remove_card(ui.ctx(), &row.address.clone());
    }

}

pub(super) fn cand_pill(b: &eravon_cands::view::Brief, lang: crate::lang::Lang) -> (String, eravon_ui::palette::Tone) {
    use crate::house::seller::Pill;
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    let absent = || b.state.as_ref().err().map(|a| crate::house::i18n::get_named(lang, &format!("cands.absent.{}", a.class())).0).unwrap_or_default();
    match crate::house::seller::pill_of(b) {
        Pill::Flawed => (t(crate::house::i18n::Key::CandsFlawed), eravon_ui::palette::Tone::Bad),
        Pill::Succeeded => (absent(), eravon_ui::palette::Tone::Warn),
        Pill::Absent => (absent(), eravon_ui::palette::Tone::Bad),
        Pill::Closed => (t(crate::house::i18n::Key::CandsClosedPill), eravon_ui::palette::Tone::Grey),
        Pill::Resign => (t(crate::house::i18n::Key::CandsResign), eravon_ui::palette::Tone::Warn),
        Pill::Choosable => (t(crate::house::i18n::Key::HomeChoosable), eravon_ui::palette::Tone::Ok),
        Pill::Unpaced => (t(crate::house::i18n::Key::CandsUnpaced), eravon_ui::palette::Tone::Warn),
        Pill::NoQuote => (t(crate::house::i18n::Key::CandsNoQuote), eravon_ui::palette::Tone::Grey),
    }
}

pub(super) fn home_pill(cands: Option<&crate::house::seller::Cands>, me: &str, address: &str, lang: crate::lang::Lang) -> (String, eravon_ui::palette::Tone) {
    cands
        .filter(|c| c.me.eq_ignore_ascii_case(me) && !c.unread)
        .and_then(|c| c.rows.as_ref().ok())
        .and_then(|rows| rows.iter().find(|b| b.row.address.eq_ignore_ascii_case(address)))
        .map_or_else(|| (crate::house::i18n::get(lang, crate::house::i18n::Key::SetNotRead).0, eravon_ui::palette::Tone::Grey), |b| cand_pill(b, lang))
}

fn law_word(lang: crate::lang::Lang, version: Option<u64>, unread: Option<&eravon_cands::judge::Unread>) -> String {
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    match (unread, version) {
        (None, Some(v)) => format!("{} {}", t(crate::house::i18n::Key::CandsLaw), crate::house::i18n::filln_in(lang, crate::house::i18n::Key::CandsVersion, &[&v.to_string()])),
        (Some(u), _) => format!("{} · {}", t(crate::house::i18n::Key::CandsLawUnread), unread_word(lang, u)),
        (None, None) => format!("{} · {}", t(crate::house::i18n::Key::CandsLawUnread), unread_word(lang, &eravon_cands::judge::Unread::NoVersion)),
    }
}

fn unread_word(lang: crate::lang::Lang, u: &eravon_cands::judge::Unread) -> String {
    use eravon_cands::judge::Unread;
    crate::house::i18n::get(
        lang,
        match u {
            Unread::Ledger(_) => crate::house::i18n::Key::CandsUnreadLedger,
            Unread::Stranger => crate::house::i18n::Key::CandsUnreadStranger,
            Unread::NotInLedger => crate::house::i18n::Key::CandsUnreadNotIn,
            Unread::Discarded => crate::house::i18n::Key::CandsUnreadDiscarded,
            Unread::NoVersion => crate::house::i18n::Key::CandsUnreadNoVersion,
        },
    )
    .0
}

fn resign_line(lang: crate::lang::Lang, items: &[String]) -> String {
    crate::house::i18n::filln_in(lang, crate::house::i18n::Key::CandsResignItems, &[&items.join(&crate::house::i18n::get(lang, crate::house::i18n::Key::ListJoin).0)])
}

fn code_word(codes: &[(String, Result<bool, crate::house::seller::Said>)], a: &str, lang: crate::lang::Lang) -> String {
    let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
    match codes.iter().find(|(x, _)| x.eq_ignore_ascii_case(a)).map(|(_, r)| r) {
        Some(Ok(true)) => t(crate::house::i18n::Key::CandsIsContract),
        Some(Ok(false)) => t(crate::house::i18n::Key::CandsIsKey),
        _ => t(crate::house::i18n::Key::CandsCodeUnread),
    }
}

fn attempts_line(attempts: &[eravon_cands::judge::Attempt], codes: &[(String, Result<bool, crate::house::seller::Said>)], lang: crate::lang::Lang) -> String {
    attempts.iter().map(|a| format!("{} · {} · {}", a.to, code_word(codes, &a.to, lang), crate::house::i18n::filln(crate::house::i18n::Key::CandsAtBlock, &[&a.block.to_string()]))).collect::<Vec<_>>().join("\n")
}

fn beyond_rows(ui: &mut egui::Ui, beyond: &[(String, String)], lang: crate::lang::Lang) {
    if beyond.is_empty() {
        return;
    }
    let refs: Vec<(&str, kv::Val)> = beyond.iter().map(|(k, v)| (k.as_str(), kv::Val::Mono(v.clone()))).collect();
    card::section(ui, &crate::house::i18n::get(lang, crate::house::i18n::Key::CandsBeyond).0, "", |ui| details_list(ui, "cand-beyond", &refs));
}

impl Win {
    pub(super) fn quote_tables(&mut self, ui: &mut egui::Ui, quotes: &[eravon_cands::quote::Row], decimals: &[(String, Option<u64>)]) -> Option<String> {
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        if quotes.is_empty() {
            states::empty(ui, Glyph::Receipt, &t(crate::house::i18n::Key::CandsNoQuote));
            return None;
        }
        let mut hit = None;
        for q in quotes {
            match &q.quote {
                Err(e) => states::err_box(ui, "quote", &t(crate::house::i18n::Key::CandsQuoteBad), e, "", ""),
                Ok(x) => {
                    let c = eravon_cands::quote::cells(x, &q.token, decimals, lang);
                    let clocks = eravon_cands::lapse::clocks(x);
                    let foot = crate::house::i18n::filln(crate::house::i18n::Key::CandsQExampleAt, &[&c.example_amount]);
                    card::section(ui, &crate::house::coin::name(lang, &q.token), &foot, |ui| {
                        card::form(ui, |ui, f| {
                            f.info(ui, &t(crate::house::i18n::Key::CandsQPremium), card::Show::Text(&format!("{} · {} {}", c.premium, t(crate::house::i18n::Key::CandsQExample), c.example_premium)));
                            f.info(ui, &t(crate::house::i18n::Key::OrdersKvLf), card::Show::Text(&format!("{} · {} {}", c.lawsuit, t(crate::house::i18n::Key::CandsQExample), c.example_lawsuit)));
                            if let Ok(k) = &clocks {
                                f.info(ui, &t(crate::house::i18n::Key::LapseOfferWindowName), card::Show::Text(&span_text(k.offer_window)));
                            }
                            f.info(ui, &t(crate::house::i18n::Key::CandsQRuling), card::Show::Text(&c.ruling_days));
                            match &clocks {
                                Ok(k) => {
                                    if f.go(ui, &t(crate::house::i18n::Key::CandsQLapse), &super::casepages::lapse_word(k.lapse), true, "").clicked() {
                                        hit = Some(q.token.clone());
                                    }
                                }
                                Err(e) => {
                                    f.info(ui, &t(crate::house::i18n::Key::CandsQLapse), card::Show::Text(e));
                                }
                            }
                            f.info(ui, &t(crate::house::i18n::Key::CandsQCap), card::Show::Text(&c.cap));
                            f.info(ui, &t(crate::house::i18n::Key::CandsQAccept), card::Show::Text(&c.accept));
                            if !c.reference.is_empty() {
                                f.note(ui, &c.reference);
                            }
                        });
                    });
                }
            }
        }
        hit
    }

    pub(super) fn rename_sheet(&mut self, ctx: &egui::Context, who: &str) {
        let Some(mut v) = self.shell.desk.rename.clone() else { return };
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let (_, save, back) = Self::sheet_with(ctx, "cand-rename", eravon_ui::metrics::MODAL_W, |ui, keys| {
            eravon_ui::sheet::title(ui, &t(crate::house::i18n::Key::CandsRename), "");
            eravon_ui::fields::stack(ui, |ui, st| {
                st.field(ui, &t(crate::house::i18n::Key::IdLabel), None, None, |ui, w| input::field(ui, &mut v, "", w, input::Look::default()));
            });
            Self::sheet_keys(keys, &t(crate::house::i18n::Key::CfBack), Some((&t(crate::house::i18n::Key::CandsSaveLabel), Role::Primary, true)));
        });
        if save {
            self.shell.desk.rename = None;
            self.step(Step::CandRename, &[("a", who), ("label", &v)]);
        } else if back {
            self.shell.desk.rename = None;
        } else {
            self.shell.desk.rename = Some(v);
        }
    }

    pub(super) fn remove_card(&mut self, ctx: &egui::Context, who: &str) {
        if !self.shell.desk.confirm_remove {
            return;
        }
        let lang = crate::lang::lang();
        let t = |k: crate::house::i18n::Key| crate::house::i18n::get(lang, k).0;
        let label = self.shell.desk.sh.seller.cands.as_ref().and_then(|c| c.rows.as_ref().ok()).and_then(|rows| rows.iter().find(|b| b.row.address.eq_ignore_ascii_case(who)).map(|b| b.row.label.clone())).filter(|l| !l.is_empty()).unwrap_or_else(|| eravon_orders::table::short(who));
        let out = eravon_ui::sheet::show(
            ctx,
            eravon_ui::sheet::Spec::new("cand-remove", eravon_ui::metrics::MODAL_W),
            &mut (),
            |ui, _| {
                eravon_ui::sheet::title(ui, &t(crate::house::i18n::Key::KitRemove), "");
                card::form(ui, |ui, f| f.info(ui, &t(crate::house::i18n::Key::IdLabel), card::Show::Text(&label)));
            },
            |ui, _| {
                let go = page::Pen::new().press(ui, &t(crate::house::i18n::Key::KitRemove), true).clicked();
                let back = key::key(ui, &t(crate::house::i18n::Key::CfBack), Role::Secondary, true).clicked();
                (go, back)
            },
        );
        match out.foot {
            (true, _) => self.step(Step::CandRemove, &[("a", who)]),
            (_, true) => self.shell.desk.confirm_remove = false,
            _ if out.esc => self.shell.desk.confirm_remove = false,
            _ => {}
        }
    }
}

pub(super) fn audit_say(label: &str, lang: crate::lang::Lang) -> Option<String> {
    use crate::house::i18n::Key as K;
    use eravon::tokens::{Label, Verdict};
    let k = match label {
        x if x == Label::Complete.as_str() => return None,
        x if x == Label::Gaps.as_str() => K::CandsAuditGaps,
        x if x == Verdict::Unproven.as_str() => K::CandsAuditUnproven,
        x if x == Label::BrokenChain.as_str() => K::CandsAuditBroken,
        _ => K::CandsAuditNone,
    };
    Some(crate::house::i18n::get(lang, k).0)
}

#[cfg(test)]
mod audit_tests {
    #[test]
    fn an_audit_label_is_said_in_words_and_complete_says_nothing() {
        for lang in [crate::lang::Lang::Zh, crate::lang::Lang::En] {
            assert!(super::audit_say("COMPLETE", lang).is_none());
            let mut seen = Vec::new();
            for l in ["GAPS", "UNPROVEN", "BROKEN_CHAIN", "NO_LABEL"] {
                let w = super::audit_say(l, lang).expect("said");
                assert!(!w.is_empty() && !w.contains('_') && !w.contains(l), "{l} {lang:?}: {w}");
                assert!(!seen.contains(&w), "each says its own: {w}");
                seen.push(w);
            }
        }
    }
}

#[cfg(test)]
mod home_pill_tests {
    use eravon_cands::view::Brief;

    fn brief(address: &str, state: Result<eravon_cands::view::Summary, eravon_cands::fault::Absent>) -> Brief {
        let row = eravon_store::record::Candidate { kind: "judge".into(), address: address.into(), label: "甲".into(), added_at: 0, seen: String::new() };
        Brief { row, state, warn: None, flaw: None }
    }

    #[test]
    fn the_home_column_draws_the_candidates_pages_state_or_not_read() {
        let lang = crate::lang::Lang::Zh;
        let t = |k| crate::house::i18n::get(lang, k).0;
        let (me, a) = ("0x00000000000000000000000000000000000000aa", "0x00000000000000000000000000000000000000b1");
        assert_eq!(super::home_pill(None, me, a, lang).0, t(crate::house::i18n::Key::SetNotRead));
        let page = crate::house::seller::Cands {
            tab: eravon_cands::table::Side::Judge,
            rows: Ok(vec![brief(a, Err(eravon_cands::fault::Absent::Unreachable("连不上".into())))]),
            detail: None,
            dtab: 0,
            said: None,
            me: me.into(),
            codes: Vec::new(),
            lapse: None,
            unread: false,
        };
        let row = page.rows.as_ref().expect("读成")[0].clone();
        assert_eq!(super::home_pill(Some(&page), me, a, lang), super::cand_pill(&row, lang), "与判官页同一个状态");
        assert_ne!(super::home_pill(Some(&page), me, a, lang).0, t(crate::house::i18n::Key::HomeChoosable));
        assert_eq!(super::home_pill(Some(&page), "0x00000000000000000000000000000000000000cc", a, lang).0, t(crate::house::i18n::Key::SetNotRead), "另一把钥读的页不算");
        let listed = crate::house::seller::Cands { unread: true, ..page };
        assert_eq!(super::home_pill(Some(&listed), me, a, lang).0, t(crate::house::i18n::Key::SetNotRead), "盘上的名单还没读链");
    }
}
