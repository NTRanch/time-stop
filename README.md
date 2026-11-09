# time-stop — torogao-timestop

A clean-room, from-measurement reimplementation of the systems core of
**Time Stop** (Unity 2022.1.23f1, Mono), built for the Torogao remake
programme by Nice Thoughts Ranch. No assets, text, or code from the
studied game are included: every table in
`data/timestop-dynamics.ron` was measured from the original's
behaviour and rebuilt as estate-authored data and Rust systems, with
estate-original placeholder content only.

## What the crate models

- Two layered per-girl state machines carried as data tables over the
  measured vocabularies: the 6-transition reaction graph and the
  9-row navigation table (exact row wins over the Any-state wildcard).
  One measured transition is declared in the original's vocabulary
  but unreachable in its graph; it is carried as measured and its
  unreachability is **proven by an exhaustive test**, not repaired.
- The time-stop gate as an event-subscription boolean — not a
  timescale. Characters are interactable only while stopped, and the
  toggle is refused without the required item or while paused.
- A consequence audit that runs exactly once per stop→resume edge.
- The seven-field save record in deterministic RON, round-trip
  byte-equal under test.
- The reduced-first roster as data: 13 members (12 walking + 1
  stationary cashier), validated at load; unknown spellings, fields,
  or row counts are typed refusals.

## Dual-target law

The remake programme ships every title on native Linux **and** Android
from this one crate plus its data file. Platform shells bind to the
crate; the systems and data live here only.

## Determinism

Sessions are deterministic: a fixed input stream replays to a fixed
trajectory hash (FNV-1a 64 over per-step state words). The fixture is
exercised by the test suite, and the same hash must match on Linux and
Android (the programme's S7 gate).

## Build and test

```sh
cargo build
cargo test
cargo run --bin timestop-smoke   # scripted cycle; prints the smoke line
```

## License

BSD 2-Clause. See `LICENSE`. The studied game remains the property of
its respective developer; this repository is an independent
reimplementation of measured behaviour.
