use super::*;

impl Settings {
    pub fn rows(&self) -> Out {
        let s = |x: &str| Out::S(x.to_string());
        let mut m = vec![
            ("endpoints", Out::A(self.endpoints.iter().map(|x| s(x)).collect())),
            ("rest", Out::A(self.rest.iter().map(|(b, k)| Out::O(vec![("base", s(b)), ("keyHere", Out::B(*k))])).collect())),
            ("restOn", Out::B(self.rest_on)),
            ("language", s(&self.language)),
            ("refreshEvery", Out::N(self.refresh_every)),
            ("way", s(&self.way)),
            ("wayUnread", s(&self.way_unread)),
            ("zone", s(&self.zone)),
            ("zoneShown", s(if eravon_orders::when::shows_utc() { "utc" } else { "system" })),
            ("look", s(&self.look)),
            ("me", s(&self.me)),
            ("network", s(eravon_literals::constants::current_network().name)),
            ("networks", Out::A(eravon_literals::constants::NETWORKS.iter().map(|n| Out::O(vec![("chainId", Out::N(n.standard.chain_id)), ("name", s(n.name))])).collect())),
            ("pastCases", Out::B(eravon_literals::constants::past_switch())),
            ("seat", s(self.seat.name())),
            (
                "pinned",
                {
                    let p = eravon_literals::constants::active_standard();
                    Out::O(vec![
                        ("chainId", Out::N(p.chain_id)),
                        ("domainVersion", s(eravon_literals::literals::DOMAIN_VERSION)),
                        ("logic", s(p.logic)),
                        ("proxyCodeHash", s(p.proxy_code_hash)),
                    ])
                },
            ),
            (
                "verbs",
                Out::A(verbs(self.seat).into_iter().map(|v| s(v)).collect()),
            ),
            ("zikaronDir", s(&self.zk.0)),
            ("zikaronLink", Out::B(self.zk_link)),
            (
                "rotate",
                match &self.rotate {
                    None => Out::Null,
                    Some(Ok(v)) => v.out(),
                    Some(Err(e)) => Out::O(vec![("detail", s(&e.detail)), ("name", s(&e.name))]),
                },
            ),
            ("autoLock", Out::B(self.auto_lock)),
            ("cliPath", s(self.cli_path.as_ref().map_or("", |c| cli_word(c).0))),
            ("cliPathSays", s(&self.cli_path.as_ref().map_or(String::new(), |c| cli_word(c).1))),
            ("cliAnchor", s(&self.cli_anchor.0)),
            ("cliAnchorLive", Out::B(self.cli_anchor.1)),
            ("idleMinutes", Out::N(self.idle_minutes)),
            ("lastBackupAt", Out::N(self.last_backup.as_ref().map_or(0, |x| x.0))),
            ("lastBackupTo", s(self.last_backup.as_ref().map_or("", |x| x.1.as_str()))),
            ("lastBackupFailedAt", Out::N(self.last_backup_failed.unwrap_or(0))),
            ("changedSinceBackup", Out::S(self.changed.map_or(String::new(), |n| n.to_string()))),
            ("section", s(self.section.map_or("home", Section::name))),
            (
                "sections",
                Out::A(Section::ALL.into_iter().filter(|x| x.in_seat(self.seat)).map(|x| s(x.name())).collect()),
            ),
            ("zikaron", zk_out(&self.zk.1)),
            ("company", self.company_out()),
            ("tokens", self.tokens_out()),
            ("accountLabel", s(company_label(&self.company).unwrap_or_default())),
        ];
        m.push((
            "keys",
            match &self.keys {
                Err(e) => e.out(),
                Ok(list) => Out::A(
                    list.iter()
                        .map(|k| {
                            Out::O(vec![
                                ("address", s(&k.address)),
                                ("current", Out::B(k.current)),
                                ("kind", s(&k.kind)),
                                ("label", s(&k.label)),
                                ("primary", Out::B(k.primary)),
                            ])
                        })
                        .collect(),
                ),
            },
        ));
        if let Some(g) = &self.gas {
            m.push((
                "gas",
                match g {
                    Ok((wei, a, n)) => Out::O(vec![("agreement", s(a)), ("endpoints", Out::N(*n as u64)), ("eth", s(&eravon_vault::tokens::figure(wei, eravon_literals::constants::NATIVE_DECIMALS, eravon_vault::tokens::Form::Plain))), ("wei", s(wei))]),
                    Err(e) => e.out(),
                },
            ));
        }
        if let Some((wei, a, at)) = &self.gas_book {
            m.push(("gasBook", Out::O(vec![("agreement", s(a)), ("at", Out::N(*at)), ("wei", s(wei))])));
        }
        if let Some(x) = &self.said {
            m.push(("said", x.out()));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        Out::O(m)
    }
}
