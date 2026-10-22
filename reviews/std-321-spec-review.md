---
id: SR-16060
title: "Spec review — same-run plugin replay context carrier (FR-404)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md, spec/test-cases/TC-1173-the-replay-facade-carries-the-call-s-plugin-replay-context-from-the-same-run.md, spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md, spec/spec.md at 9b95aea"
review_set: subset
evaluated_revision: "9b95aea40d70775a90bfcac74be5fe98e25b2255"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-404
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1173
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---

## Summary

Ticket: STD-321 (blocks QSL-1006). PR quire-spec-language#1272 adds FR-404
(`run_with_replay_context`, `RunWithReplayContext`, `CallReplayContext` on the
`qsl_replay` root), TC-1173, a pointer sentence and `references` edge in
FR-290, and the spec.md index row. Reviewed `git diff origin/main...HEAD` at
9b95aea against FR-290 (replay context, request table, refusal order, AC-12,
AC-14, AC-15), FR-395, FR-060 T12-A, FR-059, FR-098, FR-255, FR-278, ADR-013
O-04, and QSL main code: `qsl-replay/src/call_requirements.rs`,
`proof_result/plugin_replay.rs`, `limits.rs`, `compile.rs`, `execute.rs`
(`run_spine`), `spine/lifecycle.rs` (`package`, `PackageLimits`) and `lib.rs`.

Examined units: FR-404 statement and Behavior clauses (One compile, Compile
limits, Package binding, Occurrence binding, Refusal order, Same request,
Outcome independence, Typed refusals, No consumer construction, Facade
boundary), FR-404-AC-1 to AC-9, TC-1173 procedure and expected results 1 to 9,
the FR-290 added sentence. Clean: AC-1, AC-2, AC-4, AC-7, AC-8, Package
binding, Same request, Outcome independence, Refusal order, the FR-290 edit
and index row. Lead defaults 1 (separate entry), 3 (two accounting budgets)
and 5 (function-contract items refused on this path) are consistent with
FR-395, FR-100 and FR-290; defaults 2 and 4 carry findings FND-002 and
FND-001.

Ids FR-404 and TC-1173 are absent from origin/main and from every other open
PR head (20 open PRs checked). `make check-spec-validation` exits 0 with no
diagnostic on FR-404, TC-1173 or FR-290. Wording is behaviour-only. No new
crate edge arises for FR-059: `BackendId`, `OccurrenceKey` and `ProofBound`
come from `qsl_foundation`, which `qsl-replay` already depends on.

## Verdict

NOT MERGEABLE as written. One high and three medium findings. The occurrence
binding keys on `OccurrenceKey` alone, and application-node keys are
content-addressed with no package, so the run's context accepts another
run's identical member key; the "cannot mix runs" guarantee STD-321 asks for
is not delivered and no AC exercises it. The compile-limits clause names an
input the spine `package` stage does not take, so a QSL-1006 coder would have
to guess. AC-9 leaves the no-deserializer clause untested, and nothing stops
an accessor to the wrapped `PluginReplayContext` that bypasses the binding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Occurrence binding checks the key only. Application-node keys carry no package (ADR-013 O-04), so R2's `q` member key equals R's and R's context builds a request for it with R's `package_id`; the Description's "cannot pair one run's items with another run's context" does not hold, and AC-5 tests only `s`, a key R lacks. Bind on (`package_id`, key) or the FR-395 pairing key with a stated refusal, or narrow the claim, and add an AC for R2's `q` key against R's context. | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md:46-47,102-109,161 |
| FND-002 | medium | "`package` under its `library` limits" names an input the stage does not take: spine `package` takes the field-less `spine::PackageLimits {}`, `CallerLimits.library` is `qsl_semantics` `bundle::PackageLimits`, and FR-098's recompile (`run_spine`) applies only `limits.spine`. AC-6 reads only values FR-290 already copies into `stage_limits`, so no AC can fail on whether library limits bound the compile. | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md:91-97,162 |
| FND-003 | medium | AC-9 does not test the "no deserializer" clause, and its `compile_fail` "calls a constructor" names no constructor, so any spelling satisfies it. Name the constructors to refuse (for example `CallReplayContext::new`, `Default`) and add a check that `CallReplayContext` implements no `Deserialize`. | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md:127-130,165 |
| FND-004 | medium | No clause forbids an accessor to the wrapped `PluginReplayContext` (`Deref`, `AsRef`, a getter). Such an accessor is ordinary for a wrapper and would let a consumer call the unbound `PluginReplayContext::request`, bypassing the occurrence binding. State that the context exposes only `package_id` and `request`. | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md:127-130 |
| FND-005 | low | "refuse exactly as `run_with_requirements` refuses for the same inputs" leaves the limits mapping implicit, since the two entries take different limit types; state it as the Outputs clause does (the `CallerLimits`' `spine` limits as `SpineLimits`). | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md:123-126 |
| FND-006 | low | Facade boundary says the consumer names no "other QSL crate", but the proof bounds the driver passes come from `qsl_route::items_from_requirements` (FR-075), which `qsl-replay` does not depend on; AC-9 lists only `qsl_replay::spine`, `qsl_foundation` and `qsl_semantics`. Align the clause with AC-9. | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md:131-138,165 |
| FND-007 | low | AC-3's second sentence builds a request for `q`'s member without naming the assignment; TC-1173 step 3 uses `x = 9`. State it in the AC. | spec/functional/FR-404-carry-the-call-s-plugin-replay-context-through-the-replay-facade.md:159 |
