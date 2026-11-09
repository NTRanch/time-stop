//! The session: the time-stop gate, the per-girl machines,
//! the resume audit, and the save record, composed over the
//! measured tables.
//!
//! The freeze here is the original's measured design (spec
//! §2): an event-subscription bool, NOT a timescale. Toggling
//! flips one bool; the systems that obey are the ones this
//! module gates on it — girl interaction (§3), the animation
//! clock scale (the `GlobalSpeed` contract, animation note
//! §3), and the pausable delay primitive (§2 `DelayHelper`).
//! The separate game pause is a distinct layer with its own
//! flag, exactly as measured: the time toggle is ignored
//! while game-paused, surfaced here as a typed refusal.

use serde::{Deserialize, Serialize};

use crate::dynamics::{
    Dynamics, EmbarrassmentLevel, GirlInput, GirlState, NavInput, NavState, RosterRole,
    TimestopError,
};

/// The entire persistent truth of the game (spec §7,
/// MEASURED field list): two pickup flags, two settings
/// toggles, two hint flags, and the gallery unlock list.
///
/// The original persists this with `BinaryFormatter` to
/// `Data.timefreeze`; the remake deliberately does not
/// (spec §9, Avoid 1) — it serializes the same seven fields
/// as deterministic RON: field order is declaration order,
/// and unknown fields are refused at load.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveData {
    /// The Watch pickup: the time-stop gate's key (spec §2/§7).
    pub has_watch: bool,
    /// The Camera pickup: the gallery gate's key (spec §7).
    pub has_camera: bool,
    /// Settings toggle, surfaced in the menu (spec §7).
    pub sounds_on: bool,
    /// Settings toggle, surfaced in the menu (spec §7).
    pub music_on: bool,
    /// Tutorial bookkeeping (spec §7).
    pub showed_initial_hint: bool,
    /// Tutorial bookkeeping (spec §7).
    pub showed_cashier_hint: bool,
    /// Gallery unlock list; Contains-checked, no duplicates
    /// (spec §7).
    pub unlocked_arts: Vec<i32>,
}

impl SaveData {
    /// Serialize deterministically: same record, same bytes,
    /// on every platform (the S7 determinism discipline).
    pub fn to_ron(&self) -> Result<String, TimestopError> {
        ron::ser::to_string(self).map_err(|_| TimestopError::Invalid {
            field: "save",
            reason: "the save record failed to serialize",
        })
    }

    /// Parse a save record. Unknown fields, wrong types, and
    /// duplicate unlock ids are typed refusals.
    pub fn from_ron(text: &str) -> Result<SaveData, TimestopError> {
        let data: SaveData = ron::de::from_str(text)?;
        let mut seen = std::collections::BTreeSet::new();
        for art in &data.unlocked_arts {
            if !seen.insert(art) {
                return Err(TimestopError::Invalid {
                    field: "unlocked_arts",
                    reason: "the unlock list carries a duplicate id",
                });
            }
        }
        Ok(data)
    }
}

/// One girl in the session: her two layered machines and
/// her derived embarrassment level.
#[derive(Clone, Debug, PartialEq)]
pub struct Girl {
    id: String,
    role: RosterRole,
    girl_state: GirlState,
    nav_state: Option<NavState>,
    embarrassment: EmbarrassmentLevel,
}

impl Girl {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn role(&self) -> RosterRole {
        self.role
    }

    pub fn girl_state(&self) -> GirlState {
        self.girl_state
    }

    /// The nav machine's state, or `None` for the stationary
    /// cashier (spec §1: `WalkingGirl` x12, the cashier does
    /// not walk).
    pub fn nav_state(&self) -> Option<NavState> {
        self.nav_state
    }

    pub fn embarrassment(&self) -> EmbarrassmentLevel {
        self.embarrassment
    }
}

/// One audited girl transition from a resume audit: who
/// moved, from what, to what, on which input.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditEffect {
    pub girl_id: String,
    pub input: GirlInput,
    pub from: GirlState,
    pub to: GirlState,
}

/// The outcome of one resume audit (spec §3).
#[derive(Clone, Debug, PartialEq)]
pub struct AuditReport {
    /// Girls whose state machine moved under the audit.
    pub effects: Vec<AuditEffect>,
}

/// A pausable delay primitive (spec §2, `DelayHelper`): it
/// accumulates time only while time flows, so any timed
/// sequence built on it is automatically correct under the
/// freeze (spec §9, Adopt 6).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PausableTimer {
    accumulated_seconds: f64,
}

impl PausableTimer {
    pub fn new() -> PausableTimer {
        PausableTimer {
            accumulated_seconds: 0.0,
        }
    }

    pub fn accumulated_seconds(&self) -> f64 {
        self.accumulated_seconds
    }
}

