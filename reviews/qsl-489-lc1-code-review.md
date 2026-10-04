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
