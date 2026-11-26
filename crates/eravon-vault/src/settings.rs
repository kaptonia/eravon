use crate::fault::{classify, Fault, Known};
use crate::home::{Home, Slot};
use crate::roles::Role;
use eravon::json::{self, Value};

pub const FILE: &str = eravon_seal::ground::SETTINGS_FILE;

fn turn() -> std::sync::MutexGuard<'static, ()> {
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());
    TURN.lock().unwrap_or_else(|e| e.into_inner())
}

pub const CAP_DEFAULT: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Settings {
    pub role: Role,
    pub cap_bytes: u64,
    pub endpoints: Vec<String>,
    pub rest: Vec<String>,
    pub rest_on: bool,
    pub log_asks: Option<u64>,
    pub chain_id: Option<u64>,
    pub lang: Option<crate::lang::Lang>,
    pub zone: Option<crate::when::Zone>,
    pub extra: Vec<(String, Value)>,
    pub network: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            role: Role::Author,
            cap_bytes: CAP_DEFAULT,
            endpoints: Vec::new(),
            rest: Vec::new(),
            rest_on: false,
            log_asks: None,
            chain_id: None,
            lang: None,
            zone: None,
            network: None,
            extra: Vec::new(),
        }
    }
}

const MEMBERS: [&str; 10] = ["capBytes", "chainId", "endpoints", "lang", "network", eravon_seal::ground::REST, eravon_seal::ground::REST_ON, eravon_seal::ground::LOG_ASKS, "role", "zone"];

fn field<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    match v {
        Value::Obj(m) => m.iter().find(|(name, _)| name == k).map(|(_, x)| x),
        _ => None,
    }
}

impl Settings {
    pub fn read(home: &Home) -> Result<Settings, Fault> {
        let p = home.dir(Slot::Settings).join(FILE);
        let Some(bytes) = crate::local::read(&p, crate::local::Doc::Settings)? else {
            return Ok(Settings::default());
        };
        let v = json::parse(&bytes)
            .map_err(|t| Fault::known(Known::SettingsShape, format!("{}: {t:?}", p.display())))?;
        let role = match field(&v, "role") {
            Some(Value::Str(s)) if s == Role::Grantee.as_str() => Role::Grantee,
            _ => Role::Author,
        };
        let cap = match field(&v, "capBytes") {
            Some(Value::Int(n)) if *n > 0 => *n,
            _ => CAP_DEFAULT,
        };
        let ground = eravon_seal::ground::of_settings(&v);
        Ok(Settings {
            role,
            cap_bytes: cap,
            endpoints: ground.endpoints,
            rest: ground.rest,
            rest_on: ground.rest_on,
            log_asks: ground.log_asks.filter(|n| eravon_chain::action::asks_within(*n)),
            chain_id: ground.chain_id,
            lang: match field(&v, "lang") {
                Some(Value::Str(x)) => crate::lang::Lang::ALL.iter().copied().find(|l| l.as_str() == x),
                _ => None,
            },
            zone: match field(&v, "zone") {
                Some(Value::Str(x)) => crate::when::Zone::parse(x),
                _ => None,
            },
            network: match field(&v, "network") {
                Some(Value::Str(x)) if crate::deploy::named(x).is_some() => Some(x.clone()),
                _ => None,
            },
            extra: match &v {
                Value::Obj(ms) => ms.iter().filter(|(k, _)| !MEMBERS.contains(&k.as_str())).cloned().collect(),
                _ => Vec::new(),
            },
        })
    }

    pub fn write(&self, home: &Home) -> Result<(), Fault> {
        let _turn = turn();
        self.write_now(home)?;
        crate::reads::wrote(crate::reads::Copies::Settings);
        Ok(())
    }

    pub fn amend(home: &Home, f: impl FnOnce(&mut Settings)) -> Result<Settings, Fault> {
        let _turn = turn();
        let mut s = Settings::read(home)?;
        f(&mut s);
        s.write_now(home)?;
        crate::reads::wrote(crate::reads::Copies::Settings);
        Ok(s)
    }

    fn write_now(&self, home: &Home) -> Result<(), Fault> {
        let mut m = vec![
            ("capBytes".to_string(), Value::Int(self.cap_bytes)),
            ("role".to_string(), Value::Str(self.role.as_str().to_string())),
        ];
        if let Some(c) = self.chain_id {
            m.push(("chainId".to_string(), Value::Int(c)));
        }
        if let Some(n) = &self.network {
            m.push(("network".to_string(), Value::Str(n.clone())));
        }
        if !self.endpoints.is_empty() {
            m.push((
                "endpoints".to_string(),
                Value::Arr(self.endpoints.iter().map(|x| Value::Str(x.clone())).collect()),
            ));
        }
        if !self.rest.is_empty() {
            m.push((eravon_seal::ground::REST.to_string(), Value::Arr(self.rest.iter().map(|x| Value::Str(x.clone())).collect())));
        }
        if self.rest_on {
            m.push((eravon_seal::ground::REST_ON.to_string(), Value::Bool(true)));
        }
        if let Some(n) = self.log_asks {
            m.push((eravon_seal::ground::LOG_ASKS.to_string(), Value::Int(n)));
        }
        if let Some(l) = self.lang {
            m.push(("lang".to_string(), Value::Str(l.as_str().to_string())));
        }
        if let Some(z) = self.zone {
            m.push(("zone".to_string(), Value::Str(z.as_str().to_string())));
        }
        for (k, v) in &self.extra {
            if !m.iter().any(|(n, _)| n == k) {
                m.push((k.clone(), v.clone()));
            }
        }
        let bytes = json::canon_bytes(&Value::Obj(m));
        crate::local::put(&home.dir(Slot::Settings), FILE, crate::local::Doc::Settings, &bytes)
    }

