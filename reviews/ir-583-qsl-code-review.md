---
id: SR-12354
title: "Code review of quire-spec-language PR #1269: moved evaluation vocabulary repoint and reference adapter (IR-583 S0)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a999bdf3d587cbf14307e08388e6b8ae61cd594d; 252 files vs origin/main 066f5a1c9; concentrated on qsl-eval/src/value/expression/{closure,mod,family,evaluate}.rs, qsl-foundation/src/{diagnostic,lib}.rs, qsl-semantics/src/value/supplied.rs, qsl-semantics/src/check/mod.rs (callable_by_identity), qsl-replay/src/lib.rs, manifests, Cargo.lock, tools/arch-lint/api_surface.rs, xtask/src/{seam_probe,definition_scan}.rs; mechanical repoints sampled; against FR-401, FR-262 (as merged by STD-319), FR-400, FR-402, NFR-015, ADR-011 §7.2, TC-170, TC-452; rust-review lane folded in; reviewed together with quire-eval PR #3 (SR-12352, SR-12353)"
review_set: subset
date: '2026-10-22'
---
# Code review of quire-spec-language PR #1269

## Summary

Ticket: IR-583 (S0). Rust-review lane folded in. quire-eval #3 merges first,
and this PR builds against it through a lockfile pin to #3's head.

Examined and clean:

- **Repoints keep their meaning.** I sampled about 40 of the mechanical
  files across qsl-replay, qsl-analyze, the root crate and the tests. Each
  changes a path and nothing else. The four supplied helpers that became
  private free functions (`event_of_membership`, `event_of_part`,
  `storage_of_request`, `storage_of_arena`) have the same bodies as the
  methods they replace. `CallFailure`, its `From<SuppliedFailure>` and its
  `category` moved without change, as did `FamilyOutcome`'s
  `From<EvalOutcome>`.
- **`refusal_record` free functions keep locus and location.**
  `qsl_foundation::diagnostic::refusal_record(&dyn CatalogCoded,
  Option<Locus>)` has the same body as the deleted provided method. No
  implementor overrode that method on main, so dynamic dispatch never chose
  another body. `qsl_eval::value::refusal_record(&Evaluation, resolve)`
  resolves `evaluation.location` through the caller's resolver, wraps it in
  `Locus::Region`, and makes the same two-arm match. Every caller passes
  `|l| graph.region(l)` for the graph the evaluation ran.
- **No stray re-exports.** I found no `pub use` or `pub type` of a moved item
  at any QSL path. The old `qsl_foundation` root and `diagnostic` exports,
  `qsl_semantics::family::{FamilyOutcome, FamilyResult, EvalOutcome}`,
  `qsl_semantics::value::supplied::Supplied*` and
  `qsl_eval::value::{Evaluation, CallFailure}` are all gone.
  `qsl_replay::InputRefusal` is an older re-export of an item that did not
  move. FND-001 is about the reverse case: a re-export that was removed.
- **Double-charge avoidance.** The scheduler charges the root and requested
  `function.call`, and `advance` runs the body through
  `EvaluationEnv::charged`. That sets `call_charged`, so
  `ValueFunctionFamily::evaluate` skips its own charge. The synchronous
  `call_supplied` and `evaluate` now run through the same session. All 778
  qsl-eval tests (662 integration, 116 lib) pass with no expected value
  changed. Several of those tests pin absolute work spend, so a double
  charge would show up there.
- **Ownership through `Deref`.** `PackageClosure<P, X>` holds the package as
  `P: Deref<Target = CheckedPackage>`. The suspended `PackageTask` holds only
  the `NodeKey` and owned arguments, or a cloned `X` handle and owned
  parameters, so no frame borrows the package. `PhantomData<fn() -> X>`
  keeps the closure `Send`/`Sync`, whatever the variance of `X`.
- **`callable_by_identity`** is a `BTreeMap` lookup, filtered to
  `callable_by_name`, so an FR-151 synthesized candidate is not admitted.
  It mirrors `callable`.
- **Seam probe.** quire-eval declares `cfg(seam_probe)` and keeps
  `FamilyOutcome::__SeamProbe`. The QSL match in
  `ValueFunctionFamily::evaluate` keeps its `#[deny(wildcard_enum_match_arm)]`
  and its probe arm under `seam_probe_eval_downstream`. `make seam-probe`
  passed at this head (exit 0). It confirmed 17 locations under `--cfg
  seam_probe`, including `ValueFunctionFamily::evaluate`, the replay
  `convert_outcome` and `settle` matches, and `CheckCause::code` over
  `WrongSnapshotCause`. Every QSL match over a moved seam enum is still
  non-exhaustive in the probe build.
- **Dependencies.** qsl-foundation adds quire-semantic-value. SV has no
  `std` feature and depends on no QSL crate, so NFR-015's no_std build of
  qsl-foundation keeps its shape. qsl-eval, qsl-replay and qsl-bench add
  quire-eval. The layers below them do not.

## Verdict

**Not mergeable yet.** FND-001 breaks CG at its next lock update. FND-002
breaks a FR-262 rule that is stated outright. FND-002 and FND-004 share
their root cause with SR-12352 FND-001 and FND-002, so they are fixed by
re-implementing the adapter against the corrected quire-eval trait. FND-005
is a merge-order condition. The mechanical repoint itself is sound.

The documented granularity limit (FND-003) is compatible with FR-401-AC-4
and AC-5, and with FR-262-AC-4, where the interpreted function is the last
link in the chain. It does conflict with quire-eval's own `advance`
contract, and the spec does not say whether an interpreted call site must
reach a registered body. That needs a spec ruling, not a code change in S0.

