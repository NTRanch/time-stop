//! The measured tables for the Time Stop systems, as data.
//!
//! The tables ship as RON (`data/timestop-dynamics.ron`, schema
//! `torogao-timestop-dynamics/1`) following the estate's RON
//! contract discipline: the file is the contract, this module
//! validates it at load against the measured vocabularies, and
//! any value outside the measured shape is a typed refusal —
//! never a silent default. The human record of record is
//! `games/time-stop-dynamics.md` in the Lane D programme tree;
//! every value here is transcribed from it (and through it,
//! from the TimeStop1 IL export, byte-identical across two
//! independent ikdasm runs).

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

/// The embedded measured tables (see the file header for
/// provenance and the spec sections behind every value).
pub const DYNAMICS_RON: &str = include_str!("../data/timestop-dynamics.ron");

/// Roster size: 13 girls (spec §1, census cross-counts;
/// DERIVED split 12 walkers + 1 cashier).
pub const ROSTER_COUNT: usize = 13;
/// Cashiers in the roster: exactly one (spec §1, DERIVED).
pub const CASHIER_COUNT: usize = 1;
/// Girl transition rows in the measured table (spec §4).
pub const GIRL_TRANSITION_COUNT: usize = 6;
/// Nav transition rows in the measured table (spec §5).
pub const NAV_TRANSITION_COUNT: usize = 9;
/// A bound on any single vocabulary list, so a corrupt file
/// cannot allocate unbounded tables. The measured lists run
/// 5–7 entries; the bound is a contract guard, not a claim.
pub const VOCAB_COUNT_MAX: usize = 32;
/// A bound on transition rows in either table.
pub const TRANSITION_COUNT_MAX: usize = 64;

/// One girl state (spec §4 vocabulary, MEASURED). `SawNaked`
/// is declared in the original's vocabulary and unreachable
/// in its transition table — carried, never repaired (§4
/// named gap, §10.8).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum GirlState {
    None,
    Normal,
    NoClothes,
    RunningAway,
    WrongState,
    SawNaked,
}

/// One girl-machine input (spec §4 vocabulary, MEASURED).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GirlInput {
    None,
    NoClothes,
    RunAway,
    SawNaked,
    #[serde(rename = "_wrongGirlState")]
    WrongGirlState,
    Calmed,
}

/// One nav state (spec §5 vocabulary, MEASURED).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum NavState {
    None,
    Random,
    LocationEntering,
    LocationTraversing,
    LocationExiting,
    ShopLooking,
    InQueue,
}

/// One nav-machine input (spec §5 vocabulary, MEASURED).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum NavInput {
    None,
    ToRandom,
    ToLocation,
    ToLocationTraverse,
    ToLocationExit,
    LookShop,
    QueueUp,
}

/// The derived embarrassment scale the resume audit consumes
/// (spec §6 vocabulary, MEASURED; spec §3 audit mapping).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum EmbarrassmentLevel {
    None,
    OnlyPanties,
    WrongState,
    Shoes,
    MajorClothing,
}

/// A roster member's measured role (spec §1, DERIVED split).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum RosterRole {
    Walker,
    Cashier,
}

/// One row of a transition table. `from: None` is the
/// Any-state wildcard (spec §5); the girl table carries no
/// wildcard rows.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition<From, Input, To> {
    pub from: Option<From>,
    pub input: Input,
    pub to: To,
}

/// One roster entry: an estate-original placeholder id and
/// the measured role. No per-girl type is claimed (spec
/// §10.7: the original's assignment is serialized, UNKNOWN).
#[derive(Clone, Debug, PartialEq)]
pub struct RosterEntry {
    pub id: String,
    pub role: RosterRole,
}

