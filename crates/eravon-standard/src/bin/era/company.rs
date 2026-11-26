use eravon_literals::misuse::Misuse;
use super::*;

pub(super) fn shut_out(s: &a1::company::Shut) -> Out {
    Out::O(vec![
        ("at", Out::S(s.at.to_string())),
        ("kind", Out::S(s.kind.word().to_string())),
        ("why", Out::S(s.why.clone())),
    ])
}

pub(super) fn opened_rows(v: &a1::company::Views, c: &[a1::company::Checked; 3]) -> Vec<(&'static str, Out)> {
    vec![
        (
            "identity",
            Out::A(
                c.iter()
                    .map(|x| {
                        Out::O(vec![
                            ("address", Out::S(x.address.clone())),
                            ("codeHash", Out::S(x.code_hash.clone())),
                            ("site", Out::S(x.site.word().to_string())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "views",
            Out::O(vec![
                ("agreement", Out::S(v.agreement.to_string())),
                ("calendarBound", Out::N(v.calendar_bound)),
                ("endpoints", Out::N(v.endpoints as u64)),
                ("ledger", Out::S(v.ledger.clone())),
                ("wallet", Out::S(v.wallet.clone())),
            ]),
        ),
    ]
}

pub(super) fn company(f: &Flags) -> Result<Out, Misuse> {
    let t = table(f)?;
    let co = need(f, "--company")?;
    let (chain, _) = chain_and_logic(f)?;
    ans((|| {
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        Ok(match a1::company::open(&w, &t, &co)? {
            Ok((v, c)) => {
                let mut m = opened_rows(&v, &c);
                m.push(("open", Out::B(true)));
                ok(m)
            }
            Err(s) => ok(vec![("open", Out::B(false)), ("shut", shut_out(&s))]),
        })
    })())
}

pub(super) fn time_check(f: &Flags) -> Result<Out, Misuse> {
    use a1::company::Item;
    let t = table(f)?;
    let co = need(f, "--company")?;
    let now = num(f, "--now")?;
    let (chain, _) = chain_and_logic(f)?;
    let mut items = Vec::new();
    let windows = [("--t1", "t1"), ("--resign-lead", "resignLead"), ("--t2", "t2")];
    for (k, name) in windows {
        if f.one(k).is_some() {
            items.push(Item::Window { name, who: String::new(), secs: num(f, k)? });
        }
    }
    for c in f.many("--t3") {
        let (who, secs) = c.split_once('=').ok_or_else(|| Misuse::bad("--t3", c, eravon_literals::misuse::said("要写成 谁=秒数", "who=seconds")))?;
        items.push(Item::Window { name: "t3", who: who.to_string(), secs: secs.parse().map_err(|_| Misuse::bad("--t3", c, eravon_literals::misuse::said("要写成 谁=秒数,秒数是十进制整数", "who=seconds, the seconds a decimal integer")))? });
    }
    let deadlines = [("--offer-expiry", "offerExpiry"), ("--settle-expiry", "settleExpiry"), ("--evidence-t3", "evidenceT3")];
    for (k, name) in deadlines {
        if f.one(k).is_some() {
            items.push(Item::Deadline { name, who: String::new(), at: num(f, k)? });
        }
    }
    ans((|| {
        if items.is_empty() {
            return Err(Fault::Form("时间核对至少要一项期限".into()));
        }
        let t = t?;
        let w = wire(f)?;
        a2::read::chain_guard(&w, &t, chain)?;
        let (v, c) = match a1::company::open(&w, &t, &co)? {
            Ok(x) => x,
            Err(s) => return Ok(ok(vec![("open", Out::B(false)), ("shut", shut_out(&s))])),
        };
        let verdict = a1::company::time_check(v.calendar_bound, now, &items);
        let rows = verdict
            .rows
            .iter()
            .map(|r| {
                Out::O(vec![
                    ("available", Out::N(r.available)),
                    ("item", Out::S(r.name.to_string())),
                    ("short", Out::N(r.short)),
                    ("who", Out::S(r.who.clone())),
                ])
            })
            .collect();
        let mut m = opened_rows(&v, &c);
        m.push(("open", Out::B(true)));
        m.push((
            "time",
            Out::O(vec![
                ("bound", Out::N(verdict.bound)),
                ("enough", Out::B(verdict.enough)),
                ("now", Out::N(verdict.now)),
                ("rows", Out::A(rows)),
            ]),
        ));
        Ok(ok(m))
    })())
}

pub(super) fn pcall_out(c: &a1::proposal::PCall) -> Out {
    Out::O(vec![
        ("data", Out::S(order::hex_of(&c.data))),
        ("to", Out::S(order::hex_of(&c.to))),
        ("value", Out::S(k2::u256::U256::from_be(&c.value).to_dec())),
    ])
}

pub(super) fn original_out(o: &a1::proposal::Original) -> Out {
    Out::O(vec![
        ("bytes", Out::S(order::hex_of(&o.bytes))),
        ("kind", Out::S(o.kind.clone())),
        ("name", Out::S(o.name.clone())),
    ])
}

pub(super) fn proposal_read(f: &Flags) -> Result<Out, Misuse> {
    let b = read_bounded(&need(f, "--file")?)?;
    ans(a1::proposal::read(&b).map(|p| {
        ok(vec![
            ("calls", Out::N(p.calls.len() as u64)),
            ("contribution", Out::B(p.contribution())),
            ("digest", Out::S(order::hex_of(&p.digest()))),
            ("documents", Out::N(p.documents.len() as u64)),
            ("structural", Out::B(p.structural())),
        ])
    }))
}

pub(super) fn proposal_pay(f: &Flags) -> Result<Out, Misuse> {
    let given = match amount_arg(f, "--amount", "--amount-text", || chain_decimals(f, f.one("--token")))? {
        Ok(v) => v,
        Err(e) => return ans(Err(e)),
    };
    let amount = match word("--amount", &given)? {
        Ok(w) => w,
        Err(e) => return ans(Err(e)),
    };
    ans(a1::proposal::wallet_pay(&need(f, "--wallet")?, &need(f, "--token")?, &need(f, "--to")?, &amount)
        .map(|(c, d)| ok(vec![("call", pcall_out(&c)), ("documents", Out::A(vec![original_out(&d)]))])))
}

pub(super) fn proposal_consent(f: &Flags) -> Result<Out, Misuse> {
    let path = need(f, "--original")?;
    let bytes = read_bounded(&path)?;
    let name = f.one("--name").map(str::to_string).unwrap_or_else(|| eravon_literals::seam::file_name_of(&path));
    let original = a1::proposal::Original { kind: need(f, "--kind")?, name, bytes };
    let digest = order::b32_of("--digest", &need(f, "--digest")?).map_err(|_| Misuse::bad("--digest", f.one("--digest").unwrap_or_default(), eravon_literals::misuse::HEX32))?;
    ans(a1::proposal::consent(&need(f, "--logic")?, &need(f, "--party")?, &digest, original)
        .map(|(c, ds)| ok(vec![("call", pcall_out(&c)), ("documents", Out::A(ds.iter().map(original_out).collect()))])))
}