    pub fn ground(&self) -> eravon_seal::ground::Ground {
        eravon_seal::ground::Ground {
            chain_id: self.chain_id,
            endpoints: self.endpoints.clone(),
            rest: self.rest.clone(),
            rest_on: self.rest_on,
            log_asks: self.log_asks,
        }
    }

    pub fn log_asks(&self) -> u64 {
        self.log_asks.filter(|n| eravon_chain::action::asks_within(*n)).unwrap_or(eravon_chain::action::ASKS_DEFAULT)
    }

    pub fn over_cap(&self, home: &Home) -> Result<Option<(u64, u64)>, Fault> {
        let used = home.usage()?;
        Ok(if used > self.cap_bytes { Some((used, self.cap_bytes)) } else { None })
    }
}

pub fn migrate(from: &Home, to: &std::path::Path) -> Result<Home, Fault> {
    crate::trace::mark(crate::feature::Feature::H3);
    outside_home(from.root(), to)?;
    if to.exists() && std::fs::read_dir(to).map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err(Fault::known(Known::Occupied, to.display().to_string()));
    }
    copy_tree(from.root(), to)?;
    let diffs = compare_tree(from.root(), to)?;
    if !diffs.is_empty() {
        return Err(Fault::known(Known::CopyMismatch, crate::lang::both_of(|| crate::lang::filln(crate::lang::Key::Tail210, &[&(diffs.len()).to_string(), &(diffs.join(" ")).to_string()]))));
    }
    let home = Home::open(to)?;
    crate::home::write_pointer(to)?;
    Ok(home)
}

pub fn outside_home(root: &std::path::Path, to: &std::path::Path) -> Result<(), Fault> {
    let root = std::fs::canonicalize(root).map_err(|e| classify(&e, &root.display().to_string()))?;
    let target = real_path(to)?;
    if target.starts_with(&root) {
        return Err(Fault::known(Known::InsideHome, format!("{} {}", to.display(), root.display())));
    }
    Ok(())
}

fn real_path(p: &std::path::Path) -> Result<std::path::PathBuf, Fault> {
    use std::path::Component;
    let mut rest: Vec<Component> = Vec::new();
    let mut at = p;
    let base = loop {
        match std::fs::canonicalize(at) {
            Ok(b) => break b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(classify(&e, &at.display().to_string())),
        }
        let (Some(name), Some(up)) = (at.components().next_back(), at.parent()) else {
            return Err(Fault::known(Known::PathRelative, p.display().to_string()));
        };
        rest.push(name);
        at = up;
    };
    let mut out = base;
    for c in rest.into_iter().rev() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            c => out.push(c.as_os_str()),
        }
    }
    Ok(out)
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) -> Result<(), Fault> {
    std::fs::create_dir_all(to).map_err(|e| classify(&e, &to.display().to_string()))?;
    for e in std::fs::read_dir(from).map_err(|e| classify(&e, &from.display().to_string()))? {
        let e = e.map_err(|e| classify(&e, &from.display().to_string()))?;
        let src = e.path();
        let dst = to.join(e.file_name());
        let md = std::fs::symlink_metadata(&src).map_err(|e| classify(&e, &src.display().to_string()))?;
        if md.is_dir() {
            copy_tree(&src, &dst)?;
        } else if md.is_file() {
            let bytes = std::fs::read(&src).map_err(|e| classify(&e, &src.display().to_string()))?;
            eravon_store::landing::land_bytes(&dst, &bytes)
                .map_err(|t| Fault::of_landing(t))?;
        }
    }
    Ok(())
}

fn compare_tree(a: &std::path::Path, b: &std::path::Path) -> Result<Vec<String>, Fault> {
    let mut bad = Vec::new();
    for e in std::fs::read_dir(a).map_err(|e| classify(&e, &a.display().to_string()))? {
        let e = e.map_err(|e| classify(&e, &a.display().to_string()))?;
        let src = e.path();
        let dst = b.join(e.file_name());
        let md = std::fs::symlink_metadata(&src).map_err(|e| classify(&e, &src.display().to_string()))?;
        if md.is_dir() {
            bad.extend(compare_tree(&src, &dst)?);
        } else if md.is_file() {
            let x = std::fs::read(&src).unwrap_or_default();
            let y = std::fs::read(&dst).unwrap_or_default();
            if x != y {
                bad.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }
    Ok(bad)
}

