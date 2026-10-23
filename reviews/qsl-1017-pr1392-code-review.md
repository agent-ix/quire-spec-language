---
id: SR-16320
title: "Code review of quire-spec-language PR #1392: hybrid claims and interval forms"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9809b06b8cbeb1a8a02025b9a73ac02c323e1bc9; PR #1392 diff against qsl-1017-impl: qsl-cst/src/{grammar,parser}.rs; qsl-forms/src/{lib,syntax,temporal_clause}.rs; qsl-forms/tests/it/{hybrid_forms,temporal_clause_forms,time_forms}.rs; qsl-replay/src/spine.rs; qsl-semantics/src/check/{assemble,check,hybrid,mod,refusal,temporal_clause}.rs; qsl-semantics/src/check/hybrid/{callee,cause,claim,model,term,tests}.rs; qsl-semantics/src/check/temporal_clause/timed.rs; two spec files"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
---
# Code review of quire-spec-language PR #1392

## Summary

Ticket: QSL-1017, second PR in the hybrid stack. Reviewed the PR's own diff
against `qsl-1017-impl`, its callers and tests, FR-447 AC-3, AC-7, AC-9,
AC-10, AC-11 and the S3 portion of AC-15. The Rust review lane is folded into
this review. The changed spec files passed targeted `quire validate`. A
focused Cargo test was attempted through `locked-build.sh` after confirming
249 GiB free on `/`, but all build lanes were busy; it was canceled before
compilation. The full gate belongs to the code lead.

The term and formula lowering paths use explicit work stacks, checked name
lookups and typed refusals. I found no new vendoring or narrowed CI workflow.

## Verdict

**CONDITIONAL**: the hyper-expression refusal path contradicts FR-447's S3
contract; the other findings affect diagnostic location and wording. Review
the first finding against the existing FR-171 S2 rule before changing that
boundary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-447 requires a hybrid claim whose `holds` reads a hyper expression to refuse at S3 as `unsupported_construct`/`expression-form`. The shared S2 temporal builder refuses every such `holds` as `UnrepresentedConstruct` before the new hybrid claim checker runs, so the stage and tag differ. FR-171 also requires this S2 behavior for ordinary temporal clauses; route hybrid clauses deliberately or resolve that spec conflict. | qsl-forms/src/temporal_clause.rs:483-484; qsl-semantics/src/check/hybrid/claim.rs:596-608 |
| FND-002 | low | FR-447 places an unsupported clock refusal at the clock clause span. `ClockForm` stores only the quoted string span, which `clock_of` uses; `clock "wall"` is therefore located on `"wall"`, as the test confirms, rather than the whole clause. | qsl-semantics/src/check/hybrid/claim.rs:182; qsl-forms/src/temporal_clause.rs:74-81 |
| FND-003 | low | The replay diagnostic labels every `CheckCause::Hybrid` as occurring in a `hybrid` declaration. A malformed `temporal` hybrid claim takes this arm and is reported as the wrong declaration kind. | qsl-replay/src/spine.rs:958-959 |