/// Typed refusals of the loader and the systems. Every
/// failure of this crate is one of these — never a panic,
/// never a silent default.
#[derive(Debug)]
pub enum TimestopError {
    /// The RON text did not parse as the schema (this
    /// includes unknown enum spellings and unknown fields:
    /// deserialization is deny-by-default).
    Parse(ron::de::SpannedError),
    /// A parsed value fell outside the measured shape.
    Invalid {
        field: &'static str,
        reason: &'static str,
    },
    /// A roster id the table does not carry.
    UnknownGirl { id: String },
    /// A step input (a duration) broke the contract.
    InvalidStep { reason: &'static str },
    /// Time stop toggled before the Watch pickup (spec §2:
    /// `SaveData.HasWatch` must be true).
    WatchMissing,
    /// Time stop toggled while the game pause is up (spec §2:
    /// the toggle is ignored while game-paused; the remake
    /// surfaces the measured ignore as a typed refusal).
    GamePaused,
    /// Time resumed while time was not stopped.
    NotStopped,
    /// A girl interaction attempted while time flows (spec
    /// §3: girls are interactable ONLY in stopped time).
    InteractionWhileFlowing { id: String },
    /// An interaction end attempted with no focus active.
    NoFocus,
    /// A photograph attempted before the Camera pickup
    /// (spec §7).
    CameraMissing,
    /// An artwork photographed twice (spec §7: the unlock
    /// list is Contains-checked, no duplicates).
    AlreadyUnlocked { art_id: i32 },
}

impl std::fmt::Display for TimestopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimestopError::Parse(e) => write!(f, "RON parse failed: {e}"),
            TimestopError::Invalid { field, reason } => {
                write!(f, "field {field} invalid: {reason}")
            }
            TimestopError::UnknownGirl { id } => write!(f, "unknown girl id: {id}"),
            TimestopError::InvalidStep { reason } => write!(f, "invalid step: {reason}"),
            TimestopError::WatchMissing => {
                write!(f, "time stop requires the Watch (HasWatch is false)")
            }
            TimestopError::GamePaused => {
                write!(f, "time stop toggle is ignored while the game is paused")
            }
            TimestopError::NotStopped => write!(f, "time is not stopped"),
            TimestopError::InteractionWhileFlowing { id } => {
                write!(f, "girl {id} is interactable only while time is stopped")
            }
            TimestopError::NoFocus => write!(f, "no girl interaction is in focus"),
            TimestopError::CameraMissing => {
                write!(f, "photography requires the Camera (HasCamera is false)")
            }
            TimestopError::AlreadyUnlocked { art_id } => {
                write!(f, "artwork {art_id} is already unlocked")
            }
        }
    }
}

impl std::error::Error for TimestopError {}

