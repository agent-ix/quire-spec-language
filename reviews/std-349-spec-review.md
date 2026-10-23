---
id: SR-16213
title: "Spec review of quire-spec-language PR #1306: STD-349, FR-298 render refusal, source provision, trace DOT, limits and cancel"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1ffd5762ba12d6e3d1eeae981221f00faa52956f; PR #1306 (git diff origin/main...1ffd5762): spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md, spec/functional/FR-298-render-outcomes-and-views-from-typed-values.md, spec/test-cases/TC-783-render-produces-text-and-dot-from-typed-values-only-leaving-json.md; read for context: QSL FR-285, FR-286, FR-294 (main and PR #1316/#1323 heads); agent-ix/quire-specification main FR-271, FR-272 and unmerged branch spec/std-349-invalid-quire-context-cause@314d3fd0"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-298
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-783
    type: reviews
---
# SR-16213: Spec review of quire-spec-language PR #1306: STD-349

## Summary

Ticket: STD-349. Reviewed at the current head 1ffd5762, not the 6b5f4715 named in the dispatch: the author pushed "cite QSpec unsupported-render-form; FR-255 text shared with STD-350". Sub-analyses folded in: integrity, EARS, criterion strength (manual, no Jev), cross-repo fidelity, id uniqueness, scope-deferral check. Examined: FR-298's display-form, source provision, trace DOT shape, limits and cancellation, and unsupported-form sections; Inputs, Outputs and Behavior; FR-298-AC-5 to AC-10; the FR-255 rows and stage-limit cell; the FR-275 `render` row; TC-783 steps 5-10. Main is an ancestor of the head. FR-298-AC-5 to AC-10 are unique across open QSL PR heads.

The exit codes match FR-285 (refusal 20, `Limit`/`Cancelled` 22). `stage_limit_exceeded`/`input-bytes-exceeded` matches QSpec FR-272's row and `format.output_bytes`'s precedent. The AC-6 scalar-column vector is sound. AC-7 to AC-10 are concrete and testable.

## Verdict

**NOT MERGEABLE.** FND-001: the refusal's QSpec cause exists only on an unmerged QSpec branch that has no PR. FND-002: the PR body still names a QSpec follow-up (standing rule). FND-003 is medium. FND-004 and FND-005 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-298 (unsupported-form section, Behavior, AC-5, TC-783 step 5) now cites "QSpec FR-272's cause `unsupported-render-form`" and adds a `depends_on` on QSpec FR-272. QSpec main FR-272 has no `invalid_quire_context` row. The row exists only on branch `spec/std-349-invalid-quire-context-cause` (314d3fd0, FR-272-AC-24), which has no open PR. Merged alone, QSL would cite a catalog cause QSpec does not define. Open the QSpec PR (with its gates) and merge it first or together. | spec/functional/FR-298-render-outcomes-and-views-from-typed-values.md:101,131 |
| FND-002 | high | scope-deferral: the PR body's decision 1 still says "QSpec FR-272 has no cause row for `invalid_quire_context` ... If the owner wants a QSpec cause spelling, that is a QSpec follow-up". That names a follow-up, which the standing rule forbids, and it contradicts the head, which now cites the cause. Update the body to link the QSpec PR and drop the follow-up wording. | PR #1306 body, Decisions 1 |
| FND-003 | medium | The diff now adds FR-255's `prepare.work_units`/`prepare.code_bytes` rows, the `FR-294 (prepare.*)` owner entry and `prepare.*` in the stage-limit cell. These belong to STD-350/#1316. On main FR-294 defines no `PrepareLimits.work_units`, and open #1323 (STD-369) names the same setting `prepare.compile_work_units`. If #1306 merges first, FR-255 cites fields nothing defines. Keep this PR to `render.*` and let the PR that owns `prepare.*` add those rows. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:80,130-131,274 |
| FND-004 | low | "Their exact bytes are not normative: the structure this requirement states is" and "attribute spelling, layout and whitespace are not [normative]" describe the spec, not the system. Say what render guarantees: the stated structure, and byte-equal output for equal inputs. | spec/functional/FR-298-render-outcomes-and-views-from-typed-values.md:59-61 |
| FND-005 | low | "a provision whose bytes disagree with its key cannot be built" holds only if `RenderSources` is built from `Source` values and keys each one by its own digest. No constructor states this, and no AC tests it. A plain map type would let a caller insert a mismatched key. State the constructor, for example `RenderSources::from_sources`. | spec/functional/FR-298-render-outcomes-and-views-from-typed-values.md:67-71 |
