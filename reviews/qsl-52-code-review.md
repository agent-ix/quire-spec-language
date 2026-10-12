---
id: SR-7816
title: "QSL-52 PR 746 code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@99da36f7ac0ca455e5e3bc7bb8c46c6b37bd59b9 (base 423dddc9470cecdc6036f3dde4034af40e057a37); agent-ix/quire-specification@34384b8ebb917e3b9545b15f91704ad75152bf95; qsl-eval/tests/it/dispatch_calls.rs"
review_set: subset
---

## Summary

Requalified the one-file test diff at `99da36f7ac0ca455e5e3bc7bb8c46c6b37bd59b9` against `423dddc9470cecdc6036f3dde4034af40e057a37`, using QSpec main `34384b8ebb917e3b9545b15f91704ad75152bf95`. The previous Rust review finding is fixed: D is evaluated through the A-context invariant and its D01 work limits are asserted.

## Verdict

**PASS.** FND-001 is fixed at this head; the corrected D path exercises the A-rooted dispatch family and asserts the specified 6/5/1 work-limit outcomes and charge accounting. No new Rust review findings were identified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The D receiver is evaluated through the B-rooted graph and uses work budget 4/0, so the test does not assert D01's A-rooted work-6 completion or work-5 and work-1 incomplete boundaries. | qsl-eval/tests/it/dispatch_calls.rs:1970; qsl-eval/tests/it/dispatch_calls.rs:1995; qsl-eval/tests/it/dispatch_calls.rs:2048; qsl-eval/tests/it/dispatch_calls.rs:2066 |

## Examined scope

```yaml
scope:
  - {id: QSL-52, path: "Linear QSL-52", role: examined, excerpt: "Add a test that exercises D01's runtime behaviour as TC-196 states it. Trace it to TC-196 and the FR-151 AC that D01 names. If the runtime half needs code that isn't there yet, record what's missing on this ticket."}
  - {id: "qsl-eval/tests/it/dispatch_calls.rs#d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit", path: qsl-eval/tests/it/dispatch_calls.rs, role: examined, excerpt: "#[trace(\"QSpec-TC-196\", \"QSpec-FR-151-AC-8\")]\n#[test]\nfn d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit() {"}
  - {id: QSpec-TC-196-D01, path: "../quire-specification/spec/test-cases/TC-196-conformance-redefinition-and-dispatch.md", role: examined, excerpt: "| D01 | G and S2; link; then under `ModelNormalizationLimitsV1` with `dispatch_candidates: 8`, then `7`; evaluate `I` with `self` = object `d1` of type `D` under `ScalarLimitsV1` with every counter unlimited except `work_units: 6`, then `5`, then `1`; evaluate `I` for `c1` of type `C` | for `(A, A/size)`, per subtype `C`, `A`, `D`, `B` in that order: one `dispatch.subtype`, then one `dispatch.candidate` for each of `A/size` and `B/size` (work `f` = 2 for `C`, 1 for `A`, 5 for `D`, 2 for `B`), then for `D` and `B`, which each have two applicable candidates, one `dispatch.dominance` of"}
  - {id: QSpec-FR-151-AC-8, path: "../quire-specification/spec/functional/type-model/FR-151-resolve-conformance-redefinition-dispatch.md", role: examined, excerpt: "Dispatch charges at their exact bound complete and the one-less run is incomplete at the named charge point."}
  - {id: "qsl-eval/tests/it/main.rs#dispatch_calls", path: qsl-eval/tests/it/main.rs, role: context_only, excerpt: "mod dispatch_calls;"}
  - {id: AGENTS.md#integration-test-placement, path: AGENTS.md, role: context_only, excerpt: "New integration tests go in tests/it/<name>.rs with a mod <name>; line added to tests/it/main.rs never a new top-level tests/*.rs file."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: code-review
    check: test-intent
    artifact_id: QSpec-TC-196-D01
    related_ids: [QSpec-FR-151-AC-8, "qsl-eval/tests/it/dispatch_calls.rs#d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit"]
    path: qsl-eval/tests/it/dispatch_calls.rs
    lines: "1970-1979"
    excerpt: |-
      | D01 | G and S2; link; then under `ModelNormalizationLimitsV1` with `dispatch_candidates: 8`, then `7`; evaluate `I` with `self` = object `d1` of type `D` under `ScalarLimitsV1` with every counter unlimited except `work_units: 6`, then `5`, then `1`; evaluate `I` for `c1` of type `C` | for `(A, A/size)`, per subtype `C`, `A`, `D`, `B` in that order: one `dispatch.subtype`, then one `dispatch.candidate` for each of `A/size` and `B/size` (work `f` = 2 for `C`, 1 for `A`, 5 for `D`, 2 for `B`), then for `D` and `B`, which each have two applicable candidates, one `dispatch.dominance` of
    finding: "The test checks the D receiver through `graph_b`, whose dispatch root is `B.size` (lines 1970-1979 and 1995-2004), and repeats that package for the runtime budget probes (lines 2048-2084). TC-196 D01 requires the `(A, A/size)` family, where D selects B/size. The test instead asserts D completion at five work units and probes limits four and zero; D01 specifies completion at six, the one-less limit five failing at `ordering.result-retain`, and limit one failing at `dispatch.select` with zero consumed and a next charge of two. Thus this test can pass without verifying the specified A-rooted D dispatch or either exact boundary. Exercise D through `graph_a` and assert the D01 6/5/1 outcomes and charge accounting."
bindings:
  - {test_id: "qsl-eval/tests/it/dispatch_calls.rs#d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit", ac_id: QSpec-TC-196, trace: correct}
  - {test_id: "qsl-eval/tests/it/dispatch_calls.rs#d01_runtime_dispatches_d_and_c_receivers_and_observes_the_work_limit", ac_id: QSpec-FR-151-AC-8, trace: correct}
```

