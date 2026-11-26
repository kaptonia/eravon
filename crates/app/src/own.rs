use std::path::Path;

pub fn home_follows(from: &Path, to: &Path) -> Result<(), crate::fault::Fault> {
    let _ = (from, to);
    Ok(())
}

pub type OwnStep = crate::nav::SeatStep;

pub const WALK: [crate::steps::Step; 6] = [
    crate::steps::Step::Pin,
    crate::steps::Step::Key,
    crate::steps::Step::Own(OwnStep::Network),
    crate::steps::Step::Own(OwnStep::Role),
    crate::steps::Step::Own(OwnStep::Tail),
    crate::steps::Step::Backup,
];

pub type OwnProgress = crate::nav::SeatDone;
