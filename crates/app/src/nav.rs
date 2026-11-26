use crate::lang::Key;
use crate::roles::Role;
use eravon_ui::icons::Glyph;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Page {
    #[default]
    FirstRun,
    Identity,
    Skeleton,
    About,
}

impl Page {
    pub const ALL: [Page; 4] = [
        Page::FirstRun,
        Page::Identity,
        Page::Skeleton,
        Page::About,
    ];

    pub fn title(self) -> &'static str {
        crate::lang::t(self.key())
    }

    pub fn key(self) -> crate::lang::Key {
        match self {
            Page::FirstRun => crate::lang::Key::PageFirstRun,
            Page::Identity => crate::lang::Key::PageIdentity,
            Page::Skeleton => crate::lang::Key::PageSkeleton,
            Page::About => crate::lang::Key::PageAbout,
        }
    }

    pub fn icon(self) -> eravon_ui::icons::Icon {
        use eravon_ui::icons::Icon;
        match self {
            Page::FirstRun
            | Page::Identity
            | Page::Skeleton
            | Page::About => Icon::Gear,
        }
    }
}

#[path = "house/routes.rs"]
mod routes;
pub use routes::{Detail, Own, SeatDone, SeatStep, View, AUTHOR, GRANTEE};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Language,
    Appearance,
    Keys,
    Network,
    Notify,
    Data,
    House(Own),
    Cli,
    About,
}

impl Section {
    pub const ALL: [Section; 8] = [
        Section::Language,
        Section::Appearance,
        Section::Keys,
        Section::Network,
        Section::Notify,
        Section::Data,
        Section::Cli,
        Section::About,
    ];

    pub fn key(self) -> Key {
        match self {
            Section::Language => Key::SetSecGeneral,
            Section::Appearance => Key::SetAppearance,
            Section::Keys => Key::SetKeys,
            Section::Network => Key::SetNetwork,
            Section::Notify => Key::NavAlerts,
            Section::Data => Key::SetSecData,
            Section::House(o) => o.key(),
            Section::Cli => Key::SetCliGroup,
            Section::About => Key::PageAbout,
        }
    }

    pub fn glyph(self) -> eravon_ui::icons::Glyph {
        use eravon_ui::icons::Glyph;
        match self {
            Section::Language => Glyph::Lang,
            Section::Appearance => Glyph::Sun,
            Section::Keys => Glyph::Key,
            Section::Network => Glyph::Globe,
            Section::Notify => Glyph::Watch,
            Section::Data => Glyph::Data,
            Section::House(o) => o.glyph(),
            Section::Cli => Glyph::Terminal,
            Section::About => Glyph::Info,
        }
    }

    pub fn note(self, _role: Role) -> Key {
        match self {
            Section::Language => Key::SetNoteLang,
            Section::Appearance => Key::SetNoteAppearance,
            Section::Keys => Key::SetNoteKeys,
            Section::Network => Key::SetNoteNetwork,
            Section::Notify => Key::SetNoteNotify,
            Section::Data => Key::SetNoteData,
            Section::House(o) => o.note(),
            Section::Cli => Key::SetNoteCli,
            Section::About => Key::SetNoteAbout,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Section::Language => "general",
            Section::Appearance => "look",
            Section::Keys => "identity",
            Section::Network => "network",
            Section::Notify => "notify",
            Section::Data => "data",
            Section::House(o) => o.as_str(),
            Section::Cli => "cli",
            Section::About => "about",
        }
    }
}

pub fn sections(role: Role) -> Vec<Section> {
    routes::sections(&Section::ALL, role)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Place {
    Home,
    View(View, u8),
    SettingsHome,
    Settings(Section),
    Page(Page),
}

pub type Item = eravon_ui::nav::Item<Place, Key>;

pub type Group = eravon_ui::nav::Group<Place, Key>;

pub type History = eravon_ui::nav::History<Route>;

const fn it(place: Place, name: Key, glyph: Glyph) -> Item {
    Item { place, name, glyph }
}

const HOME: Item = it(Place::Home, Key::NavHome, Glyph::Home);
const ALERTS: Item = it(Place::View(View::Alerts, 0), Key::NavAlerts, Glyph::Watch);

pub fn rail(role: Role) -> &'static [Group] {
    match role {
        Role::Author => &AUTHOR,
        Role::Grantee => &GRANTEE,
    }
}

pub fn count(role: Role) -> usize {
    rail(role).iter().map(|g| g.items.len()).sum()
}

pub fn tabs(view: View, _role: Role) -> &'static [u8] {
    match view {
        _ => &[],
    }
}

pub fn home_of(page: Page, _role: Role) -> Place {
    match page {
        Page::FirstRun | Page::Skeleton | Page::About => Place::Settings(Section::About),
        Page::Identity => Place::Settings(Section::Keys),
    }
}

