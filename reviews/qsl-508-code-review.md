---
id: SR-12071
title: "Code review of quire-spec-language PR #934: TemporalTrace check of temporal clauses (QSL-508 T2)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e2c39842b97155a1703db5e0cf233d0d4c4bca93; PR #934 diff origin/main...HEAD: qsl-semantics/src/check/temporal_clause.rs, qsl-semantics/src/check/temporal_clause/tests.rs, qsl-semantics/src/check/{assemble.rs,assemble/tests.rs,check.rs,mod.rs,profile.rs,refusal.rs,region.rs}, qsl-foundation/src/bound.rs, qsl-forms/src/syntax.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/refinement.rs, qsl-replay/src/spine/clause/tests/refinement.rs"
review_set: subset
---
# Code review of quire-spec-language PR #934

## Summary

Ticket: QSL-508. Method: code-review with the rust-review lane folded in.
Read-only review of the PR diff at the reviewed sha. Nothing was built or run.

Checked:

- Each clause kind maps to its form and typed cause. A bare operator under a bounded profile gives `MissingInterval`, `unsupported_construct`/`expression-form`, at `operator_span`. `[a,*]` under a bounded profile gives `OpenInterval` at the interval span. `a > b` under every profile gives `InvertedInterval`, `ill_typed`/`type-mismatch`, built from `TemporalInterval::new`'s refusal. No selection, or two, gives `ProfileSelection`, `unknown_profile`/`unsupported-selection`, at the declaration span with the identities found. An unresolved parameter type gives `ParameterType` through `type_fault_class`. A clause that checks gives `NotEmitted`, `unsupported_construct`/`not-yet-implemented`. Code and tag come from typed `match`es with no message matching.
- An absent interval gives `None` under infinite-trace, and on every connective. `[a,*]` gives `Some{a, Open}` and has no `IntervalKey` (recorded exception).
- Horizon: one reverse pass over the top-down arena. `qsl-forms/src/temporal_clause.rs` adds the root at index 0 and operands after their parent, and the code faults if that order is broken. It uses `checked_add` and reports overflow as an `S3WorkUnits` limit at the overflowing operator's span. Production code uses `get`/`get_mut` throughout, with no indexing, `unwrap` or `expect`.
- Classifier: `check_limit` has an explicit `CheckCause::Temporal(_) => None` arm. `UnimplementedTemporalClause` is removed from `assembly_limit` with no wildcard added. `refinement.rs`'s class rules read codes only. NotEmitted gives `unsupported` (rule 5). Missing/Open give `prohibited` (rule 6). ProfileSelection, Inverted and ParameterType give `refused` (rule 3).
- Removed `UnimplementedTemporalClause`: no remaining references in the repo. Every temporal clause still refuses: at check, either with its own cause or with `NotEmitted`. `check_with_cancel` returns `Err` on every earlier exit path, so no clause compiles silently.
- Region: `DeclarationRegions::refusal_region` reads a `Temporal` cause's own span, the same way it handles `ProtocolAnchor`.
- No `NodeKey`/`EffectiveId` mint (TC-255 scan). `clock_binding` uses `WireNodeId::from_digest`, which the scan does not count as a mint, under the recorded exception.

## Verdict

PASS. No major defects in the production code. The trace-tag findings are in the gap analysis, SR-12072.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
