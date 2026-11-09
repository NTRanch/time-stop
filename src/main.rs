//! timestop-smoke: a deterministic end-to-end stop /
//! interact / resume cycle of the Time Stop systems on the
//! measured tables — the canonical fixture from the crate's
//! `trajectory` module (Watch grant, stop, two focused
//! girls, resume audit, a nav cycle, a second stop/resume).
//! Prints the outcome and the fixture's trajectory hash;
//! exit 0 on a completed run, 1 on refusal. This is the
//! skeleton's Linux-native proof artifact, not a game. The
//! printed hash is the S7 cross-platform comparison value.

use torogao_timestop::run_measured_fixture;

fn main() {
    let code = run();
    std::process::exit(code);
}

fn run() -> i32 {
    match run_measured_fixture() {
        Ok(run) => {
            println!(
                "timestop-smoke: success=true steps={} audit_runs={} audit_effects={} walker01={:?} walker02={:?} hash={:016x}",
                run.steps_executed,
                run.audit_runs,
                run.audit_effects,
                run.first_girl_state,
                run.second_girl_state,
                run.trajectory_hash
            );
            0
        }
        Err(e) => {
            eprintln!("timestop-smoke: {e}");
            1
        }
    }
}
