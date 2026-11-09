//! Integration tests for torogao-timestop, split half
//! validation (the measured behaviour, exercised) and half
//! adversarial (refusals, tampered data, unreachable states,
//! double-audit attempts). Every expected value traces to
//! `games/time-stop-dynamics.md` (cited per test).

use std::collections::BTreeSet;

use torogao_timestop::{
    DYNAMICS_RON, Dynamics, EmbarrassmentLevel, GirlInput, GirlState, NavInput, NavState,
    PausableTimer, RosterRole, SaveData, Session, TimestopError, measured_dynamics,
    run_measured_fixture,
};

fn session() -> Session {
    Session::new(measured_dynamics().expect("measured tables load"))
}

// ----------------------------------------------------------------
// Validation (8)
// ----------------------------------------------------------------

/// Spec §1/§4/§5/D-1105: the shipped tables are the measured
/// tables — counts, vocabularies, roster shape.
#[test]
fn measured_tables_match_the_spec_shape() {
    let dynamics = measured_dynamics().expect("measured tables load");
    assert_eq!(dynamics.girl_transitions().len(), 6);
    assert_eq!(dynamics.nav_transitions().len(), 9);
    assert_eq!(dynamics.roster().len(), 13);
    let cashiers = dynamics
        .roster()
        .iter()
        .filter(|r| r.role == RosterRole::Cashier)
        .count();
    assert_eq!(cashiers, 1);
    assert_eq!(dynamics.girl_initial(), GirlState::Normal);
    assert_eq!(dynamics.nav_initial(), NavState::Random);
    // The audit mapping is the measured one (spec §3).
    assert_eq!(
        dynamics.audit_input(EmbarrassmentLevel::MajorClothing),
        Some(GirlInput::NoClothes)
    );
    assert_eq!(
        dynamics.audit_input(EmbarrassmentLevel::WrongState),
        Some(GirlInput::WrongGirlState)
    );
    assert_eq!(dynamics.audit_input(EmbarrassmentLevel::Shoes), None);
    assert_eq!(dynamics.audit_input(EmbarrassmentLevel::OnlyPanties), None);
    assert_eq!(dynamics.audit_input(EmbarrassmentLevel::None), None);
}

/// Spec §4: the girl graph walks its measured cycles.
#[test]
fn girl_graph_walks_the_measured_cycles() {
    let mut s = session();
    let id = "walker-01";
    assert_eq!(
        s.apply_girl_input(id, GirlInput::NoClothes).expect("step"),
        Some((GirlState::Normal, GirlState::NoClothes))
    );
    assert_eq!(
        s.apply_girl_input(id, GirlInput::RunAway).expect("step"),
        Some((GirlState::NoClothes, GirlState::RunningAway))
    );
    assert_eq!(
        s.apply_girl_input(id, GirlInput::Calmed).expect("step"),
        Some((GirlState::RunningAway, GirlState::Normal))
    );
    assert_eq!(
        s.apply_girl_input(id, GirlInput::WrongGirlState)
            .expect("step"),
        Some((GirlState::Normal, GirlState::WrongState))
    );
    assert_eq!(
        s.apply_girl_input(id, GirlInput::Calmed).expect("step"),
        Some((GirlState::WrongState, GirlState::Normal))
    );
    // NoClothes + Calmed is the other return edge (row 4).
    s.apply_girl_input(id, GirlInput::NoClothes).expect("step");
    assert_eq!(
        s.apply_girl_input(id, GirlInput::Calmed).expect("step"),
        Some((GirlState::NoClothes, GirlState::Normal))
    );
    // A pair with no row does not move and is not an error.
    assert_eq!(
        s.apply_girl_input(id, GirlInput::RunAway).expect("step"),
        None
    );
    assert_eq!(s.girl(id).expect("girl").girl_state(), GirlState::Normal);
}

