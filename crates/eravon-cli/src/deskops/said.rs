use eravon_literals::json::Out;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    pub tone: Tone,
    pub name: String,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Bad,
    Note,
}

impl Tone {
    pub const ALL: [Tone; 3] = [Tone::Ok, Tone::Bad, Tone::Note];

    pub const fn word(self) -> &'static str {
        match self {
            Tone::Ok => "ok",
            Tone::Bad => "bad",
            Tone::Note => "note",
        }
    }
}

impl Said {
    pub fn ok(detail: impl Into<String>) -> Said {
        Said { tone: Tone::Ok, name: "ok".into(), detail: detail.into() }
    }
    pub fn bad(name: &str, detail: impl Into<String>) -> Said {
        Said { tone: Tone::Bad, name: name.to_string(), detail: detail.into() }
    }
    pub fn out(&self) -> Out {
        Out::O(vec![
            ("detail", Out::S(self.detail.clone())),
            ("name", Out::S(self.name.clone())),
            ("tone", Out::S(self.tone.word().to_string())),
        ])
    }
}

