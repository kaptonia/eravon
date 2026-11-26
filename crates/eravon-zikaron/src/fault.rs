#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Absent {
    NotConnected {
        looked: Vec<(String, String)>,
    },
    PointerStale {
        pointer: String,
        at: String,
        why: String,
    },
    NoKits {
        at: String,
        why: String,
    },
}

impl Absent {
    pub fn name(&self) -> &'static str {
        match self {
            Absent::NotConnected { .. } => "E_ZK_NOT_CONNECTED",
            Absent::PointerStale { .. } => "E_ZK_POINTER_STALE",
            Absent::NoKits { .. } => "E_ZK_NO_KITS",
        }
    }


    pub fn detail(&self) -> String {
        match self {
            Absent::NotConnected { looked } => {
                let each: Vec<String> =
                    looked.iter().map(|(at, why)| format!("{at}({why})")).collect();
                format!("三层都探不到 ZIKARON 机器目录;探过 {}", each.join("、"))
            }
            Absent::PointerStale { pointer, at, why } => {
                format!("指针档 {pointer} 指到 {at},那一处 {why};按契约 16 不退缺省")
            }
            Absent::NoKits { at, why } => format!("接在 {at},而 {why}"),
        }
    }

    pub fn at(&self) -> Option<&str> {
        match self {
            Absent::NotConnected { .. } => None,
            Absent::PointerStale { at, .. } | Absent::NoKits { at, .. } => Some(at),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Absent(Absent),
    IndexForm {
        want: &'static str,
        got: String,
    },
    IndexShape(String),
    NoSuchKit(String),
    Pack(String),
    Io(String),
    Arg(String),
}

impl Fault {
    pub fn name(&self) -> &'static str {
        match self {
            Fault::Absent(a) => a.name(),
            Fault::IndexForm { .. } => "E_ZK_INDEX_FORM",
            Fault::IndexShape(_) => "E_ZK_INDEX_SHAPE",
            Fault::NoSuchKit(_) => "E_ZK_NO_SUCH_KIT",
            Fault::Pack(_) => "E_ZK_PACK",
            Fault::Io(_) => "E_ZK_IO",
            Fault::Arg(_) => "E_ZK_ARG",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Fault::Absent(a) => a.detail(),
            Fault::IndexForm { want, got } => {
                format!("索引档的 form 认得的只有字面串 {want:?},档上是 {got};整档拒读")
            }
            Fault::IndexShape(s) | Fault::Pack(s) | Fault::Io(s) | Fault::Arg(s) => s.clone(),
            Fault::NoSuchKit(id) => format!("索引上没有 id 是 {id} 的披露包"),
        }
    }

    pub fn is_absent(&self) -> bool {
        matches!(self, Fault::Absent(_))
    }

    pub fn absent(&self) -> Option<&Absent> {
        match self {
            Fault::Absent(a) => Some(a),
            _ => None,
        }
    }
}

impl From<Absent> for Fault {
    fn from(a: Absent) -> Fault {
        Fault::Absent(a)
    }
}

pub type R<T> = Result<T, Fault>;
