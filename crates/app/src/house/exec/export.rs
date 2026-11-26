use crate::house::act::{Act, Said};
use crate::house::action::Action as Step;
use crate::house::desk::Site;
use crate::house::shell::Toast;
use crate::shell::Shell;

pub(super) fn run(sh: &mut Shell, a: &Act) -> Option<Said> {
    Some(match a.step {
        Step::ExTab | Step::ExPicker | Step::ExSearch | Step::ExPick | Step::ExDir => local(sh, a),
        Step::ExReload => {
            if sh.desk.sh.export.is_none() {
                return Some(Err("先打开导出页".to_string()));
            }
            sh.load_export();
            sh.desk.tr.push(Step::ExReload, eravon_export::mark::FEATURE, String::new());
            Ok("ex-reload".into())
        }
        Step::ExRun => export_run(sh, a),
        _ => return None,
    })
}

fn local(sh: &mut Shell, a: &Act) -> Said {
    let (data, now) = (sh.desk.data.clone(), sh.desk.now());
    let ex = sh.desk.sh.export.as_mut().ok_or_else(|| "先打开导出页".to_string())?;
    ex.said = None;
    let say = match a.step {
        Step::ExTab => {
            let v = a.arg("v")?;
            ex.tab = crate::house::export::Tab::of(&v).ok_or_else(|| "页签只认 dossier 与 statement".to_string())?;
            ex.done = None;
            v
        }
        Step::ExPicker => {
            ex.picker = a.opt("open") != Some("false");
            if ex.picker { "open" } else { "false" }.to_string()
        }
        Step::ExSearch => {
            (ex.q, ex.from, ex.to) = (a.opt("q").unwrap_or_default().to_string(), a.opt("from").unwrap_or_default().to_string(), a.opt("to").unwrap_or_default().to_string());
            ex.search();
            format!("{} · {} · {}", ex.q, ex.from, ex.to)
        }
        Step::ExPick => {
            let i = a.arg("instance")?;
            if let Err(e) = ex.choose(&i) {
                ex.said = Some(e);
            }
            i
        }
        _ => {
            let d = a.arg("dir")?;
            if let Err(e) = ex.set_dir(&data, now, &d) {
                ex.said = Some(e);
            }
            d
        }
    };
    let line = ex.said.as_ref().map_or_else(|| say.clone(), |s| format!("{say} · {} · {}", s.name, s.detail));
    if let Some(x) = ex.said.clone().filter(|_| matches!(a.step, Step::ExPick | Step::ExDir)) {
        sh.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: x.detail.clone(), next: String::new(), detail: Some(x.name.clone()) });
    }
    sh.desk.tr.push(a.step, eravon_export::mark::FEATURE, say);
    Ok(line)
}

fn export_run(sh: &mut Shell, a: &Act) -> Said {
    let Some(e) = sh.desk.sh.export.clone() else { return Err("先打开导出页".to_string()) };
    let job = e.plan(&sh.desk.data, &sh.desk.sh.seller.field.registries, sh.desk.now());
    let asked = match &job {
        Ok(_) => crate::house::asked(Ok(())),
        Err(x) => crate::house::asked(Err(x.clone())),
    };
    let job = match (asked, job) {
        (Ok(()), Ok(j)) => j,
        (Err(x), _) | (Ok(()), Err(x)) => {
            let line = format!("{} · {}", x.name, x.detail);
            sh.desk.sh.toast(Toast { tone: crate::house::seller::Tone::Bad, head: x.detail.clone(), next: String::new(), detail: Some(x.name.clone()) });
            if let Some(ex) = sh.desk.sh.export.as_mut() {
                ex.said = Some(x);
            }
            return Ok(line);
        }
    };
    sh.desk.retry_on(crate::task::Kind::Export, a.clone());
    sh.spawn_export_at(Site::Page, true, move |_s, _data, _me, mut cur, t| {
        let t = *t;
        cur.done = Some(crate::house::export::run(&job, &move || t.live()));
        cur
    });
    sh.desk.tr.push(Step::ExRun, eravon_export::mark::FEATURE, String::new());
    Ok("导出".into())
}