impl From<ron::de::SpannedError> for TimestopError {
    fn from(e: ron::de::SpannedError) -> TimestopError {
        TimestopError::Parse(e)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GirlTransitionRow {
    from: GirlState,
    input: GirlInput,
    to: GirlState,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NavTransitionRow {
    from: Option<NavState>,
    input: NavInput,
    to: NavState,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RosterRow {
    id: String,
    role: RosterRole,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DynamicsFile {
    girl_states: Vec<GirlState>,
    girl_inputs: Vec<GirlInput>,
    girl_initial: GirlState,
    girl_transitions: Vec<GirlTransitionRow>,
    nav_states: Vec<NavState>,
    nav_inputs: Vec<NavInput>,
    nav_initial: NavState,
    nav_transitions: Vec<NavTransitionRow>,
    embarrassment_levels: Vec<EmbarrassmentLevel>,
    audit_inputs: Vec<(EmbarrassmentLevel, Option<GirlInput>)>,
    roster: Vec<RosterRow>,
}

/// The validated measured tables: both transition graphs,
/// the audit mapping, and the roster, exactly as the spec
/// records them.
#[derive(Clone, Debug, PartialEq)]
pub struct Dynamics {
    girl_initial: GirlState,
    girl_transitions: Vec<Transition<GirlState, GirlInput, GirlState>>,
    nav_initial: NavState,
    nav_transitions: Vec<Transition<NavState, NavInput, NavState>>,
    audit_inputs: BTreeMap<EmbarrassmentLevel, Option<GirlInput>>,
    roster: Vec<RosterEntry>,
}

fn invalid(field: &'static str, reason: &'static str) -> TimestopError {
    TimestopError::Invalid { field, reason }
}

fn check_vocab<T: Ord + Copy>(
    field: &'static str,
    got: &[T],
    want: &[T],
) -> Result<(), TimestopError> {
    if got.len() > VOCAB_COUNT_MAX {
        return Err(invalid(field, "vocabulary exceeds the contract bound"));
    }
    let got_set: BTreeSet<T> = got.iter().copied().collect();
    let want_set: BTreeSet<T> = want.iter().copied().collect();
    if got_set.len() != got.len() {
        return Err(invalid(field, "vocabulary carries a duplicate entry"));
    }
    if got_set != want_set {
        return Err(invalid(
            field,
            "vocabulary differs from the measured set in the spec",
        ));
    }
    Ok(())
}

fn check_transitions<From, Input, To>(
    field: &'static str,
    rows: &[Transition<From, Input, To>],
    want_count: usize,
) -> Result<(), TimestopError>
where
    From: Ord + Copy,
    Input: Ord + Copy,
    To: Copy,
{
    if rows.len() > TRANSITION_COUNT_MAX {
        return Err(invalid(
            field,
            "transition table exceeds the contract bound",
        ));
    }
    if rows.len() != want_count {
        return Err(invalid(
            field,
            "transition row count differs from the measured table",
        ));
    }
    let mut keys: BTreeSet<(Option<From>, Input)> = BTreeSet::new();
    for row in rows {
        if !keys.insert((row.from, row.input)) {
            return Err(invalid(
                field,
                "duplicate (from, input) row in the transition table",
            ));
        }
    }
    Ok(())
}

impl Dynamics {
    /// Parse and validate one dynamics RON document. Any
    /// deviation from the measured shape — an unknown enum
    /// spelling, an unknown field, a vocabulary that differs
    /// from the spec's set, a duplicate or missing transition
    /// row, a roster that is not 12 walkers + 1 cashier — is a
    /// typed refusal.
    pub fn load(ron_text: &str) -> Result<Dynamics, TimestopError> {
        let file: DynamicsFile = ron::de::from_str(ron_text)?;

        check_vocab(
            "girl_states",
            &file.girl_states,
            &[
                GirlState::None,
                GirlState::Normal,
                GirlState::NoClothes,
                GirlState::RunningAway,
                GirlState::WrongState,
                GirlState::SawNaked,
            ],
        )?;
        check_vocab(
            "girl_inputs",
            &file.girl_inputs,
            &[
                GirlInput::None,
                GirlInput::NoClothes,
                GirlInput::RunAway,
                GirlInput::SawNaked,
                GirlInput::WrongGirlState,
                GirlInput::Calmed,
            ],
        )?;
        check_vocab(
            "nav_states",
            &file.nav_states,
            &[
                NavState::None,
                NavState::Random,
                NavState::LocationEntering,
                NavState::LocationTraversing,
                NavState::LocationExiting,
                NavState::ShopLooking,
                NavState::InQueue,
            ],
        )?;
        check_vocab(
            "nav_inputs",
            &file.nav_inputs,
            &[
                NavInput::None,
                NavInput::ToRandom,
                NavInput::ToLocation,
                NavInput::ToLocationTraverse,
                NavInput::ToLocationExit,
                NavInput::LookShop,
                NavInput::QueueUp,
            ],
        )?;
        check_vocab(
            "embarrassment_levels",
            &file.embarrassment_levels,
            &[
                EmbarrassmentLevel::None,
                EmbarrassmentLevel::OnlyPanties,
                EmbarrassmentLevel::WrongState,
                EmbarrassmentLevel::Shoes,
                EmbarrassmentLevel::MajorClothing,
            ],
        )?;

        if file.girl_initial != GirlState::Normal {
            return Err(invalid(
                "girl_initial",
                "the measured initial girl state is Normal",
            ));
        }
        if file.nav_initial != NavState::Random {
            return Err(invalid(
                "nav_initial",
                "the measured initial nav state is Random",
            ));
        }

        let girl_transitions: Vec<Transition<GirlState, GirlInput, GirlState>> = file
            .girl_transitions
            .iter()
            .map(|r| Transition {
                from: Some(r.from),
                input: r.input,
                to: r.to,
            })
            .collect();
        check_transitions("girl_transitions", &girl_transitions, GIRL_TRANSITION_COUNT)?;
        if girl_transitions
            .iter()
            .any(|r| r.input == GirlInput::None || r.to == GirlState::None)
        {
            return Err(invalid(
                "girl_transitions",
                "the measured girl table has no None input or target rows",
            ));
        }

        let nav_transitions: Vec<Transition<NavState, NavInput, NavState>> = file
            .nav_transitions
            .iter()
            .map(|r| Transition {
                from: r.from,
                input: r.input,
                to: r.to,
            })
            .collect();
        check_transitions("nav_transitions", &nav_transitions, NAV_TRANSITION_COUNT)?;
        let wildcards = nav_transitions.iter().filter(|r| r.from.is_none()).count();
        if wildcards != 1 {
            return Err(invalid(
                "nav_transitions",
                "the measured nav table carries exactly one Any-state wildcard row",
            ));
        }

        // The audit mapping is total over the measured scale
        // (spec §3): every level names its input or explicitly
        // names none, exactly once.
        let mut audit_inputs: BTreeMap<EmbarrassmentLevel, Option<GirlInput>> = BTreeMap::new();
        for (level, input) in &file.audit_inputs {
            if audit_inputs.insert(*level, *input).is_some() {
                return Err(invalid(
                    "audit_inputs",
                    "an embarrassment level carries two audit entries",
                ));
            }
        }
        if audit_inputs.len() != file.embarrassment_levels.len() {
            return Err(invalid(
                "audit_inputs",
                "the audit mapping must cover every embarrassment level",
            ));
        }
        for level in &file.embarrassment_levels {
            if !audit_inputs.contains_key(level) {
                return Err(invalid(
                    "audit_inputs",
                    "an embarrassment level has no audit entry",
                ));
            }
        }
        if audit_inputs[&EmbarrassmentLevel::MajorClothing] != Some(GirlInput::NoClothes)
            || audit_inputs[&EmbarrassmentLevel::WrongState] != Some(GirlInput::WrongGirlState)
        {
            return Err(invalid(
                "audit_inputs",
                "the measured audit routes MajorClothing to NoClothes and WrongState to _wrongGirlState",
            ));
        }

        if file.roster.len() != ROSTER_COUNT {
            return Err(invalid("roster", "the measured roster carries 13 members"));
        }
        let mut ids: BTreeSet<&str> = BTreeSet::new();
        let mut cashiers = 0usize;
        for row in &file.roster {
            if row.id.is_empty() || !ids.insert(row.id.as_str()) {
                return Err(invalid("roster", "roster ids must be non-empty and unique"));
            }
            if row.role == RosterRole::Cashier {
                cashiers += 1;
            }
        }
        if cashiers != CASHIER_COUNT {
            return Err(invalid(
                "roster",
                "the roster carries exactly one cashier (D-1105)",
            ));
        }

        Ok(Dynamics {
            girl_initial: file.girl_initial,
            girl_transitions,
            nav_initial: file.nav_initial,
            nav_transitions,
            audit_inputs,
            roster: file
                .roster
                .iter()
                .map(|r| RosterEntry {
                    id: r.id.clone(),
                    role: r.role,
                })
                .collect(),
        })
    }

    /// The girl machine's initial state (spec §4).
    pub fn girl_initial(&self) -> GirlState {
        self.girl_initial
    }

    /// The girl transition table (spec §4, 6 rows).
    pub fn girl_transitions(&self) -> &[Transition<GirlState, GirlInput, GirlState>] {
        &self.girl_transitions
    }

    /// Resolve one girl-machine step against the table. A
    /// (state, input) pair with no row is not an error — the
    /// measured machine simply does not move — so this returns
    /// the target state or `None` for no transition.
    pub fn girl_step(&self, from: GirlState, input: GirlInput) -> Option<GirlState> {
        self.girl_transitions
            .iter()
            .find(|r| r.from == Some(from) && r.input == input)
            .map(|r| r.to)
    }

    /// The nav machine's initial state for a walking girl
    /// (spec §5).
    pub fn nav_initial(&self) -> NavState {
        self.nav_initial
    }

    /// The nav transition table (spec §5, 9 rows).
    pub fn nav_transitions(&self) -> &[Transition<NavState, NavInput, NavState>] {
        &self.nav_transitions
    }

    /// Resolve one nav-machine step: an exact (state, input)
    /// row wins; the Any-state wildcard row answers otherwise
    /// (spec §5). `None` means no row answers.
    pub fn nav_step(&self, from: NavState, input: NavInput) -> Option<NavState> {
        if let Some(row) = self
            .nav_transitions
            .iter()
            .find(|r| r.from == Some(from) && r.input == input)
        {
            return Some(row.to);
        }
        self.nav_transitions
            .iter()
            .find(|r| r.from.is_none() && r.input == input)
            .map(|r| r.to)
    }

    /// The audit input a girl at `level` receives at resume,
    /// or `None` for no input (spec §3, total over the scale).
    pub fn audit_input(&self, level: EmbarrassmentLevel) -> Option<GirlInput> {
        self.audit_inputs.get(&level).copied().flatten()
    }

    /// The roster as data (spec §1; D-1105).
    pub fn roster(&self) -> &[RosterEntry] {
        &self.roster
    }
}

/// Load the embedded measured tables.
pub fn measured_dynamics() -> Result<Dynamics, TimestopError> {
    Dynamics::load(DYNAMICS_RON)
}
