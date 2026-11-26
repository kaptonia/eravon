use std::path::Path;

use eravon_chain::endpoints::Table;
use eravon_chain::transport::Transport;
use eravon_dossier as k4;
use eravon_galeed as k1;
use eravon_recheck::check::{self, Opts};
use eravon_recheck::dossier::Opened;
use eravon_recheck::report::Item;

use crate::assemble::Assembled;
use crate::seam;
use crate::{Why, why};

pub const E_UNWRITABLE: &str = "E_S7_UNWRITABLE";
pub const E_SELFCHECK: &str = "E_S7_SELFCHECK";

#[derive(Clone, Debug)]
pub struct Done {
    pub dir: String,
    pub report: Result<Vec<Item>, Why>,
    pub selfcheck: Vec<u8>,
}

pub fn export<T: Transport>(t: &T, table: &Table, a: &Assembled, dest: &str, name: &str, opts: &Opts) -> Result<Done, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let stage = seam::staging(dest, name);
    writable(dest, &stage)?;
    let fail = |e: Why| {
        let _ = std::fs::remove_dir_all(&stage);
        e
    };
    for (n, b) in &a.files {
        std::fs::write(seam::join(&stage, n), b).map_err(|e| fail(why(E_UNWRITABLE, format!("写不进 {dest}:{e}"))))?;
    }
    std::fs::write(seam::join(&stage, eravon_literals::literals::DOSSIER_INDEX_FILE), &a.index)
        .map_err(|e| fail(why(E_UNWRITABLE, format!("写不进 {dest}:{e}"))))?;

    let report = recheck(t, table, &stage, opts).map_err(&fail)?;
    let selfcheck = selfcheck(Path::new(&stage)).map_err(&fail)?;

    let mut target = seam::join(dest, name);
    let mut n = 2u32;
    while Path::new(&target).exists() {
        target = seam::join(dest, &format!("{name}-{n}"));
        n += 1;
    }
    eravon_os::replace(Path::new(&stage), Path::new(&target)).map_err(|e| fail(why(E_UNWRITABLE, format!("改不了名 {target}:{e}"))))?;
    Ok(Done { dir: target, report, selfcheck })
}

fn writable(dest: &str, stage: &str) -> Result<(), Why> {
    if !Path::new(dest).is_dir() {
        return Err(why(E_UNWRITABLE, format!("{dest} 不是一个目录")));
    }
    let _ = std::fs::remove_dir_all(stage);
    std::fs::create_dir(stage).map_err(|e| why(E_UNWRITABLE, format!("{dest} 写不进:{e}")))
}


fn recheck<T: Transport>(t: &T, table: &Table, stage: &str, opts: &Opts) -> Result<Result<Vec<Item>, Why>, Why> {
    let unopened = |why: String| Ok(Ok(vec![Item::unread(eravon_recheck::report::Name::Code, L_UNOPENED, "卷宗打不开,链比对一项也没重算".to_string(), why)]));
    match eravon_recheck::dossier::open(Path::new(stage)) {
        Ok(Opened::Whole(c)) => Ok(check::run(t, table, &c, opts).map_err(|e| why(&e.name(), e.detail()))),
        Ok(Opened::Refused(k)) => unopened(String::from_utf8_lossy(&k).into_owned()),
        Err(e) => unopened(format!("{}:{}", e.name(), e.detail())),
    }
}

const L_UNOPENED: &str = "dossier/1 §3 卷宗形制与绑定";


fn selfcheck(dir: &Path) -> Result<Vec<u8>, Why> {
    let refuse = |v: k1::Value| why(E_SELFCHECK, String::from_utf8_lossy(&eravon_galeed::schema::answer_bytes(&v)).into_owned());
    let b = match k4::files::read_index_bytes(dir) {
        Ok(b) => b,
        Err(k4::files::Stop::Fault(f)) => return Err(refuse(f.to_value(true))),
    };
    let idx = k4::index::index(&b).map_err(|f| refuse(f.to_value(true)))?;
    match k4::files::files(dir, &idx) {
        Ok(w) => Ok(eravon_literals::json::write(&eravon_literals::json::Out::O(vec![
            ("buyerConsent", eravon_literals::json::Out::S(w.buyer_consent.name().to_string())),
            ("candidate", eravon_literals::json::Out::N(w.candidate)),
            ("caseId", eravon_literals::json::Out::S(w.case_id.clone())),
            ("entryId", eravon_literals::json::Out::S(w.entry_id.clone().unwrap_or_default())),
            ("instance", eravon_literals::json::Out::S(w.instance.clone())),
            ("ok", eravon_literals::json::Out::B(true)),
            ("sellerConsent", eravon_literals::json::Out::S(w.seller_consent.name().to_string())),
            ("termsHash", eravon_literals::json::Out::S(w.terms_hash.clone())),
        ]))),
        Err(k4::files::Stop::Fault(f)) => Err(refuse(f.to_value(true))),
    }
}

pub fn opts_for(registries: &[String]) -> Opts {
    Opts { registries: eravon_cands::wire::registries(registries).unwrap_or_default(), events_from: 0 }
}
