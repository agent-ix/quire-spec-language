---
id: SR-12235
title: "Code review (Rust lane) of QSL-571 E17d FR-192 statistical settlement map, FR-193 witness keeping and the replay_sampled core (PR #1118)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c04bde8337d3005ee606c6900b0e427940d7b824; qsl-replay/src/{lib.rs,outcome.rs,proof_result.rs,statistical.rs,statistical/settle.rs,model_check/limits.rs}, qsl-analyze/src/{statistical.rs,statistical/tests.rs}, qsl-eval/src/simulation/{mod.rs,sampled.rs,sampled/replay.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: reviews
---
# SR-12235: Code review (Rust lane) of QSL-571 E17d FR-192 statistical settlement map, FR-193 witness keeping and the replay_sampled core (PR #1118)

## Summary

Ticket: QSL-571. PR quire-spec-language#1118, branch qsl-571-impl, head c04bde833 (one implementation commit 90277efcf plus a merge of main). The review covers `git diff origin/main...HEAD` only: 11 files, +1710/-80. The rust-review lane is folded into this file. FR-192 and FR-193 were read on this repo's `origin/main`, including #1098's merged Expansion rows. QSpec FR-408, FR-409 and FR-410 were read by id, read-only.

No gates were run, per the brief. One narrow run through `locked-build.sh`: `cargo test -p qsl-replay -p qsl-eval -p qsl-analyze --lib` filtered to `statistical` and `sampled::replay`. Its result is in the Verdict.

## Verdict

**PASS (no major).** Nothing blocks. Two mediums are about what the outcome wire and the terminal record carry, not about which value or category a run settles.

What is sound:
- **Settlement map.** `settle_statistical` and `settle_statistical_refusal` match on every `StatisticalOutcome`, `StatisticalStop`, `ExpansionLimit` option and `StatisticalRefusal` variant by name, with no `_` arm. Completed settles `Measured(verdict)` and never `Proved`/`Refuted`. A budget settles `Incomplete(LimitReached{Statistical(limit), value})` with setting `statistical.<member>`. An expansion naming `EvaluationMeter`/`MaxCandidates` settles `limit-reached` with the configured bound from `SubjectLimits`. One naming no limit settles `Failed`, with `runtime_invariant`/`established-invariant-broken` when `is_internal_failure()` and the stop's own cause otherwise. Cancelled settles `cancelled{source}`. Unsupported settles `not-markov`/`total-above-draw-range`. The four request refusals settle `unsupported_projection`/`unsupported-requested-capability`. `ConfidenceBeyondPrecision` settles its own cause with no code. `NoTest` settles `Inconclusive(NoInitialState)`. `Internal` settles `Failed` with the fault's catalog code. `NotStatistical` returns `Err(NotStatisticalRequest)` with no record. The rows match FR-192's table, apart from the three refusal variants that do not exist yet (see SR-12236).
- **Measured axis.** `TerminalValue::category` maps Accepted to Success, Rejected to Violation and Undecided to Inconclusive. `basis()`/`certification` are `None` for `Measured`, and `ItemLabel::Measured` is a separate label, so nothing that counts `proved` can count a measurement. A rejection by an undefined sample is `Decision::Rejected` (the EN-4 `undefined()` helper), so it settles `measured: rejected`, category violation.
- **Witness keeping.** `violating` follows the bound direction (event false for `>= θ`, true for `<= θ`/`< θ`; fraction below/above `θ`), and an unactivated sample is never a witness. `Witnesses::offer` inserts by `partition_point` on the trace index and truncates to `max`. The kept set is therefore the `max` smallest trace indices across every test, whatever order the tests ran in. `max_witnesses = 0` keeps none. The undefined witness is stored apart from the bounded set, so it is kept whatever `max_witnesses` is. An undefined sample ends the run (`TestEnd::Undefined` → `break`), so at most one exists. The regenerative path keeps only the undefined witness. `path_probability` is the exact `Rational` product of the steps' probabilities.
- **replay_sampled.** It checks the support first, before any draw. It then recomputes the behaviour with `sample_probabilistic` itself, over `steps.len() + 1` positions. `sample` always emits exactly `positions − 1` steps (a terminal state stutters), so the zip covers every recorded step. It compares each step with `SampledStep`'s full `PartialEq` (transition, post digest, choices, values, probability, rewards). It then compares the exact `Rational` product with the recorded path probability, with no tolerance. `TotalAboveDrawRange` maps to `invalid_runtime_input`/`invalid-value`. The match on `SampleStop` is exhaustive.
- **FR-100-AC-8.** The new `qsl_replay` re-exports (`StatisticalRequestRefusal` and the `statistical::settle` items) are all `qsl_replay` types, and nothing is added under `spine`. `SubjectLimits.expansion: ExpansionLimits` and `SampledWitness.steps: Vec<SampledStep>` name `qsl_eval` types in `qsl_replay::statistical`. That module already did so (`StatisticalStop::Expansion(ExpansionStop)`), and AC-8 is scoped to `spine` and to `pub use` re-exports.
- **Panics and arithmetic.** No production `unwrap`/`expect`/indexing panics were added. `usize::try_from(max).unwrap_or(usize::MAX)`, the saturating `positions` and the saturating `measured_bytes` are all total. No `unsafe` was added.
- **The `meter`/`max_candidates` setting decision.** See FND-004. It is judged acceptable.