/// Spec §5: the nav graph walks the location cycle, the
/// shop detour, the queue, and the Any-state wildcard.
#[test]
fn nav_graph_walks_cycle_detour_queue_and_wildcard() {
    let mut s = session();
    let id = "walker-03";
    let walk = |s: &mut Session, input: NavInput| s.apply_nav_input(id, input).expect("nav step");
    assert_eq!(
        walk(&mut s, NavInput::ToLocation),
        Some((NavState::Random, NavState::LocationEntering))
    );
    assert_eq!(
        walk(&mut s, NavInput::ToLocationTraverse),
        Some((NavState::LocationEntering, NavState::LocationTraversing))
    );
    assert_eq!(
        walk(&mut s, NavInput::LookShop),
        Some((NavState::LocationTraversing, NavState::ShopLooking))
    );
    assert_eq!(
        walk(&mut s, NavInput::ToLocationTraverse),
        Some((NavState::ShopLooking, NavState::LocationTraversing))
    );
    assert_eq!(
        walk(&mut s, NavInput::QueueUp),
        Some((NavState::LocationTraversing, NavState::InQueue))
    );
    assert_eq!(
        walk(&mut s, NavInput::ToLocationExit),
        Some((NavState::InQueue, NavState::LocationExiting))
    );
    // The wildcard row answers ToRandom from a state with no
    // exact row for it (ShopLooking): re-enter and detour,
    // then reset from the detour state.
    walk(&mut s, NavInput::ToRandom);
    walk(&mut s, NavInput::ToLocation);
    walk(&mut s, NavInput::ToLocationTraverse);
    walk(&mut s, NavInput::LookShop);
    assert_eq!(
        walk(&mut s, NavInput::ToRandom),
        Some((NavState::ShopLooking, NavState::Random))
    );
}

/// Spec §2/§3: the gate — interaction is refused while time
/// flows and allowed while stopped; focus tracks the girl.
#[test]
fn interaction_gate_follows_the_freeze() {
    let mut s = session();
    s.grant_watch();
    assert!(matches!(
        s.interact("walker-01"),
        Err(TimestopError::InteractionWhileFlowing { .. })
    ));
    s.toggle_time_stop().expect("stop");
    assert!(s.time_stopped());
    s.interact("walker-01").expect("interact while stopped");
    assert_eq!(s.focused(), Some("walker-01"));
    s.end_interaction().expect("end focus");
    assert_eq!(s.focused(), None);
}

/// Spec §3: stop → interact → resume runs the audit, and
/// the audit moves exactly the girls the mapping names.
#[test]
fn stop_interact_resume_audit_end_to_end() {
    let mut s = session();
    s.grant_watch();
    s.toggle_time_stop().expect("stop");
    s.interact("walker-01").expect("focus 1");
    s.set_embarrassment("walker-01", EmbarrassmentLevel::MajorClothing)
        .expect("level 1");
    s.interact("walker-02").expect("focus 2");
    s.set_embarrassment("walker-02", EmbarrassmentLevel::WrongState)
        .expect("level 2");
    s.set_embarrassment("walker-04", EmbarrassmentLevel::OnlyPanties)
        .expect("level 4");
    let report = s.resume_time().expect("resume");
    assert_eq!(s.audit_runs(), 1);
    assert_eq!(report.effects.len(), 2);
    assert_eq!(report.effects[0].girl_id, "walker-01");
    assert_eq!(report.effects[0].input, GirlInput::NoClothes);
    assert_eq!(report.effects[0].to, GirlState::NoClothes);
    assert_eq!(report.effects[1].girl_id, "walker-02");
    assert_eq!(report.effects[1].input, GirlInput::WrongGirlState);
    assert_eq!(report.effects[1].to, GirlState::WrongState);
    // OnlyPanties maps to no input: walker-04 did not move.
    assert_eq!(
        s.girl("walker-04").expect("girl").girl_state(),
        GirlState::Normal
    );
    // Resume clears the focus (the interaction is over).
    assert_eq!(s.focused(), None);
    assert!(!s.time_stopped());
}

/// Spec §7: the seven-field save record round-trips through
/// its deterministic RON form, byte for byte.
#[test]
fn save_record_round_trips_all_seven_fields() {
    let save = SaveData {
        has_watch: true,
        has_camera: true,
        sounds_on: false,
        music_on: true,
        showed_initial_hint: true,
        showed_cashier_hint: false,
        unlocked_arts: vec![3, 17, 42],
    };
    let text = save.to_ron().expect("serialize");
    let back = SaveData::from_ron(&text).expect("parse");
    assert_eq!(back, save);
    assert_eq!(back.to_ron().expect("re-serialize"), text);
    // A session resumed from the record carries its flags.
    let s = Session::with_save(measured_dynamics().expect("tables"), back);
    assert!(s.save().has_watch && s.save().has_camera);
    assert_eq!(s.save().unlocked_arts, vec![3, 17, 42]);
}

