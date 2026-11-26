use super::*;

pub fn switch_to(shell: &mut Shell, want: crate::roles::Role) -> Applied {
    if shell.tasks.in_flight(Kind::Seat) {
        shell.seat_wish = Some(want);
        return Applied::Started(Kind::Seat);
    }
    if shell.settings.role == want {
        shell.seat_wish = None;
        return Applied::Seated(want);
    }
    if !shell.identity_listed() {
        let view = match crate::register::view() {
            Ok(v) => v,
            Err(f) => return shell.trouble(f),
        };
        if let Err(f) = shell.commit_settings(|s| s.role = want) {
            return shell.trouble(f);
        }
        shell.seat_identities(Some(view));
        shell.seat_follow();
        return Applied::Seated(shell.settings.role);
    }
    super::office::seat_start(shell, want)
}

pub(super) struct RerootCarry;

pub(super) fn reroot_carry(_shell: &Shell) -> RerootCarry {
    RerootCarry
}

pub(super) fn reroot_carried(_shell: &mut Shell, _carried: RerootCarry, _now_id: Option<&str>) {}
