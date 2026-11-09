//! torogao-timestop: the Time Stop systems as Torogao
//! data + systems.
//!
//! Clean-room provenance. The systems were studied from the
//! estate's own copy of TimeStop1 (Unity 2022.1.23f1 Mono)
//! under the fathom clean-room discipline: behavior and
//! structure were measured from the IL, never copied. The
//! record of record is `games/time-stop-dynamics.md` in the
//! Lane D programme tree (`nsfw-remakes-20261108/`), with
//! its S2b sibling `games/time-stop-animation.md`. No
//! studied game's assets, art, text, names, or code appear
//! in this crate: what transfers is two code-built state
//! machines re-expressed as data tables, an event-subscription
//! freeze gate, an audit-at-resume consequence pass, and a
//! seven-field save record.
//!
//! The crate is a systems skeleton for the Torogao remake
//! programme (PIPELINE.md in `nsfw-remakes-20261108/`): pure,
//! deterministic functions over explicit inputs. No wall
//! clock, no RNG, no I/O beyond the RON loader in `dynamics`.
//! Content is estate-original placeholder data only; the
//! full content pass is an Owner-gated commission (S5), and
//! the first-person sandbox, ambient life, and pose/clothing
//! surface are later stages, not this crate.

#![forbid(unsafe_code)]

pub mod dynamics;
pub mod session;
pub mod trajectory;

pub use dynamics::{
    DYNAMICS_RON, Dynamics, EmbarrassmentLevel, GirlInput, GirlState, NavInput, NavState,
    RosterEntry, RosterRole, TimestopError, Transition, measured_dynamics,
};
pub use session::{AuditEffect, AuditReport, Girl, PausableTimer, SaveData, Session};
pub use trajectory::{FixtureRun, run_fixture, run_measured_fixture};
