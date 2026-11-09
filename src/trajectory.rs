//! The canonical cross-platform trajectory: one fixture,
//! run identically on every platform, and the FNV-1a digest
//! of its per-step record.
//!
//! The digest discipline is the coin pilot's construction
//! (itself the estate's `state_hash` discipline), re-implemented
//! here so this crate gains no dependency on the app crate:
//! FNV-1a 64-bit (offset basis `0xcbf29ce484222325`, prime
//! `0x100000001b3`) over the little-endian byte stream of each
//! record's words, in order. The record here is the session
//! snapshot after each fixture step: the two pause layers,
//! the audit counter, and every girl's (girl state, nav
//! state, embarrassment) discriminants in roster order.
//! Equal digests on two platforms mean the two runs walked
//! the same states at the same steps, bit for bit — the S7
//! cross-platform determinism gate (PIPELINE.md, D-212).

use crate::dynamics::{Dynamics, EmbarrassmentLevel, GirlState, NavState, TimestopError};
use crate::measured_dynamics;
use crate::session::Session;

/// FNV-1a offset basis (64-bit).
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a prime (64-bit).
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// An incremental FNV-1a hasher over little-endian u64
/// words, byte at a time — the `state_hash` discipline.
struct TrajectoryHasher {
    state: u64,
}

impl TrajectoryHasher {
    fn new() -> TrajectoryHasher {
        TrajectoryHasher {
            state: FNV_OFFSET_BASIS,
        }
    }

    fn write_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

fn girl_state_word(state: GirlState) -> u64 {
    match state {
        GirlState::None => 0,
        GirlState::Normal => 1,
        GirlState::NoClothes => 2,
        GirlState::RunningAway => 3,
        GirlState::WrongState => 4,
        GirlState::SawNaked => 5,
    }
}

fn nav_state_word(state: Option<NavState>) -> u64 {
    match state {
        None => u64::MAX,
        Some(NavState::None) => 0,
        Some(NavState::Random) => 1,
        Some(NavState::LocationEntering) => 2,
        Some(NavState::LocationTraversing) => 3,
        Some(NavState::LocationExiting) => 4,
        Some(NavState::ShopLooking) => 5,
        Some(NavState::InQueue) => 6,
    }
}

fn embarrassment_word(level: EmbarrassmentLevel) -> u64 {
    match level {
        EmbarrassmentLevel::None => 0,
        EmbarrassmentLevel::OnlyPanties => 1,
        EmbarrassmentLevel::WrongState => 2,
        EmbarrassmentLevel::Shoes => 3,
        EmbarrassmentLevel::MajorClothing => 4,
    }
}

/// Snapshot one step of the session into the hasher.
fn write_snapshot(hasher: &mut TrajectoryHasher, session: &Session) {
    hasher.write_u64(u64::from(session.time_stopped()));
    hasher.write_u64(u64::from(session.game_paused()));
    hasher.write_u64(session.audit_runs());
    for girl in session.girls() {
        hasher.write_u64(girl_state_word(girl.girl_state()));
        hasher.write_u64(nav_state_word(girl.nav_state()));
        hasher.write_u64(embarrassment_word(girl.embarrassment()));
    }
}

/// The result of one fixture run.
#[derive(Clone, Debug, PartialEq)]
pub struct FixtureRun {
    /// FNV-1a digest of the run's per-step record.
    pub trajectory_hash: u64,
    /// Fixture steps executed (one snapshot per step).
    pub steps_executed: usize,
    /// Resume audits run across the fixture (two resumes).
    pub audit_runs: u64,
    /// Total audit effects across both audits (both fire on
    /// the first resume; the second finds no answering row).
    pub audit_effects: usize,
    /// walker-01's girl state at the end (NoClothes, via the
    /// first audit).
    pub first_girl_state: GirlState,
    /// walker-02's girl state at the end (WrongState, via
    /// the first audit).
    pub second_girl_state: GirlState,
}

/// Run the canonical fixture over `dynamics`: grant the
/// Watch, stop time, focus two girls and set their derived
/// embarrassment levels, resume (audit one), walk one nav
/// cycle, stop and resume again (audit two). The same code
/// runs in `timestop-smoke` on Linux and in any future
/// Android shell — the printed hash is the S7 comparison
/// value.
pub fn run_fixture(dynamics: Dynamics) -> Result<FixtureRun, TimestopError> {
    let mut session = Session::new(dynamics);
    let mut hasher = TrajectoryHasher::new();
    let mut steps = 0usize;
    let mut audit_effects = 0usize;

    macro_rules! step {
        () => {{
            write_snapshot(&mut hasher, &session);
            steps += 1;
        }};
    }

    session.grant_watch();
    step!();
    session.toggle_time_stop()?;
    step!();
    session.interact("walker-01")?;
    step!();
    session.set_embarrassment("walker-01", EmbarrassmentLevel::MajorClothing)?;
    step!();
    session.interact("walker-02")?;
    step!();
    session.set_embarrassment("walker-02", EmbarrassmentLevel::WrongState)?;
    step!();
    let first = session.resume_time()?;
    audit_effects += first.effects.len();
    step!();
    session.apply_nav_input("walker-01", crate::NavInput::ToLocation)?;
    step!();
    session.apply_nav_input("walker-01", crate::NavInput::ToLocationTraverse)?;
    step!();
    session.toggle_time_stop()?;
    step!();
    let second = session.resume_time()?;
    audit_effects += second.effects.len();
    step!();

    Ok(FixtureRun {
        trajectory_hash: hasher.finish(),
        steps_executed: steps,
        audit_runs: session.audit_runs(),
        audit_effects,
        first_girl_state: session.girl("walker-01")?.girl_state(),
        second_girl_state: session.girl("walker-02")?.girl_state(),
    })
}

/// The fixture over the measured tables.
pub fn run_measured_fixture() -> Result<FixtureRun, TimestopError> {
    run_fixture(measured_dynamics()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stream_hashes_to_the_offset_basis() {
        assert_eq!(TrajectoryHasher::new().finish(), FNV_OFFSET_BASIS);
    }

    #[test]
    fn fixture_hash_is_deterministic_across_runs() {
        let a = run_measured_fixture().expect("first run");
        let b = run_measured_fixture().expect("second run");
        assert_eq!(a, b);
        assert_eq!(a.audit_runs, 2);
        assert_eq!(a.audit_effects, 2);
        assert_eq!(a.first_girl_state, GirlState::NoClothes);
        assert_eq!(a.second_girl_state, GirlState::WrongState);
    }
}
