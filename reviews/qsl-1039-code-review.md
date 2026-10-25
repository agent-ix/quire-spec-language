---
id: SR-17090
title: "Code review of PR #1490: ModelLetters::atom fault channel, observation caching and metering (QSL-1039), code-review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8508046d5213213188fbcc01d9f417908e3dc739; git diff origin/main...8508046d5 (PR #1490, branch qsl-1039-letters-fault): qsl-eval/src/model_subject.rs, qsl-eval/src/simulation/model/letters.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/src/model_check/liveness.rs, qsl-analyze/src/analyze/items.rs, qsl-replay/src/certificate/closure.rs, qsl-analyze/tests/it/*.rs (18 files); context qsl-eval/src/simulation/model/walk.rs, qsl-eval/src/simulation/model/state_predicate.rs, qsl-eval/src/simulation/key.rs, qsl-replay/src/statistical/settle.rs, qsl-replay/src/certificate/check.rs, qsl-replay/src/model_check/replay.rs"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-380
    type: reviews
---
## Summary

Ticket: QSL-1039. PR: quire-spec-language#1490, reviewed at
8508046d5213213188fbcc01d9f417908e3dc739. The current head 9426259936b2 only
merges main into the branch: every PR file is byte-identical to 8508046d5. The
PR was merged as dd0fb3119 (2026-10-25T05:52Z) while this review was running,
so every finding below applies to main and needs a follow-up PR. Method: code-review with the
rust-review lane folded in.

The PR changes `ModelLetters::atom` to return `Result<AtomReading, AtomStop>`
(`Incomplete`, `Refused(ClauseOutcome)`, `Unobservable(ExpansionStop)`,
`Fault(InternalFault)`). It removes every panic from `ClauseLetters`, maps the
new stops in EN-1 safety (check.rs), liveness, the closure certificate checker
and replay (`From<AtomStop> for LetterStop`), and adds a settlement in
`analyze` (items.rs). It also caches the last observed state by its state-key
digest and charges one `collection-visit` per object on every read
(SR-16618 FND-005).

Checked and clean:

- Panic surface. `letters.rs` and `model_subject.rs` contain no `panic!`,
  `unwrap`, `expect` or `unreachable!` on a production path. The one remaining
  panicking API is the `RefCell` borrow in `observe`; the shared borrow is a
  temporary dropped at the end of its `let` statement, before `borrow_mut`, and
  nothing on the evaluation path re-enters the letters, so it cannot panic.
  `ClauseLetters` has the only production `ModelLetters` impl. The compiler
  enforces that every caller of `atom` was converted (check.rs, liveness.rs,
  closure.rs, replay.rs, and 18 test impls).
- Soundness. No `AtomStop` becomes a reading. check.rs and liveness.rs turn
  each stop into `Halt::Refusal` or a meter halt (liveness `ending` returns
  `Err(refusal)`). The closure checker returns a fault or an expansion halt.
  Replay returns `LetterStop::Fault`, which FR-127 settles `failed`. So a
  refused, unobservable or faulted atom is never reported as holds or violated.
- Cache staleness. The key is the FR-101 `quire.simulation.state-key/v1`
  SHA-256 digest over the canonical encoding of `state.state`. That is the
  input of both `observation()` (whose document is derived from the same
  digest) and `rows_of`. `ModelSimState` is borrowed immutably for the whole
  read and the model is fixed for the letters' lifetime, so an entry is served
  only for a state with the same key, the identity exploration already
  deduplicates on. A collision would need a SHA-256 collision. Reviewer probe:
  two initial states, A holding `l` and `s` and B holding `l` only, are read
  A, B, A, B, B through one `ClauseLetters`. The probe reads `holds(x = x)` for
  instance `s`, which is `True` at A and `absent-key` undefined at B. Each read
  is compared, as (reading, work units charged), against a fresh
  `ClauseLetters`, and `observations()` must be 4. The probe passes on the
  head. Under mutation M1 (cache ignores the digest) it fails: B reads `True`
  with A's charge, which is a wrong reading.
- Metering parity. The charge loop runs after the cached and the uncached
  branch alike, so a read charges `rows.len()` collection visits either way.
- Test impls. The 18 test `ModelLetters` impls change only their return type
  and imports, plus one `Err(stop)` that became `Err(stop.into())`. No
  assertion line changed.
- No `#[allow]` was added and no `simulation/protocol/*` file was edited. The
  diff does not touch `qsl-replay/src/hybrid/process_request.rs`, so its
  string-edge hits are baseline.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `analyze` settles a refused `holds` atom whose refusal is a kernel refusal with its own catalog code as `failed`/`runtime_invariant`/`established-invariant-broken` and drops the code. The PR's own headline case, an FR-380 population bound exceeded (`KernelRefused(CardinalityOutOfBound)`, which the PR's test asserts), therefore settles as an engine invariant break instead of `cardinality_out_of_bound`/`above-maximum`. FR-281's catch-all row requires "failed with its FR-272 diagnostic". FR-203's table (row `PredicateRefused(KernelRefused(r))`) and FR-192's `predicate_refused` (qsl-replay/src/statistical/settle.rs:261-279) settle the same `ClauseOutcome` with the refusal's own code and cause. The claimed items.rs precedent covers refusal types that carry no catalog code; `KernelRefused` carries one (`Refusal::code()`/`cause()` return `Some` for `CardinalityOutOfBound`). | qsl-analyze/src/analyze/items.rs:150-157 |
