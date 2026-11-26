use eravon_store::fault::Fault as StoreFault;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Label,
    Address(String),
    Pointer(String),
    Chain { want: u64, got: u64 },
    Exists(String),
    Missing(String),
    Kind(String),
    Store { name: &'static str, detail: String },
}

pub type R<T> = Result<T, Fault>;

impl Fault {
    pub fn name(&self) -> &'static str {
        match self {
            Fault::Label => "E_CANDS_LABEL",
            Fault::Address(_) => "E_CANDS_ADDRESS",
            Fault::Pointer(_) => "E_CANDS_POINTER",
            Fault::Chain { .. } => "E_CANDS_CHAIN",
            Fault::Exists(_) => "E_CANDS_EXISTS",
            Fault::Missing(_) => "E_CANDS_MISSING",
            Fault::Kind(_) => "E_CANDS_KIND",
            Fault::Store { name, .. } => name,
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Fault::Label => "请填写备注名".into(),
            Fault::Address(a) => format!("{a:?} 不是有效地址"),
            Fault::Pointer(p) => format!("{p:?} 不是有效的行会地址"),
            Fault::Chain { want, got } => format!("网络不匹配({got} ≠ {want})"),
            Fault::Exists(a) => format!("{a} 已存在"),
            Fault::Missing(a) => format!("找不到 {a}"),
            Fault::Kind(k) => format!("候选表上有一行的类是 {k:?},只有 judge 与 guild 两种;这一本表是早先按错位写下来的"),
            Fault::Store { detail, .. } => detail.clone(),
        }
    }
}

impl From<StoreFault> for Fault {
    fn from(e: StoreFault) -> Fault {
        Fault::Store { name: e.name(), detail: e.detail() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Absent {
    NoRegistry,
    Unreachable(String),
    NoDocument,
    Unverified(String),
    Retired,
    Succeeded { to: String, block: u64 },
}

impl Absent {
    pub fn name(&self) -> &'static str {
        match self {
            Absent::NoRegistry => "E_CANDS_NO_REGISTRY",
            Absent::Unreachable(_) => "E_CANDS_UNREACHABLE",
            Absent::NoDocument => "E_CANDS_NO_DOCUMENT",
            Absent::Unverified(_) => "E_CANDS_UNVERIFIED",
            Absent::Retired => "E_CANDS_RETIRED",
            Absent::Succeeded { .. } => "E_CANDS_SUCCEEDED",
        }
    }

    pub fn class(&self) -> &'static str {
        match self {
            Absent::NoRegistry | Absent::Unreachable(_) | Absent::NoDocument => "unreachable",
            Absent::Unverified(_) => "unverified",
            Absent::Retired => "retired",
            Absent::Succeeded { .. } => "succeeded",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Absent::NoRegistry => "暂时读不到这位判官的资料:判官登记处还没设好".into(),
            Absent::Unreachable(e) => format!("无法连接:{e}"),
            Absent::NoDocument => "未找到该判官的资料".into(),
            Absent::Unverified(e) => format!("校验失败:{e}"),
            Absent::Retired => "已停业".into(),
            Absent::Succeeded { to, block } => format!("已交给继任 {to}(第 {block} 块),继任人在登记处还没有判官资料"),
        }
    }
}
