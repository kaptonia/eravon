use super::*;

impl Win {
    pub(super) fn rail_count(&self, item: Place) -> usize {
        if matches!(item, Place::View(crate::nav::View::Alerts, _)) {
            self.alert_count()
        } else {
            0
        }
    }

    pub(super) fn detail_route(&self, route: &Route) -> bool {
        matches!(route, Route::House(_))
    }

    pub(super) fn bar_title(&self, route: &Route) -> bool {
        matches!(route, Route::Section(_))
    }

    pub(super) fn toolbar_acts(&mut self, ui: &mut egui::Ui, route: &Route, now: f64) {
        self.house_toolbar(ui, route, now);
    }

    pub(super) fn body_before(&mut self, ui: &mut egui::Ui, now: f64) {
        let _ = (ui, now);
    }

    pub(super) fn follow_extra(&mut self, now: f64) {
        self.house_follow(now);
    }

    pub(super) fn page_entered(&mut self, _now: f64) {
        let route = self.route();
        if let Some(want) = route_page(&route, self.ux.nav.place.unwrap_or(Place::Home)) {
            if self.shell.desk.sh.seller.page() != want && self.shell.unlocked() {
                self.house_go(want);
            }
        }
        if route.is_root() && self.hx.rail_landing {
            self.house_rail_root();
        }
        if self.house_detail().is_some_and(|d| d != route && self.hx.route_was.as_ref() == Some(&d)) {
            self.house_leave_to(Some(&route));
        }
        if self.house_detail().as_ref() != Some(&route) {
            if let Route::House(d) = &route {
                self.house_detail_open(d);
            }
        }
        self.hx.route_was = Some(route);
    }
}

impl Win {
    pub(super) fn hist_mut(&mut self) -> &mut History {
        eravon_ui::nav::hist(self)
    }

    pub(super) fn wizard_when_due(&mut self) {
        if std::mem::take(&mut self.shell.desk.wizard_due) {
            self.wizard_open(self.progress().first_open().unwrap_or(crate::nav::Step::Pin));
        }
    }

    pub(super) fn arrive(&mut self, p: Page) {
        match p {
            Page::FirstRun => {
                let step = self.progress().first_open().unwrap_or(crate::nav::Step::Pin);
                self.wizard_open(step);
            }
            Page::Identity | Page::Skeleton | Page::About => {}
        }
    }

    pub(super) fn page_body(&mut self, ui: &mut egui::Ui, route: &Route, now: f64) {
        match route {
            Route::Root(Place::SettingsHome) | Route::Root(Place::Settings(_)) => self.settings_home(ui, now),
            Route::Section(s) => self.settings_section(ui, *s, now),
            _ => self.house_body(ui, route, now),
        }
    }

    pub(super) fn title_of(&self, route: &Route) -> String {
        match route {
            Route::Root(Place::SettingsHome) | Route::Root(Place::Settings(_)) => t(Key::NavSettings).to_string(),
            Route::Section(s) => t(s.key()).to_string(),
            _ => self.house_title(route),
        }
    }
}

fn route_page(route: &Route, root: Place) -> Option<Option<crate::house::seller::Page>> {
    match route {
        Route::Root(p) => Win::page_of(*p),
        Route::Section(_) => Some(Some(crate::house::seller::Page::Settings)),
        _ => Win::page_of(root),
    }
}

