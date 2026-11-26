use crate::house::action::Action as Step;
use crate::shell::Shell;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Act {
    pub step: Step,
    pub args: Vec<(String, String)>,
}

impl Act {
    pub fn of(step: Step) -> Act {
        Act { step, args: Vec::new() }
    }

    pub fn with(step: Step, args: &[(&str, &str)]) -> Act {
        Act { step, args: args.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() }
    }

    pub fn arg(&self, k: &str) -> Result<String, String> {
        self.opt(k).map(str::to_string).ok_or_else(|| format!("{} 少了 {k}", self.step.name()))
    }

    pub fn opt(&self, k: &str) -> Option<&str> {
        self.args.iter().find(|(x, _)| x == k).map(|(_, v)| v.as_str())
    }
}

pub type Said = Result<String, String>;

pub fn family_said(sh: &mut Shell, a: crate::action::Action) -> crate::action::Applied {
    let got = crate::action::apply(sh, a);
    said_as_family(sh, got)
}

pub fn family_said_started(sh: &mut Shell, a: crate::action::Action) -> crate::action::Applied {
    let got = crate::action::apply(sh, a);
    said_as_family(sh, got)
}

pub(crate) fn said_as_family(sh: &mut Shell, got: crate::action::Applied) -> crate::action::Applied {
    if let crate::action::Applied::Trouble(f) = &got {
        sh.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: f.human().to_string(), next: f.next().to_string(), detail: Some(f.shown()) });
    }
    got
}

pub fn apply_said(sh: &mut Shell, a: Act) {
    let before = sh.desk.sh.toasts.clone();
    if let Err(e) = apply(sh, a) {
        if sh.desk.sh.toasts == before && !before.iter().any(|t| t.head == e) {
            sh.desk.sh.toast(crate::house::shell::Toast { tone: crate::house::seller::Tone::Bad, head: e, next: String::new(), detail: None });
        }
    }
}

pub fn apply(sh: &mut Shell, a: Act) -> Said {
    if sh.desk.data.is_empty() {
        sh.desk.data = crate::house::data().map_err(|f| f.said().to_string())?;
        sh.desk.session = eravon_vault::session::Session::locked(&sh.desk.data);
    }
    if sh.held(a.step) {
        return Err(crate::house::i18n::t(crate::house::i18n::Key::PinLockedTitle));
    }
    let said = crate::house::exec::run(sh, &a);
    sh.desk.flush();
    said
}
