---
id: SR-8717
title: "Spec review of quire-spec-language PR #807 (QSL-737): FR-088 AC-8 note and TC-260 step 3"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ff905e16d51205409fddb3834f869343bad65eed; spec/functional/FR-088-clause-name-and-type-identity.md, spec/test-cases/TC-260-value-type-ref-is-exactly-two-member-union.md"
review_set: subset
---
# Spec review of quire-spec-language PR #807

## Summary

Ticket: QSL-737. Method: spec-review, base checklist only, over the two spec hunks PR #807 changes. The FR-088 AC-8 note and TC-260 step 3 now describe the raw-wire allowance for `RawRecord.type_identity`. The reasoning is sound. One wording defect in TC-260 step 3 makes the step claim more than the scan enforces.

Examined:
- FR-088-AC-8 and its scope note, FR-088:245 and 255-259 (examined)
- TC-260 step 3 and Expected Results, TC-260:26-34 and 42-46 (examined)
- FR-087-AC-6 and FR-087-CON-3, FR-087:584 and 596 (context_only; their text did not change, and the PR depends on them for TC-255)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-260 step 3 calls `RawRecord.type_identity` "the one allowed exception" and ends "A bare type field anywhere else, or under another name, fails." Both overclaim. First, step 3's own scope ("the places that record a field's type for a domain-model declaration") already leaves out a runtime value's wire type claim, so this is a field outside the rule, not an exception to it. Second, a type held under a field name without `type` in it, or in an enum-variant field (for example `RawNode::Record.type_identity: String` in the same file), does not fail. FR-088's note says "a field type under another name is not covered". Fix: "`RawRecord.type_identity` in `observation.rs` is not a model field-type site: it is the raw view of a snapshot value's wire `type` string, resolved by lookup at FR-106 check 6. The scan skips that one field." Then drop "or under another name, fails", or bring it in line with the FR-088 note. | spec/test-cases/TC-260-value-type-ref-is-exactly-two-member-union.md:31-34; spec/functional/FR-088-clause-name-and-type-identity.md:255-259 |

## Verdict

**CONDITIONAL**: one low wording finding. No hash, digest, pin or tracking record was added. IDs and links are unchanged and valid. The FR-088 note gives the allowance as a scope statement next to the coverage notes that are already there, which is the right place for it. It states what the field is (a wire shape held until a checked declaration resolves it by lookup), and the code agrees (document.rs:1296). AC-8's text did not change and does not need to. The allowance does not weaken it, because the field is outside AC-8's subject. FR-087-AC-6 and FR-087-CON-3 back the TC-255 production fix exactly as quoted in the PR body. The enum-variant blind spot in the scan is recorded as a coverage finding in the gap analysis for this PR (SR-8716 FND-001), not repeated here.