pub fn settle(place: Place, role: Role) -> Place {
    let place = match place {
        Place::Page(p) => home_of(p, role),
        other => other,
    };
    match place {
        Place::View(v, t) => {
            let on_rail = rail(role).iter().flat_map(|g| g.items.iter()).any(|i| matches!(i.place, Place::View(x, _) if x == v));
            if !on_rail {
                return Place::Home;
            }
            let ts = tabs(v, role);
            if !ts.is_empty() && !ts.contains(&t) {
                Place::View(v, ts[0])
            } else {
                Place::View(v, t)
            }
        }
        other => other,
    }
}

pub fn lit(place: Place, role: Role) -> Option<Place> {
    match settle(place, role) {
        Place::Settings(_) | Place::SettingsHome | Place::Page(_) => None,
        Place::Home => Some(Place::Home),
        Place::View(v, _) => rail(role)
            .iter()
            .flat_map(|g| g.items.iter())
            .find(|i| matches!(i.place, Place::View(x, _) if x == v))
            .map(|i| i.place),
    }
}

pub const HOME_NAME: &str = "Home";
pub const SETTINGS_NAME: &str = "Settings";
pub const WIZARD_NAME: &str = "Wizard";

const OLD_SECTIONS: [(&str, Section); 8] = [
    ("language", Section::Language),
    ("appearance", Section::Appearance),
    ("keys", Section::Keys),
    ("seat", Section::Data),
    ("chain", Section::Network),
    ("cadence", Section::Notify),
    ("mirror", Section::Data),
    ("backup", Section::Data),
];

pub fn place_named(name: &str) -> Option<(Place, bool)> {
    let n = name.to_ascii_lowercase();
    if n == HOME_NAME.to_ascii_lowercase() {
        return Some((Place::Home, false));
    }
    if n == WIZARD_NAME.to_ascii_lowercase() {
        return Some((Place::Home, true));
    }
    if n == SETTINGS_NAME.to_ascii_lowercase() {
        return Some((Place::SettingsHome, false));
    }
    if let Some(v) = View::ALL.iter().find(|v| v.as_str().to_ascii_lowercase() == n) {
        return Some((Place::View(*v, 0), false));
    }
    let rest = n.strip_prefix(&format!("{}.", SETTINGS_NAME.to_ascii_lowercase()))?;
    if let Some(s) = sections(Role::Author).iter().find(|s| s.as_str() == rest) {
        return Some((Place::Settings(*s), false));
    }
    OLD_SECTIONS.iter().find(|(o, _)| *o == rest).map(|(_, s)| (Place::Settings(*s), false))
}

pub fn alias_names() -> Vec<String> {
    let mut v = vec![HOME_NAME.to_string(), WIZARD_NAME.to_string(), SETTINGS_NAME.to_string()];
    v.extend(View::ALL.iter().map(|x| x.as_str().to_string()));
    v.extend(sections(Role::Author).iter().map(|s| format!("{SETTINGS_NAME}.{}", s.as_str())));
    v
}

pub use crate::steps::{prev, Progress, Step, Sub};

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash, Default)]
pub enum Stack {
    #[default]
    Home,
    View(View),
    Settings,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Route {
    Root(Place),
    House(Detail),
    Section(Section),
}

impl Route {
    pub fn name(&self) -> String {
        match self {
            Route::Root(Place::Home) => HOME_NAME.to_string(),
            Route::Root(Place::SettingsHome) => SETTINGS_NAME.to_string(),
            Route::Root(Place::Settings(s)) | Route::Section(s) => format!("{SETTINGS_NAME}.{}", s.as_str()),
            Route::Root(Place::View(v, t)) => format!("{}.{t}", v.as_str()),
            Route::Root(Place::Page(p)) => format!("{p:?}"),
            Route::House(d) => d.name(),
        }
    }

    pub fn is_root(&self) -> bool {
        matches!(self, Route::Root(_))
    }
}

pub fn stack_of(place: Place) -> Stack {
    match place {
        Place::Home | Place::Page(_) => Stack::Home,
        Place::View(v, _) => Stack::View(v),
        Place::Settings(_) | Place::SettingsHome => Stack::Settings,
    }
}

pub fn history_of(place: Place, role: Role) -> (Stack, History) {
    let place = settle(place, role);
    let h = match place {
        Place::Settings(s) => History { back: vec![Route::Root(Place::SettingsHome)], cur: Route::Section(s), fwd: Vec::new() },
        other => History::at(Route::Root(other)),
    };
    (stack_of(place), h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_goes_to_the_page_before_then_the_step_before() {
        for role in [Role::Author, Role::Grantee] {
            assert_eq!(prev(Step::Key, Sub::Check, role), Some((Step::Key, Sub::First)));
            assert_eq!(prev(Step::Pin, Sub::First, role), None);
            for pair in Step::walk(role).windows(2) {
                assert_eq!(prev(pair[1], Sub::First, role), Some((pair[0], Sub::First)), "{:?}", pair[1]);
                assert_eq!(prev(pair[1], Sub::Check, role), Some((pair[1], Sub::First)), "{:?}", pair[1]);
            }
        }
    }
}