Narrow test run at c04bde833 (debug): qsl-analyze `test result: ok. 46 passed; 0 failed`, qsl-eval `ok. 2 passed` (both replay_sampled tests), qsl-replay `ok. 5 passed` (the three settle tests among them). `target/` was deleted afterwards.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A measurement rejected by an undefined sample writes its outcome-item cause as `ItemCause::kind("undefined-evaluation")`, with no `at` and no `cause`. FR-192's row says "`rejected`, cause `UndefinedEvaluation{where, cause}`". QSpec FR-408-AC-11 and FR-410 `decision` also name the cause with its `where` (test index, trace index, position) and its catalogued reason. The typed verdict keeps them (`tests[i].undefined`), but the wire item drops them. `ItemCause::undefined_evaluation(at, cause)` already exists. Build the cause from the rejecting test's `SampledUndefined` (test, trace, position and cause) with that constructor. | qsl-replay/src/outcome.rs:406-415 |
| FND-002 | medium | `StatisticalRecord::into_terminal` drops `diagnostic`. `TerminalRecord` has no diagnostic field, so a `Failed` record from a non-internal expansion stop (whose one diagnostic is the stop's own `ExpansionStop.cause`, per FR-192), or from an `Internal` refusal, loses that diagnostic when it becomes the FR-331 terminal record. No production caller exists yet, so nothing is wrong today. The first caller that settles through `into_terminal` would silently lose the cause. Either return the diagnostic beside the record (`(TerminalRecord, Option<CatalogCode>)`), or have the record carry it. | qsl-replay/src/statistical/settle.rs:46-52, 92-94 |
| FND-003 | low | The unsupported item writes only `kind` and `code`. A `StatisticalRequest(refusal)` cause does not name its refusal on the wire, although FR-192 says "naming the refusal", and `StatisticalRequestRefusal::as_str` is not called anywhere. `TotalAboveDrawRange{total}` writes no `total`, although its doc says "`total` a decimal string". Add the refusal name and the total to `ItemCause` for these causes. | qsl-replay/src/outcome.rs:456-462; qsl-replay/src/proof_result.rs:322-363 |
| FND-004 | low | An expansion stop's `setting` is `meter`/`max_candidates`, from `ModelCheckLimit::setting`. FR-192's row says `model_check.meter`/`model_check.max_candidates`. Judgement: the code's choice is right for now. The bound comes from the same FR-120 limits the model-check path settles with, and that path reports the same setting. FR-255 registers no `model_check.*` name, so writing one would invent an unregistered setting. The drift belongs in FR-192's text (amend to the registered name), or in FR-255 if the planner registers the dotted names. It is not a code fix, and the PR correctly leaves FR-192-AC-1 untagged until then. | qsl-replay/src/statistical/settle.rs:157-175; qsl-replay/src/model_check/limits.rs:113-125 |
| FND-005 | low | `ModelCheckLimit::as_str` now mixes spellings on the one `limit` wire field. The statistical limits give `max_samples`/`max_draws`/`max_cycle_steps` (snake case), and every other limit is kebab case (`max-states`, `evaluation-meter`). QSpec FR-408 writes `max_samples`, so this may be intended. If it is, say so in the `as_str` doc. If not, use `max-samples`. | qsl-replay/src/statistical.rs:224-231; qsl-replay/src/model_check/limits.rs:96-109 |
