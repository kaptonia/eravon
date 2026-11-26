use super::*;
use crate::landing::{self, Back, OnOk};

impl Win {
    pub(super) fn land(&mut self, ctx: &egui::Context, now: f64) {
        let landed = self.shell.drain_at(now);
        self.paths_landed(ctx, &landed);
        self.sync_landed(&landed, now);
        let claimed: Vec<_> = landed.into_iter().filter(|o| !landing::goes_back(o.kind, Back::Said)).filter(|o| self.ux.claim(o.kind)).collect();
        for o in claimed {
            let kind = o.kind;
            if o.renoded {
                continue;
            }
            let good = match &o.result {
                Ok(d) => !done_is_bad(d),
                Err(_) => false,
            };
            if !(o.stale && o.result.is_ok()) {
                self.ux.landed.insert(kind, (good, now));
            }
            if o.result.is_ok() && (landing::of(kind).ok == OnOk::Quiet || o.stale) {
                continue;
            }
            let (said, tone) = match o.result {
                Ok(Done::Check(r)) => super::house::said::self_checked(&r),
                Ok(Done::Keystore(crate::task::Keystore::BackedUp { path, .. })) => (fill1(Key::SaidKeyBackedUp, &width::file_name(&path)), Tone::Note),
                Ok(Done::BackupMade { path, .. }) => (fill1(Key::SaidBackupMade, &width::file_name(&path)), Tone::Note),
                Ok(Done::BackupSeen { .. }) => continue,
                Ok(Done::Vault(_)) => continue,
                Ok(Done::House(_)) | Ok(Done::Archive { .. }) => continue,
                Ok(Done::Path(_)) => continue,
                Ok(Done::CliPath(eravon_os::cli_path::State::On)) => (t(Key::SaidCliPathOn).to_string(), Tone::Note),
                Ok(Done::CliPath(eravon_os::cli_path::State::Off)) => (t(Key::SaidCliPathOff).to_string(), Tone::Note),
                Ok(Done::CliPath(_)) => continue,
                Ok(Done::Door(_)) => continue,
                Ok(Done::Seated(_)) => continue,
                Ok(Done::Kept(_)) => continue,
                Err(_) => continue,
            };
            let view = self.task_away(kind).then(|| vec![(t(Key::NavLook).to_string(), eravon_ui::toast::Act::Tag(kind as u64))]).unwrap_or_default();
            self.toasts.say_keys(said, "", "", tone, now, view);
        }

        for k in crate::task::Kind::ALL.into_iter().filter(|k| landing::goes_back(*k, Back::Said)) {
            if let Some(a) = self.shell.said.remove(&k) {
                self.said_back(k, a, now);
            }
        }
        if let Some(a) = self.shell.vault_said.take() {
            let site = self.ux.vault_site.take();
            let a = self.told(a, None, now);
            if let Some(site) = site {
                self.vault_back(site, a, now);
            }
        }
    }
}
