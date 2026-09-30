---
id: SR-745
title: "PR 492 code review (QSL-245 remainder)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR diff; qsl-eval/src/value/expression/evaluate.rs; qsl-eval/src/value/expression/mod.rs; qsl-eval/tests/it/model_reference_queries.rs; qsl-foundation/src/diagnostic/stage.rs; qsl-replay/src/bounds.rs; qsl-semantics/src/check/region.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#492. Code review with the Rust lane (rust-review) over the PR diff only.

- **CheckedInvariant to InternalFault.** `Machine::stopped` now returns `Err(InternalFault::new("S6a", "checked-program-invariant"))` for a kernel `CheckedInvariant`. `Machine::run` already returned `Result<Evaluation, InternalFault>`, and `stopped` is private, so no caller's signature changes. Every production caller was traced. `CheckedPackage::call` goes through `evaluate_declaration` and `ValueFunctionFamily::evaluate`; `CheckedPackage::evaluate` calls `Machine::run` directly. Both already map `Err` to `CallFailure::Fault`. The two production consumers are `qsl_replay::spine::run`, whose output is byte-identical (the same `RunRefusal::Fault` with `S6a`/`checked-program-invariant`, now reached through `convert_call_failure`), and `qsl_replay::replay`, whose behaviour changes (see the gap-analysis review).
- **"Defense-in-depth, never reached" is true.** `Machine::stopped` is the only place that builds `Evaluated(outcome_from_stop(Err(stop)))`. `Halt::Family` cannot carry a kernel refusal. The success arm builds `Completed`. `ValueFunctionFamily::evaluate` adds only `Kernel(Incomplete)`. So `qsl-replay/src/spine/call.rs:544` can be reached only from its own unit test.
- **Test input.** `not x` given `Value::Integer` reaches `pop_boolean` (evaluate.rs:522), whose non-Boolean arm is `invariant()`, which is `Halt::Stop(Refusal::CheckedInvariant)`. The mutation re-run confirms it: with the check disabled, the test panics with `Kernel(Refused(CheckedInvariant))`.
- **Rust idiom.** No new `unwrap` or `expect` in production, no panics, no integer-width changes. Removing `DeclarationRegions::limit_exceeded` also drops the now-unused `LimitExceeded`/`Locus` import. The tests follow repo conventions (`#[trace]`, whole-map asserts, `let ... else` with a panic message).

## Verdict

**Approve with changes.** Four low findings, all comments or test hygiene. None changes behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `stopped`'s doc cites FR-096-AC-9, which is the I2 reader's version refusal. The requirement is FR-096-AC-15, or the Behavior text "A kernel `CheckedInvariant` SHALL become an `InternalFault`". | qsl-eval/src/value/expression/evaluate.rs:492-494 |
| FND-002 | low | Two docs are stale now that `CheckedInvariant` is `Err`. `Machine::run`'s "`Err(InternalFault)` (FR-090-AC-10)" paragraph still names only `resolve_population` faults. `CheckedPackageEvaluation::call` still says the kernel outcome is returned "unchanged". | qsl-eval/src/value/expression/evaluate.rs:423-432; qsl-eval/src/value/expression/mod.rs:289-296 |
| FND-003 | low | The doc on `a_checking_limit_stop_is_located_at_its_node` says node count is located at "the node whose entry failed the charge". The node-count case asserts the whole declaration text. That case is the family's per-declaration `check_node_count` (actual 7 is the preimage count, family.rs:1470), not `Typer`'s package-wide count, whose actual would be bound+1 = 3. | qsl-semantics/src/check/region.rs:411-418, 440-446 |
| FND-004 | low | `a_lowering_stop_is_located_by_its_location` hard-codes `declared + 36` with no derivation. A cost-table change would silently move the stop to a different node, or past the end of lowering. Derive it, or say in a comment what the 36 units are. | qsl-semantics/src/check/region.rs:489 |

## Dispositions

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.24.1 module=spec-artifacts-process@v0.26.0 id=SR-745 pr=quire-spec-language#492 date=2026-09-26 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | `stopped` doc now cites FR-096-AC-15 |
| FND-002 | fixed | `Machine::run` and `CheckedPackageEvaluation::call`/`evaluate` docs name the CheckedInvariant `Err` path |
| FND-003 | fixed | region test doc separates the family precheck (declaration span) from `Typer`'s per-node stops |
| FND-004 | fixed | `declared + 36` replaced by `budget_located_at_c_plus_d(declared)`, a bounded scan over real `check` runs |

Verified by reading the fix-round diff. FND-004: the scan is a real derivation. It runs `unit.check` for each budget in `declared+1 .. declared+1000` and returns the first whose single refusal resolves to the `c + d` text; if the cost model moves the stop off that node the scan panics instead of passing. The test loop then asserts kind `WorkBudget`, `region: None`, `location == body([2])` and region equality independently of the scan predicate. Re-ran in a scratch worktree: all 5 `check::region` tests pass.

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      /// Converts a stop to an `Evaluation`. A kernel `CheckedInvariant` is an
      /// S6a invariant break, an `InternalFault` and never a refusal record
      /// (FR-096-AC-15, observed through
      /// [`qsl_semantics::check::ValueFunctionFamily::evaluate`]).
  - fnd: FND-002
    outcome: fixed
    after_excerpt: |-
      /// **`Err(InternalFault)` (FR-090-AC-10, FR-096-AC-15).** Two distinct
      /// invariant breaks return `Err` from here, never `Ok(Evaluation {
      /// outcome: Outcome::Refused(_), .. })`: [`Self::resolve_population`] ...
      /// and a kernel `Stop::Refused(Refusal::CheckedInvariant)`, which `Self::stopped`
      /// itself turns into `Err(InternalFault)` rather than an `Outcome`
      /// (FR-096-AC-15).
      [mod.rs] ... except a kernel `Refusal::CheckedInvariant`, which never reaches
      this `Ok` arm at all (FR-096-AC-15)
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |-
      /// at a specific source text. Depth is located at the node whose entry
      /// failed the charge (`Typer`'s own per-node check); node count, input
      /// bytes and work budget are the family's own per-declaration precheck
      /// (`check_node_count`/`check_input_bytes`/`ValueFunctionFamily::check`'s
      /// own work charge), fired before `Typer` starts, and located at the
      /// whole declaration's span.
  - fnd: FND-004
    outcome: fixed
    after_excerpt: |-
      fn budget_located_at_c_plus_d(declared: u64) -> u64 {
          for budget in (declared + 1)..(declared + 1000) {
              ... unit.check(CheckingLimits::default().with_work_budget(budget)) ...
              if text(&region) == "c + d" { return budget; }
          }
          panic!("no budget in range locates the lowering stop at c + d");
      }
      ...
          (c_plus_d, &[2][..], "c + d"),
```

+++


