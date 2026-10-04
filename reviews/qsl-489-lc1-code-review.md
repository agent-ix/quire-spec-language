---
id: SR-1278
title: "Code review of quire-spec-language LC1: cancel at every charge, typed front-end operations"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ddd162c7bf269ee9dfe4b8e3aec50341993c256b; git diff origin/main...HEAD (PR #624): quire-exact/src/cancel.rs, quire-exact/src/accounting.rs, qsl-cst/src/parser.rs, qsl-cst/src/lib.rs, qsl-semantics/src/check/mod.rs, qsl-foundation/src/diagnostic/stage.rs, qsl-eval/src/value/expression/mod.rs, qsl-package/src/checked_v2.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/spine/call.rs, src/command.rs, and their tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: reviews
---
# Code review of quire-spec-language LC1

## Summary

Ticket: QSL-489 (LC1), PR #624, head ddd162c7. The Rust lane (rust-review) is
folded into this file. Linear is locked, so the ticket body could not be read.
Scope was judged from FR-275 to FR-278 (the PR's stated contract), the spec
status rows, and the 10-02 lead ruling that gave LC1 `StageFailure::Fault`
and `CallFailure::Cancelled`. That ruling is recorded in #610's spec rows,
which say "owned by slice LC1".

What the change does, checked against the code:

- `quire_exact::Cancel` is an `Arc` holding two `AtomicU8`s. Cancelling uses
  compare-and-exchange, so the first cause wins. `poll` records a trip. The
  test observer is called before the handle is read.
- `Meter::check_injected` polls the handle at every kernel meter charge. A
  cancelled handle denies the charge as a `WorkUnits` `Incomplete` with
  `limit = consumed`. S1 polls at the leaf-budget commit and at every parser
  step. In S3, only `contract_meter` carries the handle.
- `lifecycle::stage` checks the handle before `run`. Afterwards, if the
  handle tripped, it replaces whatever the denial became with `Cancelled`, so
  a cancelled operation never returns an output. `execute` does the same with
  `CallFailure::Cancelled`.
- `spine::compile`, `Compiled` and `Resolution`'s public use are gone. No
  wrapper is left. `run`, the CLI `compile` and the tests compose the four
  operations themselves.

The focused tests pass at this head: `cargo test -p qsl-replay --lib
spine::lifecycle`, 11 passed, run under locked-build.

Bench (180.6 to 192.8 ms on checker/independent/5000) is not a finding.
Each charge adds one `Option` branch, one load of the observer pointer and
one Acquire load, which is a plain load on x86. The charge-dense benchmark,
evaluator/call_chain/1, moved 263.8 to 264.9 ns, under 1%. The S3 checker
charges once per declaration, so the checker benchmarks barely reach the
poll. Within the same run, checker benchmarks move about 16% both ways:
object_chain/5000 went from 95.97 to 80.20 ms. That is the noise floor, and
the +6.8% sits inside it.

## Verdict

Changes requested. The typed operations, the predecessor-typed inputs, the
decision point in `stage` and the cancel type are sound. However, FR-276's
central rule, "check the handle at every charge of a work meter or a stage
limit", is met only for the kernel meter and S1. `StageFailure::Fault`,
which was assigned to LC1, is missing, and a `panic!` was added in its place.
The `Meter` size bound is vacuous.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-276 says each operation checks the handle at every charge of a work meter or a stage limit. Several charge sites never poll it. I1: `select_models` and `admit_unit` charge `ModelNormalizationLimits` meters and receive no handle. S3: the `Typer` node charge, the `DeclarationMeter` measure pass, and the type-environment work meter in `PackageDeclarations::assemble` all run unpolled. Only `contract_meter` gets `with_cancel`, and it is charged once per declaration. E4: `emit_checked` takes no handle. So a cancel during one large function body, a large domain package or a large emit does not stop within one charge. In scope: polling "at every charge" is QSL-489's stated purpose. Fix: pass the handle into each of these meters and charge loops. | qsl-replay/src/spine/lifecycle.rs:359-371,388-397; qsl-semantics/src/check/mod.rs:946-947; qsl-semantics/src/check/family.rs:630 |
| FND-002 | high | `StageFailure::Fault` does not exist. FR-275 Behavior: "an internal invariant failure shall return `StageFailure::Fault` ... rather than a panic across the public boundary". The 10-02 ruling gave it to LC1, and #610's FR-285 and TC-769 rows say "owned by slice LC1". This PR rewrites both rows to "does not exist yet" with no owner. Instead it adds a production `panic!` in `read_v2` on `StageFailure::Cancelled`, and `refusal_or_fault` returns a bare `InternalFault` outside `StageFailure`. Fix: add `StageFailure::Fault(InternalFault)`, category from the fault (FR-285-AC-3, TC-769 step 3). Route `read_v2`'s impossible arm and `refusal_or_fault`'s fault arms through it. | qsl-package/src/checked_v2.rs:168-170; qsl-replay/src/spine/lifecycle.rs:116-130; qsl-foundation/src/diagnostic/stage.rs:283-294 |
| FND-003 | medium | The `Meter` size bound is vacuous. `size_of::<Meter>()` is about 208 bytes: limits 10×u64 = 80, consumed 80, `Option<InjectedDenial>` 24, two u64 counters 16, `Option<Cancel>` 8. So a `Vec` or `String` field (24 bytes) still passes `<= 256`, and so do two of them. The comment "A `Vec` or `String` field is larger than this and fails the build" is false. The renamed test asserts the same compile-time constant. The struct doc still says the meter "retains no heap state", but it now holds an `Arc`. The property is real: meter memory must not grow with its charges. Fix: move the counters into an inner struct, assert `!needs_drop` on it, and keep `Option<Cancel>` outside it. Or delete the assertion and the test. Fix the doc either way. | quire-exact/src/accounting.rs:527-536,553-558,812-832 |
| FND-004 | medium | `package` takes no limits value. FR-275 Behavior: "Each QSL lifecycle operation shall take its typed request, its limits value and `&Cancel`". `package(checked, cancel)` breaks the call shape. Fix: add an E4 limits type, even one with no fields yet, or name its bound. Thread it through. | qsl-replay/src/spine/lifecycle.rs:279-293 |
| FND-005 | low | `convert_call_failure` maps `CallFailure::Cancelled` to a fault, with the comment "The meter `run` builds holds no `Cancel` handle". That is stale. `run` now passes its own handle to `execute`, which attaches it to the meter. The arm is still unreachable, because nobody cancels that handle, but the reason given is wrong. | qsl-replay/src/spine/call.rs:502-507 |
| FND-006 | low | The cross-thread cancel tests rely on the scheduler. If the second thread is starved until the operation finishes, they fail spuriously. If the meter poll is removed, the observer never runs and the canceller spins forever, so the test hangs instead of failing. Fix: in the observer, at charge N, hand off to the second thread and block until it has cancelled. The test then still cancels from a second thread, is deterministic, and can assert `after_cancel == 1` and `charges == N + 1` exactly. | qsl-replay/src/spine/lifecycle/tests.rs:330-369,462-504,524-563 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | `Cancel::poll` now runs `fetch_add` on a shared `AtomicU64` at every charge, to feed `StageWork`. That is a locked read-modify-write on x86. `spine::run`'s `execute` always attaches a handle, so every evaluation charge pays for it. The PR's bench predates the change: it measured a poll that did only a load. Fix: re-run `evaluator/call_chain` and `checker/independent` against `ddd162c7`. If the cost shows, count each stage's work from its own meter's admissions instead of the shared handle. | quire-exact/src/cancel.rs:113-114 |
| FND-008 | low | `Resolution` checks every library of the closure under the importing unit's `LockEvidence`: `declarations.lock_evidence = self.lock.clone()` runs in `check_unit`, which `compile_library` also calls. A library's checked graph, and therefore its recomputed `package_id`, can then depend on who imports it, because a text law selected from the lock changes the graph. Every caller passes `LockEvidence::default()` today, so nothing differs yet. Fix: state in FR-278 or ADR-015 D-1 which lock a library is compiled under, and implement that (most likely the library's own). | qsl-replay/src/spine/lifecycle.rs:842; qsl-replay/src/spine/lifecycle.rs:768-779 |
| FND-009 | low | Some `LimitExceeded` counters from S1 are not the counter at the failed charge (FR-277 Outputs). `source.source_bytes` reports `actual = bound + 1` although the source's real length is known. `source.work_units` reports the per-token field as `configured_bound` and the total step budget plus one as `actual`, so the two numbers are in different units. FR-277-AC-1 checks only kind, configured value and field, so no test catches this. Fix: report the real byte length, and give both work numbers in the same unit (the total budget, or per token). | qsl-replay/src/spine/lifecycle.rs:175-205 |

## Dispositions

Round 1, reviewed at a47829387eae0445ada116f6594f599654187cf6 (fix commits
7116333d6, bfd81c9d9, a47829387 on ddd162c7). Focused tests pass:
`cargo test -p qsl-replay --lib` (290), the `spine` doc tests (both
`compile_fail` doctests and their controls) and `cargo test -p quire-exact
--lib`, all run under locked-build. Owner rulings applied, not raised:
`execute` stays crate-internal with no wrappers, and FR-276-AC-2 stays at
4,000 declarations.

I checked the poll sites against the code. Every one listed in the AC-2 test
doc polls:

- the kernel `Meter`, which also covers S3 lowering, because lowering charges
  `contract_meter`;
- S1's leaf commit and parser step;
- the I1 normalization meter, through `admit_unit_with_cancel` and
  `normalize_with_cancel`;
- the measure pass, once per node, in `encode_expression`;
- the `Typer` node charge;
- the type environment, through `TypeEnvironment::bounded_with_cancel`;
- E4, at entry and at every node.

The one site that does not poll is `link_dispatch`. Its only callers are
`checked_dispatch_operation`, qsl-eval's tests and xtask, so no spine
operation reaches it.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 7116333d6 |
| FND-002 | fixed | 7116333d6 |
| FND-003 | fixed | 7116333d6 |
| FND-004 | fixed | 7116333d6 |
| FND-005 | fixed | 7116333d6 |
| FND-006 | fixed | 7116333d6 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | `Cancel`'s charge counter is now a `u32`, because the no_std target has no 64-bit atomics, and `charged` takes `wrapping_sub` of two readings. If one stage of one operation makes 2^32 or more charges, its `StageWork` counter silently reports the count modulo 2^32, which is a wrong result. That takes raised limits and minutes of work, so it is rare, but nothing reports it. Fix: make `charged` saturate, by having the guard flag a wrap, or accept it and document on `StageWork` that each counter is modulo 2^32. | quire-exact/src/cancel.rs:146-153; qsl-replay/src/spine/lifecycle.rs:119-124 |

Round 2, reviewed at 906b32262628682460096b4785d2d0cb2a71d798, rebased on
main (fix commits 83f03a6f2, a294f79be, f314b57c0, 906b32262). The pre-merge
`make ci` passed on this head (logs/qsl-624-premerge-ci.log). Lead ruling on
FND-008: a library is checked under its own default `LockEvidence`.

- **FND-007:** `poll` now pays for the count only while a `ChargeCount`
  guard is held. Only `stage()` holds one, so `spine::run`'s `execute` polls
  with a load alone. The coder's criterion runs show no regression against
  the ddd162c7 baseline. call_chain/1 was 279 ns at ddd162c7 and 276 and
  294 ns at head, which is within the run-to-run noise. checker/independent
  showed no significant change at 5000 and an improvement at 1000.
- **FND-008:** `compile_library` checks under `LockEvidence::default()`.
  FR-278 Behavior and its row state this, and
  `a_library_is_checked_under_its_own_lock_evidence_not_its_importers` tests
  it.
- **FND-009:** `source.source_bytes` reports the source's real length, and
  `source.work_units` gives the bound and the counter both in parser steps.
  Both are asserted in `parse_names_the_source_limit_field_it_reached`.

I also checked the typed `LimitsField` (17 variants). It matches every field
of `qsl_cst::Limits` (4), `ModelNormalizationLimits` (8),
`TypeEnvironmentLimits` (2) and `CheckingLimits` (3). The match in
`CompileRefusal::stage` is exhaustive with the same stage mapping as before,
`None` included. `dependencies.depth` is absent, as B4 owns it. I found no
defect in it.

`ChargeCount` decrements `counting` in `Drop`. A leaked guard, from
`mem::forget`, only keeps counting on, so it costs time and cannot give a
wrong result. FND-010 above is the one new item.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-007 | fixed | 83f03a6f2 |
| FND-008 | fixed | 83f03a6f2 |
| FND-009 | fixed | 83f03a6f2 |
