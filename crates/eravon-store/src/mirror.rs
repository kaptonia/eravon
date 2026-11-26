use eravon_literals::json::{In, Out};

use crate::atomic;
use crate::doc;
use crate::fault::{Fault, R};
use crate::writer::{self, Words};
use crate::seam;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mirror {
    pub cut_at: u64,
    pub from: String,
}

pub fn from_of(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}


pub fn mark(data: &str, cut_at: u64, from: &str) -> R<()> {
    let bytes = doc::seal(
        seam::V_MIRROR,
        vec![("cutAt", Out::N(cut_at)), ("from", Out::S(from.to_string()))],
    );
    atomic::write_raw(&seam::mirror_path(data), &bytes)
}

pub fn of(data: &str) -> R<Option<Mirror>> {
    let path = seam::mirror_path(data);
    let bytes = match atomic::read_file(&path) {
        Ok(b) => b,
        Err(Fault::Missing(_)) => return Ok(None),
        Err(e) => return Err(e),
    };
    let v: In = doc::open(seam::V_MIRROR, &bytes)?;
    Ok(Some(Mirror {
        cut_at: doc::need_u64(&v, "cutAt")?,
        from: doc::need_str(&v, "from")?,
    }))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stand {
    pub writable: bool,
    pub holder: Option<Words>,
    pub other: Option<String>,
    pub mark_unread: Option<String>,
    pub mirror: Option<Mirror>,
}

impl Stand {
    pub fn stale(&self) -> bool {
        self.mirror.is_some() || self.holder.is_some() || self.other.is_some() || self.mark_unread.is_some()
    }

    pub fn says(&self) -> String {
        match (&self.mirror, &self.holder, &self.other) {
            (Some(m), _, _) => format!(
                "这一家是镜像:{} 在 {} 切的束铺下来的;此后写者那一侧的改动不会到这里",
                m.from, m.cut_at
            ),
            (None, Some(h), _) => {
                format!("{}(进程 {})正握着这一家的笔,此刻只读得,写不得", h.writer, h.pid)
            }
            (None, None, Some(o)) => format!("这一家由另一台机器在写(那台机器的标 {o}),此刻只读得;改由本机写才写得"),
            (None, None, None) => match &self.mark_unread {
                Some(why) => format!("这一家的写者标读不懂({why}),此刻只读得、标原样留着;改由本机写才写得"),
                None => String::new(),
            },
        }
    }

    pub fn rows(&self) -> Vec<(&'static str, Out)> {
        let mut m = vec![
            ("stale", Out::B(self.stale())),
            ("writable", Out::B(self.writable)),
        ];
        if let Some(h) = &self.holder {
            m.push(("heldBy", Out::S(h.writer.clone())));
            m.push(("heldPid", Out::N(u64::from(h.pid))));
        }
        if let Some(o) = &self.other {
            m.push(("otherMachine", Out::S(o.clone())));
        }
        if let Some(w) = &self.mark_unread {
            m.push(("markUnread", Out::S(w.clone())));
        }
        if let Some(x) = &self.mirror {
            m.push(("mirrorCutAt", Out::N(x.cut_at)));
            m.push(("mirrorFrom", Out::S(x.from.clone())));
        }
        if self.stale() {
            m.push(("staleSays", Out::S(self.says())));
        }
        m.sort_by(|a, b| a.0.cmp(b.0));
        m
    }
}

pub fn stand(data: &str) -> R<Stand> {
    let mirror = of(data)?;
    let holder = writer::probe(data)?;
    let other = writer::other_machine(data)?;
    let mark_unread = writer::mark_trouble(data).map(|f| f.detail());
    Ok(Stand { writable: mirror.is_none() && holder.is_none() && other.is_none() && mark_unread.is_none(), holder, other, mark_unread, mirror })
}


pub fn gate(path: &str) -> R<()> {
    for d in std::path::Path::new(path).ancestors() {
        let d = d.to_string_lossy();
        if d.is_empty() {
            continue;
        }
        match of(&d) {
            Ok(None) => continue,
            Ok(Some(_)) => return need_writable(&d),
            Err(Fault::Version { .. } | Fault::Form(_)) => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

pub fn need_writable(data: &str) -> R<()> {
    match of(data)? {
        None => Ok(()),
        Some(m) => Err(Fault::Mirror(format!(
            "这一家是镜像({} 在 {} 切的束铺下来的),只读;要写请回原机",
            m.from, m.cut_at
        ))),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn home(tag: &str) -> String {
        let d = std::env::temp_dir().join(format!("eravon-mirror-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("建得出临时家");
        d.display().to_string()
    }

    #[test]
    fn a_plain_home_is_writable_and_says_nothing() {
        let d = home("plain");
        let s = stand(&d).unwrap();
        assert!(s.writable);
        assert!(!s.stale());
        assert_eq!(s.says(), "");
    }

    #[test]
    fn a_mirror_is_read_only_and_says_when_it_was_cut() {
        let d = home("mirror");
        mark(&d, 900, "原机").unwrap();
        let s = stand(&d).unwrap();
        assert!(!s.writable, "副机只读");
        assert!(s.stale());
        assert!(s.says().contains("镜像"));
        assert!(s.says().contains("900"), "陈旧到哪一刻说得出来:{}", s.says());
        assert_eq!(of(&d).unwrap().unwrap().from, "原机");
    }

    #[test]
    fn every_write_under_a_mirror_is_refused_at_the_atomic_layer() {
        let d = home("gate");
        mark(&d, 900, "原机").unwrap();
        let r = atomic::write_atomic(&format!("{d}/trace/shell-trace.json"), b"{}");
        assert!(matches!(r, Err(Fault::Mirror(_))), "{r:?}");
        assert!(!atomic::exists(&format!("{d}/trace")), "目录也没造");
        assert!(matches!(atomic::mkdirs(&d), Err(Fault::Mirror(_))));
        assert!(!atomic::exists(&seam::lock_path(&d)), "锁档一个字节也没造");
    }

    #[test]
    fn a_same_named_file_that_is_not_ours_does_not_make_a_mirror() {
        let d = home("foreign");
        std::fs::write(seam::mirror_path(&d), b"something else").unwrap();
        atomic::write_atomic(&format!("{d}/x/y.json"), b"{}").unwrap();
    }

    #[test]
    fn a_home_someone_else_is_writing_reads_but_does_not_write() {
        let d = home("held");
        let lock_file = seam::lock_path(&d);
        std::fs::write(&lock_file, doc::seal(seam::V_LOCK, vec![("pid", Out::N(7)), ("since", Out::N(900)), ("writer", Out::S("第一个写者".into()))])).unwrap();
        let other = std::fs::OpenOptions::new().read(true).write(true).open(&lock_file).unwrap();
        assert!(eravon_os::lock_now(&other), "the other instance holds it");
        let s = stand(&d).unwrap();
        assert!(!s.writable);
        assert!(s.stale());
        assert!(s.says().contains("第一个写者"), "说得出是谁:{}", s.says());
        drop(other);
        let s = stand(&d).unwrap();
        assert!(s.writable, "放开即又写得了:{s:?}");
    }
}
