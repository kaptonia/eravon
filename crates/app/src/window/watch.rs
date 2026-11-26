use super::*;

impl Win {
    pub(super) fn banner(&mut self, ui: &mut egui::Ui, now: f64) {
        match self.shell.banner() {
            crate::shell::Banner::None => {}
            which @ (crate::shell::Banner::OtherMachine | crate::shell::Banner::MarkUnread) => {
                let say = if which == crate::shell::Banner::MarkUnread { Key::SetMarkUnread } else { Key::SetOtherMachine };
                let take = states::banner(ui, states::Banner::Warn, t(say), |ui| key::key(ui, t(Key::SetTakeWriter), Role::Secondary, true).clicked());
                if take {
                    self.act(Action::TakeWriter, now);
                }
            }
            crate::shell::Banner::ReadOnly(who) => {
                let why = if who.is_empty() { t(Key::HomeNoLock).to_string() } else { who };
                states::note_box(ui, &fill1(Key::ReadOnlyBar, &why));
            }
        }
    }

    pub(super) fn alert_count(&self) -> usize {
        let d = &self.shell.desk;
        match d.seat() {
            crate::house::shell::Seat::Seller => d.sh.seller.home.as_ref().filter(|h| h.fresh_for(&d.addr)).map_or(0, |h| h.badge as usize),
            crate::house::shell::Seat::Buyer => d.sh.buyer_home.as_ref().filter(|h| h.fresh_for(&d.addr) && !h.stopped).map_or(0, |h| h.badge as usize),
        }
    }

}
