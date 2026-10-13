---
id: SR-8711
title: "Spec review of quire-spec-language PR #806 (QSL-742): FR-056 signature refusal paragraph and FR-056-AC-32"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@17468cbaacb0946f6bb50a72e20f402a81e6bd0d; spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/test-cases/TC-997-clause-or-field-naming-a-variant-type.md, spec/tests.md"
review_set: subset
---
# Spec review of quire-spec-language PR #806

## Summary

Ticket: QSL-742. Method: spec-review base over the PR's spec hunks. The PR adds an FR-056 "Variant types" paragraph on operation signature refusals, adds FR-056-AC-32, extends TC-997 with step 4, and updates TC-997's row in spec/tests.md. The target design source is QSpec PR #242 (QSpec FR-154-AC-11, TC-235 M29), compared by id only. `quire validate` on FR-056 and TC-997 reports no errors.

Examined:
- FR-056 operation-signature paragraph, FR-056:523-533 (examined)
- FR-056-AC-32 (examined)
- TC-997 description, step 4 and its expected result (examined)
- spec/tests.md TC-997 row (examined)
- FR-056 field paragraph and FR-056-AC-28 (context_only)
- QSpec FR-154 "every declaration refusal retains the IR node identity and the node's FCD origin" and QSpec native-diagnostics `malformed-declaration` row (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new paragraph and FR-056-AC-32 require the node and path but say nothing about origin. QSpec FR-154 says every declaration refusal keeps the IR node identity **and** the node's FCD `origin` verbatim. QSpec native-diagnostics says `malformed-declaration` keeps node, member path and origin. The implementation leaves artifact and span `None` (SR-8709 FND-002). The QSL spec should either require the origin, or state the exception and why. As written, a reader cannot tell whether the missing origin is intended. The FR-056 field paragraph and FR-056-AC-28 have the same gap. | spec/functional/FR-056-admit-domain-package-model-declarations.md:523-533,730 |
| FND-002 | low | "an enum" is listed as a meaning other than a variant type, and FR-056-AC-32 has a separate "naming an enum" clause. In QSL the only `Enum` record is the one intake produces from a variant type (intake.rs:3849). So "naming an enum" is the `Tier` case, not a separate test. Fix: drop "enum" from the list, or name a meaning that does form a separate case (for example an event type). | spec/functional/FR-056-admit-domain-package-model-declarations.md:526-527,730 |
| FND-003 | low | "Variant types are owned by QSL-738 as for fields." is ambiguous. The field paragraph says outright that its refusal is a placeholder QSL-738 replaces. This sentence does not say whether the signature refusal is also a placeholder. Fix: "This refusal of a variant type is a placeholder that QSL-738 replaces, as for fields." | spec/functional/FR-056-admit-domain-package-model-declarations.md:533 |
| FND-004 | low | TC-997's title ("A clause or field naming a variant type refuses while the declaration is admitted") does not mention operation signatures, which step 4 now covers. It also appears in spec/tests.md. Fix: "A clause, field or operation signature naming a variant type refuses while the declaration is admitted". | spec/tests.md:220; spec/test-cases/TC-997-clause-or-field-naming-a-variant-type.md |

## Verdict

The requirement is sound and testable, and it matches the target design:
- The operation node, `Operation.params[i].typeRef` with a zero-based `i`, and `Operation.returns.typeRef` match QSpec FR-154-AC-11 and TC-235 M29.
- The dangling case is scoped correctly to a package built without the reader.
- FR-056-AC-32 is unique across every remote branch and open PR. #804 holds AC-29..31, and no branch uses AC-32 or above.

Merge interplay:
- With #804, FR-056 conflicts twice, both as adjacent inserts: #804's "Effective identity of a variant type" section after this paragraph, and its AC-29..31 rows next to AC-32. Resolve by keeping both, rows in order 28, 29, 30, 31, 32. normalize.rs, variant_type.rs and spec/tests.md merge cleanly.
- #805, #792 and #802 merge with no conflict.

Public-repo check: the hunks cite only QSL ticket ids and no QSpec internals.

No finding blocks merge. FND-001 should be settled together with SR-8709 FND-002.