| FND-002 | medium | The new observation charge (one `collection-visit` per object of the state, on every atom read) has no spec basis. ADR-014 TR-5 charges per (temporal node, position) visit, and the observe-once precedent `state_predicates` charges no observation at all. EN-1 gives each atom read a fresh `Meter::new(self.meter)`, so a state holding more objects than that per-evaluation budget now stops every read `Incomplete` (settled `incomplete`, `accounting.work_units`), where on main it read normally. This changes when a check stops and needs a spec ruling and an AC (FR-125 or an ADR-014 TR-5 amendment). | qsl-eval/src/simulation/model/letters.rs:183-215 |
| FND-003 | medium | The cache tests cover only re-reads of one state. Mutating the cache to serve the last observation for any state (`.filter(\|_\| true)` in place of the digest comparison) passes all four new tests and the other nine `model_instances` tests (reviewer mutation M1). Under M1 such a cache serves a wrong reading. A changed-state test can be built: two initial snapshots, one holding `l` and `s` and one holding only `l`, with `holds(x = x)` read for instance `s` in the order A, B, A, B, B. The reading is `True` at A and `absent-key` at B. Each read is compared against a fresh `ClauseLetters` (reading and charge), and `observations()` must be 4. The reviewer's probe passes on the head; under M1 it fails because B reads `True`. | qsl-analyze/tests/it/model_instances.rs:1188-1255 |
| FND-004 | low | The closure checker replaces the evaluator's own `InternalFault` (stage and invariant) with `a-holds-atom-reads`, and drops the refusal's `ClauseOutcome` behind `a-holds-atom-is-not-refused`. `From<AtomStop> for LetterStop` drops the `ClauseOutcome` and the `ExpansionStop` the same way. The settlement is still a fault, so nothing unsound results, but the diagnostic that would locate the defect is lost. | qsl-replay/src/certificate/closure.rs:1141-1142; qsl-eval/src/model_subject.rs:105-121 |
| FND-005 | low | `StateUnobservable(stop)` settles `failed(BROKEN)` whatever the stop's `cause` and `limit` are. FR-203-AC-6, liveness `ending` and closure `expansion_halt` all settle an expansion stop by its limit, or by its own cause when it names none. Today `observe` only produces `stop(BROKEN)`, so no result is wrong yet. But a future `rows_of` or observation stop that names a limit would settle `failed` instead of `incomplete`. | qsl-analyze/src/analyze/items.rs:155-157; qsl-eval/src/simulation/model/letters.rs:187,201 |

## Verdict

Changes requested; the PR is already merged, so they go in a follow-up. The core goal is met: the seam has no panics, every caller
is converted, and no refusal or fault can come out as holds or violated. The
cache is keyed soundly and charges the same whether it hits or misses.

FND-001 must be fixed in the follow-up. It is not a missing spec ruling: FR-281's
last row together with FR-203/FR-192's identical `ClauseOutcome` rule already
define it. Mirror `predicate_refused`: `KernelRefused(r)` with a code and cause
settles `failed` with that code; `CheckedInvariant`, `NotBoolean` and the
undefined outcomes settle `BROKEN`. Add a settlement test (see SR-17091).

FND-003 should be fixed in the follow-up with a changed-state test. The
reviewer's probe can be used as-is: two initial snapshots, `holds(x = x)`
for instance `s`.

FND-002 needs a spec ruling before the charge rule is relied on. Either get the
ruling and an AC, or remove the observation charge until a ticket carries
the ruling.

On the settlement choices the coder raised:

- `FamilyRefused(code)` -> `failed(code)`: correct (FR-203, FR-192, FR-281).
- Other refusals -> `BROKEN`: correct only for `CheckedInvariant` and
  `NotBoolean`; wrong for `KernelRefused` with a code (FND-001).
- `StateUnobservable` -> `BROKEN`: acceptable today, latent (FND-005).
- Replay `LetterStop::Fault`: sound. The engine reads the same deterministic
  letters at the same states, so a refusal seen first on replay is an
  invariant break, which FR-127 settles `failed`. No AC states it, so it
  should be covered by the AC SR-17091 FND-001 asks for.
- Closure fault and expansion halt: sound; diagnostics are lost (FND-004).

Gates, run by the reviewer at 8508046d5 with
CARGO_TARGET_DIR=~/.cache/reviewer-pr1490/target, every cargo call under
`flock /tmp/qsl-cargo.lock`:

- `cargo fmt --all --check`: exit 0.
- `cargo clippy -p qsl-eval -p qsl-analyze -p qsl-replay --all-targets -- -D warnings`: exit 0.
- `cargo test -p qsl-eval -p qsl-analyze -p qsl-replay --no-fail-fast`: exit 0,
  0 failures (qsl-analyze `it` 769 passed).

Reviewer mutations, each run on a scratch copy of the head against the 13
`model_instances` tests plus the reviewer probe:

- M1, digest comparison replaced with `.filter(|_| true)`: only the probe
  fails.
- M2, observation charge removed: `the_state_observation_is_charged_to_the_meter_on_every_read`
  fails.
- M3, `Err(AtomStop::Refused(outcome))` replaced with `Ok(AtomReading::False)`:
  `a_refused_holds_evaluation_is_a_refusal_of_the_atom_and_of_the_check` fails.