/// The session state: save record, the two pause layers,
/// the roster of girls, the focus, and the audit counter.
#[derive(Clone, Debug)]
pub struct Session {
    dynamics: Dynamics,
    save: SaveData,
    game_paused: bool,
    time_stopped: bool,
    girls: Vec<Girl>,
    focused: Option<String>,
    audit_runs: u64,
}

impl Session {
    /// A fresh session over the measured tables, with a
    /// default (all-false, empty) save record.
    pub fn new(dynamics: Dynamics) -> Session {
        Session::with_save(dynamics, SaveData::default())
    }

    /// A session resumed from a save record.
    pub fn with_save(dynamics: Dynamics, save: SaveData) -> Session {
        let girls = dynamics
            .roster()
            .iter()
            .map(|entry| Girl {
                id: entry.id.clone(),
                role: entry.role,
                girl_state: dynamics.girl_initial(),
                nav_state: match entry.role {
                    RosterRole::Walker => Some(dynamics.nav_initial()),
                    RosterRole::Cashier => None,
                },
                embarrassment: EmbarrassmentLevel::None,
            })
            .collect();
        Session {
            dynamics,
            save,
            game_paused: false,
            time_stopped: false,
            girls,
            focused: None,
            audit_runs: 0,
        }
    }

    pub fn save(&self) -> &SaveData {
        &self.save
    }

    pub fn girls(&self) -> &[Girl] {
        &self.girls
    }

    pub fn girl(&self, id: &str) -> Result<&Girl, TimestopError> {
        self.girls
            .iter()
            .find(|g| g.id == id)
            .ok_or_else(|| TimestopError::UnknownGirl { id: id.to_owned() })
    }

    fn girl_mut(&mut self, id: &str) -> Result<&mut Girl, TimestopError> {
        self.girls
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or_else(|| TimestopError::UnknownGirl { id: id.to_owned() })
    }

    pub fn time_stopped(&self) -> bool {
        self.time_stopped
    }

    pub fn game_paused(&self) -> bool {
        self.game_paused
    }

    /// How many resume audits have run in this session. The
    /// audit runs exactly once per stop→resume edge (§3);
    /// this counter is how a test proves it.
    pub fn audit_runs(&self) -> u64 {
        self.audit_runs
    }

    /// The girl currently in focus, if any (spec §3:
    /// `FocusHandler.CurrentGirl`).
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// The animation clock scale every puppet multiplies in
    /// (animation note §3: the `GlobalSpeed` contract — a
    /// step, 0.0 while stopped and 1.0 while flowing, never
    /// a ramp, written from the time state only).
    pub fn anim_clock_scale(&self) -> f64 {
        if self.time_stopped { 0.0 } else { 1.0 }
    }

    /// The Watch pickup (spec §7): sets `HasWatch`. A pickup
    /// hides itself when the flag is already set, so a second
    /// grant is an idempotent no-op, not an error.
    pub fn grant_watch(&mut self) {
        self.save.has_watch = true;
    }

    /// The Camera pickup (spec §7): sets `HasCamera`.
    pub fn grant_camera(&mut self) {
        self.save.has_camera = true;
    }

    /// The game pause layer (spec §2): a flag of its own,
    /// sharing no machinery with the time stop.
    pub fn set_game_paused(&mut self, paused: bool) {
        self.game_paused = paused;
    }

    /// Toggle the time stop (spec §2). Refused without the
    /// Watch and while game-paused. On the stop→resume edge
    /// the consequence audit runs exactly once and its
    /// report is returned; on the resume→stop edge, `None`.
    pub fn toggle_time_stop(&mut self) -> Result<Option<AuditReport>, TimestopError> {
        if !self.save.has_watch {
            return Err(TimestopError::WatchMissing);
        }
        if self.game_paused {
            return Err(TimestopError::GamePaused);
        }
        if self.time_stopped {
            self.time_stopped = false;
            self.focused = None;
            let report = self.run_audit();
            Ok(Some(report))
        } else {
            self.time_stopped = true;
            Ok(None)
        }
    }

    /// Resume time, spelled out: a typed refusal when time
    /// is not stopped, so a second resume can never re-run
    /// the audit.
    pub fn resume_time(&mut self) -> Result<AuditReport, TimestopError> {
        if !self.time_stopped {
            return Err(TimestopError::NotStopped);
        }
        match self.toggle_time_stop()? {
            Some(report) => Ok(report),
            None => Err(TimestopError::NotStopped),
        }
    }

