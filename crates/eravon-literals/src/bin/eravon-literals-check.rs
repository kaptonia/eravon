#![forbid(unsafe_code)]

use std::process::ExitCode;

use eravon_literals as k7;
use eravon_literals::misuse::{self, Misuse};
use eravon_literals::json::Out;

#[allow(dead_code)]
const EXIT_OK: u8 = 0;
const EXIT_COLLISION: u8 = 1;
const EXIT_MISUSE: u8 = 2;

fn argv() -> Option<Vec<String>> {
    std::env::args_os().skip(1).map(|a| a.into_string().ok()).collect()
}

fn main() -> ExitCode {
    if let Some(a) = argv() {
        misuse::keep_arguments(&a);
    }
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(m) => {
            m.say("eravon-literals-check");
            ExitCode::from(EXIT_MISUSE)
        }
    }
}

mod register {
    use eravon_literals::schema::{Arg, Verb, T_PATH};

    pub const ALL: &[Verb] = &[
        Verb::plain("", &[Arg::need("--galeed", T_PATH), Arg::need("--zikaron", T_PATH)]),
        Verb::plain("schema", &[]),
    ];
}

fn run() -> Result<u8, Misuse> {
    let first = argv().and_then(|a| a.into_iter().next()).unwrap_or_default();
    k7::trace::entry("eravon-literals-check", k7::trace::Feature::K7, &first);
    if argv().map(|a| a == ["schema"]).unwrap_or(false) {
        let mut rows = k7::schema::out_rows(k7::schema::register(
            &k7::schema::version("eravon-literals-check"),
            register::ALL,
            &[],
        ));
        rows.push(("ok", Out::B(true)));
        print(&Out::O(rows));
        return Ok(EXIT_OK);
    }
    let (galeed_path, zikaron_path) = args()?;

    k7::preflight().map_err(|code| Misuse::args(coded(code, "本二进制的自检没过", "this binary's self-check failed")))?;

    let mut families = vec![k7::own_family()];
    families.push(read_family(&galeed_path, "galeed")?);
    families.push(read_family(&zikaron_path, "zikaron")?);

    let collisions = k7::check::distinct(&families);
    let ok = collisions.is_empty();

    print(&Out::O(vec![
        ("v", Out::S(k7::ANSWER_VERSION.to_string())),
        ("ok", Out::B(ok)),
        (
            "families",
            Out::A(
                families
                    .iter()
                    .map(|f| {
                        Out::O(vec![
                            ("name", Out::S(f.name.clone())),
                            ("source", Out::S(f.source.clone())),
                            ("count", Out::N(f.literals.len() as u64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "collisions",
            Out::A(
                collisions
                    .iter()
                    .map(|c| {
                        Out::O(vec![
                            ("literal", Out::S(c.literal.clone())),
                            (
                                "families",
                                Out::A(c.families.iter().map(|n| Out::S(n.clone())).collect()),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "literals",
            Out::O(vec![
                (
                    "documents",
                    Out::A(
                        k7::literals::DOCUMENT_VERSIONS
                            .iter()
                            .map(|d| Out::S((*d).to_string()))
                            .collect(),
                    ),
                ),
                (
                    "standard",
                    Out::O(vec![
                        ("chainId", Out::N(k7::constants::STANDARD_TABLE.chain_id)),
                        (
                            "proxyCodeHash",
                            Out::S(k7::constants::STANDARD_TABLE.proxy_code_hash.to_string()),
                        ),
                    ]),
                ),
            ]),
        ),
        ("company", company()?),
        ("network", network()),
        (
            "constants",
            Out::O(vec![
                (
                    "standard",
                    Out::O(vec![
                        ("board", Out::S(k7::constants::BOARD.to_string())),
                        ("boardCodeHash", Out::S(k7::constants::BOARD_CODE_HASH.to_string())),
                        ("boardInitcodeHash", Out::S(k7::constants::BOARD_INITCODE_HASH.to_string())),
                        ("logic", Out::S(k7::constants::STANDARD_TABLE.logic.to_string())),
                        ("logicInitcodeHash", Out::S(k7::constants::LOGIC_INITCODE_HASH.to_string())),
                        ("panel", Out::S(k7::constants::PANEL.to_string())),
                        ("panelCodeHash", Out::S(k7::constants::PANEL_CODE_HASH.to_string())),
                        ("panelInitcodeHash", Out::S(k7::constants::PANEL_INITCODE_HASH.to_string())),
                        ("proxyCodeHash", Out::S(k7::constants::STANDARD_TABLE.proxy_code_hash.to_string())),
                        ("verifier", Out::S(k7::constants::VERIFIER.to_string())),
                        ("verifierCodeHash", Out::S(k7::constants::VERIFIER_CODE_HASH.to_string())),
                        ("verifierInitcodeHash", Out::S(k7::constants::VERIFIER_INITCODE_HASH.to_string())),
                    ]),
                ),
                (
                "unfilled",
                Out::A(
                    k7::constants::unfilled_slots()
                        .iter()
                        .map(|s| {
                            Out::O(vec![
                                ("name", Out::S(s.name.to_string())),
                                ("bornIn", Out::S(s.born_in.to_string())),
                                ("why", Out::S(s.why.to_string())),
                            ])
                        })
                        .collect(),
                ),
                ),
                ("units", Out::O(vec![("nativeDecimals", Out::N(k7::constants::NATIVE_DECIMALS as u64))])),
            ]),
        ),
    ]));

    Ok(if ok { EXIT_OK } else { EXIT_COLLISION })
}

fn network() -> Out {
    let n = k7::constants::default_network();
    let list = |v: &[&str]| Out::A(v.iter().map(|x| Out::S((*x).to_string())).collect());
    Out::O(vec![
        ("chainId", Out::N(n.standard.chain_id)),
        ("factory", list(n.factory)),
        ("firstBlock", Out::N(n.standard.first_block)),
        ("name", Out::S(n.name.to_string())),
        ("registries", list(&n.registries.get().copied().unwrap_or(&[]).iter().map(|r| r.address).collect::<Vec<_>>())),
        ("sample", list(k7::constants::endpoint_table_factory_sample().get().copied().unwrap_or(&[]))),
    ])
}

fn company() -> Result<Out, Misuse> {
    use k7::constants::{abi_signatures, CHELEK_ABI, CHELEK_IDENTITY};
    let sigs = |abi: &str, sorted: bool| -> Result<Out, Misuse> {
        let mut rows = abi_signatures(abi).map_err(|_| Misuse::args(misuse::said("CHELEK 的 ABI 读不成签名", "the CHELEK ABI does not read as signatures")))?;
        if sorted {
            rows.sort();
        }
        Ok(Out::A(rows.into_iter().map(Out::S).collect()))
    };
    let id = CHELEK_IDENTITY;
    Ok(Out::O(vec![
        (
            "abi",
            Out::O(vec![
                ("ledger", sigs(CHELEK_ABI.ledger, true)?),
                ("views", sigs(CHELEK_ABI.company_views, false)?),
                ("wallet", sigs(CHELEK_ABI.wallet, true)?),
            ]),
        ),
        (
            "defaults",
            Out::O(vec![
                ("companyMode", Out::B(k7::constants::COMPANY_MODE_DEFAULT_ON)),
                ("timeCheck", Out::B(k7::constants::TIME_CHECK_DEFAULT_ON)),
            ]),
        ),
        (
            "identity",
            Out::O(vec![
                ("ledgerCodeHash", Out::S(id.ledger_code_hash.to_string())),
                ("proxyCodeHash", Out::S(id.proxy_code_hash.to_string())),
                ("walletCodeHash", Out::S(id.wallet_code_hash.to_string())),
            ]),
        ),
        ("sourceCommit", Out::S(k7::constants::CHELEK_SOURCE_COMMIT.to_string())),
    ]))
}

fn args() -> Result<(String, String), Misuse> {
    let mut galeed: Option<String> = None;
    let mut zikaron: Option<String> = None;
    let mut it = argv()
        .ok_or_else(Misuse::not_text)?
        .into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--galeed" => {
                if galeed.is_some() {
                    return Err(Misuse::repeated("--galeed"));
                }
                galeed = Some(it.next().ok_or_else(|| Misuse::no_value("--galeed"))?)
            }
            "--zikaron" => {
                if zikaron.is_some() {
                    return Err(Misuse::repeated("--zikaron"));
                }
                zikaron = Some(it.next().ok_or_else(|| Misuse::no_value("--zikaron"))?)
            }
            other => return Err(Misuse::unknown_arg("", other)),
        }
    }
    let galeed = galeed.ok_or_else(|| Misuse::missing("--galeed"))?;
    let zikaron = zikaron.ok_or_else(|| Misuse::missing("--zikaron"))?;
    Ok((galeed, zikaron))
}

fn coded(code: &'static str, zh: &str, en: &str) -> misuse::Said {
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    misuse::said(leak(format!("{zh}({code})")), leak(format!("{en} ({code})")))
}

const FILE_BOUND: u64 = 16 * 1024 * 1024;

fn read_family(path: &str, family: &'static str) -> Result<k7::foreign::Family, Misuse> {
    let _ = family;
    let meta = std::fs::metadata(path).map_err(|_| Misuse::unreadable(path, misuse::UNREADABLE))?;
    if !meta.is_file() || meta.len() > FILE_BOUND {
        return Err(Misuse::unreadable(path, misuse::TOO_BIG));
    }
    let bytes = std::fs::read(path).map_err(|_| Misuse::unreadable(path, misuse::UNREADABLE))?;
    let text = String::from_utf8(bytes).map_err(|_| Misuse::unreadable(path, misuse::said("不是 UTF-8 文本", "not UTF-8 text")))?;
    k7::foreign::literals_of(&text, family, path).map_err(|code| Misuse::unreadable(path, coded(code, "这份登记簿里取不到这一家的字面", "this registry yields no literals of that family")))
}

fn print(v: &Out) {
    use std::io::Write;
    let bytes = k7::json::write(v);
    let out = std::io::stdout();
    let mut h = out.lock();
    if h.write_all(&bytes).is_err() || h.flush().is_err() {
        std::process::exit(EXIT_MISUSE as i32);
    }
}