The cancel handle in `call` and `evaluate` (`SuppliedRun::new(..,
Cancel::new())`) was already there on main and is unchanged here.
`call_supplied` uses the caller's run, whose handle may differ from the
meter's. The new code path is the adapter's `admit` (FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR deletes `pub use qsl_foundation::diagnostic::Category;` from qsl-replay and adds nothing in its place. It leaves the comment above it ("re-exported so CG names `Category::Success` and `Category::Violation` through this crate") orphaned. CG's origin/main imports `qsl_replay::Category` in src/replay/function.rs:14, src/replay/state_clause.rs:27 and tests/it/kani_obligations_state_clause_replay.rs:16. Its only QSL dependency is qsl-replay at `branch = "main"`, and it has no quire-semantic-value edge, so CG fails to compile at its next lock update. Fix: land a paired CG PR with this one, adding the SV (or quire-eval) dependency and importing `quire_semantic_value::catalog::Category`, and delete the orphaned comment. Whether qsl-replay's CG facade may keep the item as `pub use quire_semantic_value::catalog::Category;` is a spec question; see the spec points below. | qsl-replay/src/lib.rs:234-236 |
| FND-002 | high | `PackageClosure::admits_request` maps an undecided membership check to `false` (`types.admits(value_type, argument).unwrap_or(false)`, commented "A refused reservation decides nothing, so it admits nothing"). The scheduler reports `false` as `Fault(child-request-not-admitted)`. FR-262 says such a check "SHALL terminate the session with `CallFailure::Storage`, whose `parameter` is the index of the callee's parameter". The root cause is quire-eval's `bool` signature (SR-12352 FND-001). Fix: re-implement against the corrected trait, returning the parameter index and the `EnvironmentFailure`. | qsl-eval/src/value/expression/closure.rs:194-221 |
| FND-003 | medium | An interpreted frame runs to its terminal result in one transition, and its own calls run inside `Machine`. As the module doc admits, when an interpreted body calls a callable that has a registered host body, the interpreted body runs instead. Which body runs for a sealed binding therefore depends on whether the call site is the root, a host body, or interpreted code. That contradicts quire-eval's `CheckedClosure::advance` contract ("A source application is returned as `Transition::Call` ... the frame never recurses into an evaluator for it"). It also contradicts FR-401 Behaviour 4, which says the leaf "drives interpreted and registered bodies through one task stack". No test shows the bypass. Fix: add a test that pins the current behaviour, and take the spec ruling below. Either FR-401 allows this granularity for the S0 reference adapter until IR-590, or the adapter must return a child request for each call to a registered identity. | qsl-eval/src/value/expression/closure.rs:18-28, 259-310 |
| FND-004 | medium | `PackageClosure::admit`, the Begin admission for a closure built with `new`, builds its supplied-value run with `Cancel::new()`. The meter's installed handle, which FR-262 makes the session's handle, is therefore never polled during Begin. A caller that cancels before Begin gets admission and then a located incomplete result at the first charge, never the pre-charge `CallFailure::Cancelled` that FR-262 reserves for Begin's supplied-value run. The root cause is quire-eval's admission methods, which receive no meter (SR-12352 FND-002). Fix with that change: build the run from `meter.cancel()`. | qsl-eval/src/value/expression/closure.rs:111-130 |
| FND-005 | medium | Cargo.lock pins `quire-eval` under `?branch=main` to `942a1b4e`, which is quire-eval #3's branch head and not on quire-eval main. quire-eval squash-merges, so that commit never reaches main. A fresh resolution or `--locked` build after the branch is deleted can fail to find it, and it is not the code that will ship. Fix: after #3 merges, run `cargo update -p quire-eval` and commit the lock before this PR merges, as the PR body says. Do not merge with the current lock. | Cargo.lock:1391-1395 |
| FND-006 | low | `a_session_over_the_adapter_matches_call` compares `package.call` with a session it drives directly. `call` now drives the same session, so the two spends are equal even if both double-charge. Assert the absolute spend instead: one `function.call` plus the body's own charges. | qsl-eval/tests/it/reference_adapter.rs:126-157 |
| FND-007 | low | `declaring` walks the whole dependency closure, deduplicating with a `Vec<*const CheckedPackage>` (quadratic). It runs again for each of `signature`, `admit_call` and `advance` on every application. `PackageTask` is a public enum with public fields and a `Finished` state, which exposes the adapter's internal state machine in `qsl_eval::value`. Fix: use a `BTreeSet`, or resolve the declaring package once into the task, and make `PackageTask` opaque (a private-field struct) or `pub(crate)`. | qsl-eval/src/value/expression/closure.rs:93-109, 133-151 |

## Spec points for draco

1. **Interpreted call sites and registered bodies (FND-003).** FR-262 and
   FR-401 Behaviour 4 do not say whether a registered host body must run
   when interpreted code applies its identity. Choose one. (a) Every
   application of a sealed identity runs the registered body, so the S0
   reference adapter is non-conforming until IR-590 and FR-401 must say so.
   (b) The S0 reference adapter may run an interpreted frame to its terminal
   result in one transition, reaching registered bodies only from the root
   and from host-body requests. Then quire-eval's `advance` doc and FR-401
   Behaviour 4 need that exception written in.
2. **Facade re-exports of moved items (FND-001).** FR-401 says a moved type
   leaves "no copy or re-export at a former path". It does not say whether
   qsl-replay's CG facade (`qsl_replay::Category`, documented for CG) counts
   as a former path, or whether CG must take its own SV edge. Rule on it,
   and name the CG adaptation if CG adapts.
3. **Begin's supplied-value cancel handle (FND-004).** This is the same
   point as SR-12352's: state that Begin's supplied-value run polls the
   meter's installed handle.