    /// The consequence audit (spec §3): one deterministic
    /// pass over every girl's embarrassment level, in roster
    /// order, applying the measured audit inputs. Runs only
    /// from the resume edge — there is no other caller.
    fn run_audit(&mut self) -> AuditReport {
        self.audit_runs += 1;
        let mut effects = Vec::new();
        for index in 0..self.girls.len() {
            let level = self.girls[index].embarrassment;
            let Some(input) = self.dynamics.audit_input(level) else {
                continue;
            };
            let from = self.girls[index].girl_state;
            if let Some(to) = self.dynamics.girl_step(from, input) {
                self.girls[index].girl_state = to;
                effects.push(AuditEffect {
                    girl_id: self.girls[index].id.clone(),
                    input,
                    from,
                    to,
                });
            }
        }
        AuditReport { effects }
    }

    /// Interact with a girl (spec §3): focus mode. The gate
    /// is the measured rule — girls are interactable ONLY
    /// while time is stopped — a state gate, not a distance
    /// check (spec §9, Adopt 5), so it cannot be bypassed.
    pub fn interact(&mut self, id: &str) -> Result<(), TimestopError> {
        self.girl(id)?;
        if !self.time_stopped {
            return Err(TimestopError::InteractionWhileFlowing { id: id.to_owned() });
        }
        self.focused = Some(id.to_owned());
        Ok(())
    }

    /// End the current focus (spec §3:
    /// `StoppedGirlInteraction`).
    pub fn end_interaction(&mut self) -> Result<(), TimestopError> {
        if self.focused.take().is_none() {
            return Err(TimestopError::NoFocus);
        }
        Ok(())
    }

    /// Record a girl's derived embarrassment level.
    ///
    /// This is the systems core's port for the clothing
    /// fold (spec §6: `ClothesManager.Embarrased()` folds
    /// the applied pieces into a level). The fold's exact
    /// rule — which piece/stage combinations yield which
    /// level — is NOT pinned by the dynamics spec, so the
    /// core carries the level as measured vocabulary and
    /// consumes it in the audit (§3); the fold itself is
    /// flagged in the build report's REVIEW register rather
    /// than invented here.
    pub fn set_embarrassment(
        &mut self,
        id: &str,
        level: EmbarrassmentLevel,
    ) -> Result<(), TimestopError> {
        self.girl_mut(id)?.embarrassment = level;
        Ok(())
    }

    /// Apply one input to a girl's state machine. Returns
    /// the (from, to) pair when a measured row answers, or
    /// `None` when the machine does not move.
    pub fn apply_girl_input(
        &mut self,
        id: &str,
        input: GirlInput,
    ) -> Result<Option<(GirlState, GirlState)>, TimestopError> {
        let from = self.girl(id)?.girl_state;
        match self.dynamics.girl_step(from, input) {
            Some(to) => {
                self.girl_mut(id)?.girl_state = to;
                Ok(Some((from, to)))
            }
            None => Ok(None),
        }
    }

    /// Apply one input to a walking girl's nav machine.
    /// The cashier has no nav machine (spec §1); applying a
    /// nav input to her is a typed refusal, not a no-op.
    pub fn apply_nav_input(
        &mut self,
        id: &str,
        input: NavInput,
    ) -> Result<Option<(NavState, NavState)>, TimestopError> {
        let from = match self.girl(id)?.nav_state {
            Some(state) => state,
            None => {
                return Err(TimestopError::Invalid {
                    field: "nav_state",
                    reason: "the cashier is stationary and has no nav machine",
                });
            }
        };
        match self.dynamics.nav_step(from, input) {
            Some(to) => {
                self.girl_mut(id)?.nav_state = Some(to);
                Ok(Some((from, to)))
            }
            None => Ok(None),
        }
    }

    /// Photograph an artwork (spec §7): requires the Camera;
    /// appends the id to the unlock list, Contains-checked.
    pub fn photograph(&mut self, art_id: i32) -> Result<(), TimestopError> {
        if !self.save.has_camera {
            return Err(TimestopError::CameraMissing);
        }
        if self.save.unlocked_arts.contains(&art_id) {
            return Err(TimestopError::AlreadyUnlocked { art_id });
        }
        self.save.unlocked_arts.push(art_id);
        Ok(())
    }

    /// Advance a pausable timer by `dt_seconds` of flowing
    /// time (spec §2 `DelayHelper`): while stopped, the
    /// timer does not move. Non-finite or negative steps are
    /// typed refusals.
    pub fn advance_timer(
        &self,
        timer: &mut PausableTimer,
        dt_seconds: f64,
    ) -> Result<(), TimestopError> {
        if !dt_seconds.is_finite() || dt_seconds < 0.0 {
            return Err(TimestopError::InvalidStep {
                reason: "a timer step must be finite and non-negative",
            });
        }
        if !self.time_stopped {
            timer.accumulated_seconds += dt_seconds;
        }
        Ok(())
    }
}