/// The fixture: deterministic across runs, and its end
/// state is the measured audit outcome (golden-value form).
#[test]
fn fixture_is_deterministic_with_the_measured_end_state() {
    let a = run_measured_fixture().expect("fixture a");
    let b = run_measured_fixture().expect("fixture b");
    assert_eq!(a, b);
    assert_eq!(a.steps_executed, 11);
    assert_eq!(a.audit_runs, 2);
    assert_eq!(a.audit_effects, 2);
    assert_eq!(a.first_girl_state, GirlState::NoClothes);
    assert_eq!(a.second_girl_state, GirlState::WrongState);
}

/// Spec §2/animation §3: the pausable timer and the
/// animation clock scale obey the freeze, and only it.
#[test]
fn timer_and_anim_scale_obey_the_freeze() {
    let mut s = session();
    s.grant_watch();
    let mut timer = PausableTimer::new();
    assert_eq!(s.anim_clock_scale(), 1.0);
    s.advance_timer(&mut timer, 0.5).expect("flow step");
    assert_eq!(timer.accumulated_seconds(), 0.5);
    s.toggle_time_stop().expect("stop");
    assert_eq!(s.anim_clock_scale(), 0.0);
    s.advance_timer(&mut timer, 0.5).expect("stopped step");
    assert_eq!(timer.accumulated_seconds(), 0.5);
    s.resume_time().expect("resume");
    s.advance_timer(&mut timer, 0.25).expect("flow step");
    assert_eq!(timer.accumulated_seconds(), 0.75);
}

// ----------------------------------------------------------------
// Adversarial (8)
// ----------------------------------------------------------------

/// Spec §4/§10.8 (D-1105 behaviour obligation): SawNaked is
/// declared in both vocabularies and UNREACHABLE — proven
/// by exhaustive reachability over the measured table, not
/// by inspection. If a future table edit gives it a row,
/// this test fails: the graph is never silently repaired.
#[test]
fn sawnaked_is_declared_and_provably_unreachable() {
    let dynamics = measured_dynamics().expect("measured tables load");
    // No row consumes the SawNaked input, no row produces
    // the SawNaked state.
    for row in dynamics.girl_transitions() {
        assert_ne!(row.input, GirlInput::SawNaked);
        assert_ne!(row.to, GirlState::SawNaked);
    }
    // Exhaustive BFS from the initial state over every input
    // in the vocabulary: the reachable set is exactly the
    // four live states.
    let inputs = [
        GirlInput::None,
        GirlInput::NoClothes,
        GirlInput::RunAway,
        GirlInput::SawNaked,
        GirlInput::WrongGirlState,
        GirlInput::Calmed,
    ];
    let mut reachable: BTreeSet<GirlState> = BTreeSet::new();
    let mut frontier = vec![dynamics.girl_initial()];
    reachable.insert(dynamics.girl_initial());
    while let Some(state) = frontier.pop() {
        for input in inputs {
            if let Some(to) = dynamics.girl_step(state, input) {
                if reachable.insert(to) {
                    frontier.push(to);
                }
            }
        }
    }
    assert_eq!(
        reachable,
        BTreeSet::from([
            GirlState::Normal,
            GirlState::NoClothes,
            GirlState::RunningAway,
            GirlState::WrongState,
        ])
    );
    assert!(!reachable.contains(&GirlState::SawNaked));
}

/// Spec §3: the audit runs EXACTLY once per resume — a
/// resume attempted while flowing is refused and does not
/// re-run it, and a fresh stop/resume pair adds exactly one.
#[test]
fn audit_runs_exactly_once_per_resume() {
    let mut s = session();
    s.grant_watch();
    assert!(matches!(s.resume_time(), Err(TimestopError::NotStopped)));
    assert_eq!(s.audit_runs(), 0);
    s.toggle_time_stop().expect("stop");
    s.resume_time().expect("resume");
    assert_eq!(s.audit_runs(), 1);
    // Second resume with no intervening stop: refused, and
    // the counter does not move.
    assert!(matches!(s.resume_time(), Err(TimestopError::NotStopped)));
    assert_eq!(s.audit_runs(), 1);
    s.toggle_time_stop().expect("stop again");
    let report = s.toggle_time_stop().expect("resume via toggle");
    assert!(report.is_some());
    assert_eq!(s.audit_runs(), 2);
}