## Evidence and limits

The diff is test-only and introduces no production panic, unsafe block, wire conversion, asynchronous work or new test double. The test is registered through the existing integration-test module and uses the real domain bridge, dispatch checker, checked package and evaluator; result/body assertions are value-bearing. No applicable AssuranceProfile was found. No tests or builds were run: the dispatch explicitly forbids builds, and the candidate summary records the focused test as passing. The supplied all-features clippy baseline-red classification is accepted as planner-approved and matches the comparison receipt at `/home/peter/dev/worktrees/logs/batches/qsl52-clippy-baseline-compare-d52-c940.txt`; it is not a new finding.

Requalification evidence: the commit diff remains one added test in `qsl-eval/tests/it/dispatch_calls.rs`; PR #747 changes `qsl-eval/tests/it/model_reference_queries.rs`, `qsl-semantics/src/check/check.rs`, `qsl-semantics/src/check/mod.rs`, `qsl-semantics/tests/it/model_population.rs`, and `qsl-semantics/tests/it/state_clauses.rs`, none of which overlaps this diff. QSpec D01 and FR-151-AC-8 at the cited main commit retain the same required `(A, A/size)` family, work limits 6/5/1, and six-unit completion. No tests, builds or source edits were performed. The strict matrix scan previously produced no output and was stopped; this requalification directly inspected the unchanged `#[trace]` binding and assertions.

No tracker comment was created, per the instruction not to post to the public mirror. There is no comment id or read-back. The `has-review` label was already present on QSL-52.

At `99da36f7ac0ca455e5e3bc7bb8c46c6b37bd59b9`, `add_invariant` sets `context: a_type`, and the D budget cases call `evaluate_clause("D01Invariant", d_type, "d1", ...)`; the test asserts work 6 completes, work 5 stops at `OrderingResultRetain`, and work 1 stops at `DispatchSelect` with zero consumed and next charge 2. Explicit fixture admission is documented as outside FR-106 coverage. The supplied receipt `batches/qsl52-rebase-check-99da36f.log` reports workspace check and focused D01 test passing at this SHA; this requalification ran no builds or tests.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | Requalified at 5253c4fcdd54db296eeb390ea5fd9311d0703a17 against 362a7b3398a79e86a1fe0af22960afc93f120363: the committed test still evaluates D through graph_b rooted at B.size, asserts completion at five work units, and probes limits four and zero; it does not verify D01's A-rooted work-6 completion or work-5/work-1 incomplete boundaries. |
| FND-001 | fixed 99da36f7ac0ca455e5e3bc7bb8c46c6b37bd59b9 | `add_invariant` sets `context: a_type`; D is passed to `evaluate_clause` and the test asserts work-6 completion, work-5 `OrderingResultRetain` incomplete, and work-1 `DispatchSelect` incomplete with zero consumed. |