/// Spec §2: the toggle's two measured gates refuse —
/// no Watch, and game-paused.
#[test]
fn toggle_refused_without_watch_and_while_paused() {
    let mut s = session();
    assert!(matches!(
        s.toggle_time_stop(),
        Err(TimestopError::WatchMissing)
    ));
    assert!(!s.time_stopped());
    s.grant_watch();
    s.set_game_paused(true);
    assert!(matches!(
        s.toggle_time_stop(),
        Err(TimestopError::GamePaused)
    ));
    assert!(!s.time_stopped());
    s.set_game_paused(false);
    s.toggle_time_stop().expect("stop once unpaused");
    assert!(s.time_stopped());
}

/// Loader refusal: an unknown state spelling in a
/// transition row is a parse refusal, never a default.
#[test]
fn loader_refuses_an_unknown_state_spelling() {
    let tampered = DYNAMICS_RON.replace(
        "(from: Normal, input: NoClothes, to: NoClothes)",
        "(from: Normal, input: NoClothes, to: Undressed)",
    );
    assert_ne!(tampered, DYNAMICS_RON);
    assert!(Dynamics::load(&tampered).is_err());
}

/// Loader refusal: an unknown top-level field and a
/// duplicated transition row each refuse the document.
#[test]
fn loader_refuses_unknown_fields_and_duplicate_rows() {
    let with_field = DYNAMICS_RON.replace(
        "    girl_states:",
        "    walking_speed: 3.5,\n    girl_states:",
    );
    assert!(Dynamics::load(&with_field).is_err());
    let duplicated = DYNAMICS_RON.replace(
        "(from: WrongState, input: Calmed, to: Normal),",
        "(from: WrongState, input: Calmed, to: Normal),\n        (from: Normal, input: NoClothes, to: NoClothes),",
    );
    assert!(Dynamics::load(&duplicated).is_err());
}

/// Loader refusal (D-1105): a roster with the cashier
/// dropped, or a second cashier added, refuses.
#[test]
fn loader_refuses_a_roster_without_exactly_one_cashier() {
    let no_cashier = DYNAMICS_RON.replace(
        "(id: \"cashier\", role: Cashier)",
        "(id: \"cashier\", role: Walker)",
    );
    assert!(Dynamics::load(&no_cashier).is_err());
    let two_cashiers = DYNAMICS_RON.replace(
        "(id: \"walker-12\", role: Walker)",
        "(id: \"walker-12\", role: Cashier)",
    );
    assert!(Dynamics::load(&two_cashiers).is_err());
}

/// Spec §7 adversarial: the save codec refuses an unknown
/// field and a duplicate unlock id; the session refuses a
/// duplicate photograph and a photograph with no Camera.
#[test]
fn save_and_gallery_refuse_duplicates_and_unknown_fields() {
    let mut s = session();
    assert!(matches!(s.photograph(7), Err(TimestopError::CameraMissing)));
    s.grant_camera();
    s.photograph(7).expect("first photograph");
    assert!(matches!(
        s.photograph(7),
        Err(TimestopError::AlreadyUnlocked { art_id: 7 })
    ));
    let dup_text = "(has_watch:true,has_camera:true,sounds_on:true,music_on:true,showed_initial_hint:false,showed_cashier_hint:false,unlocked_arts:[7,7])";
    assert!(SaveData::from_ron(dup_text).is_err());
    let unknown_text = "(has_watch:true,has_camera:true,sounds_on:true,music_on:true,showed_initial_hint:false,showed_cashier_hint:false,unlocked_arts:[],has_jetpack:true)";
    assert!(SaveData::from_ron(unknown_text).is_err());
}

/// Adversarial edges: interaction stays refused after a
/// resume (yesterday's focus grants nothing), unknown girls
/// refuse everywhere, the cashier refuses nav inputs, and a
/// non-finite timer step refuses.
#[test]
fn refused_edges_stay_refused() {
    let mut s = session();
    s.grant_watch();
    s.toggle_time_stop().expect("stop");
    s.interact("cashier").expect("the cashier is a girl too");
    s.resume_time().expect("resume");
    assert!(matches!(
        s.interact("cashier"),
        Err(TimestopError::InteractionWhileFlowing { .. })
    ));
    assert!(matches!(
        s.interact("walker-99"),
        Err(TimestopError::UnknownGirl { .. })
    ));
    assert!(s.apply_nav_input("cashier", NavInput::ToLocation).is_err());
    let mut timer = PausableTimer::new();
    assert!(s.advance_timer(&mut timer, f64::NAN).is_err());
    assert!(s.advance_timer(&mut timer, -1.0).is_err());
    assert_eq!(timer.accumulated_seconds(), 0.0);
}
