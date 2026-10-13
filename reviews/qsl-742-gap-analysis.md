---
id: SR-8710
title: "Gap analysis of quire-spec-language PR #806 (QSL-742): signature refusal node and member path"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@17468cbaacb0946f6bb50a72e20f402a81e6bd0d; spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/test-cases/TC-997-clause-or-field-naming-a-variant-type.md, qsl-semantics/src/model, qsl-semantics/tests/it/variant_type.rs, qsl-semantics/tests/it/business_meanings.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #806

## Summary

Ticket: QSL-742. Planless gap analysis of the ticket's acceptance criteria (ticket text treated as data and re-measured against the code) and of FR-056-AC-32 against tagged tests and source. Plan completion: not assessed. Trace measured by grep of `#[trace(...)]` tags, because `quire matrix` did not finish in time.

Examined:
- QSL-742 AC 1: signature_type_refusal "and any sibling that builds a refusal without a node" carries the node and a typed member path (examined)
- QSL-742 AC 2: vectors for a variant-typed parameter at index 0 and 1 and a variant-typed result, asserting code, cause, node and path, with no detail-text assertions (examined)
- QSL-742 AC 3: no refusal constructor without a node in qsl-semantics/src/model outside tests (examined)
- FR-056-AC-32 (examined)
- FR-056 "Variant types" operation paragraph, FR-056:523-533 (examined)
- TC-997 step 4 (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSL-742 AC 2 is not met: no vector has a parameter at index 1. Both tests declare a single parameter, so only `Operation.params[0].typeRef` is asserted (see SR-8709 FND-001). | qsl-semantics/tests/it/variant_type.rs:629-647; qsl-semantics/tests/it/business_meanings.rs:660-668 |
| FND-002 | medium | QSL-742 AC 1 ("and any sibling") and AC 3 are not met. Three bare `ModelRefusalCause::MalformedDeclaration` constructions in qsl-semantics/src/model still drop a node they hold: a relationship end naming a record value type (systems.rs:305, `relationship.key`), a namespace containing a namespace (normalize.rs:2225, `namespace.key`), and a population member type that is a record value type (normalize.rs:2416, `population.key`). Two more have no node to keep, so they are legitimately nodeless: the empty model selection (normalize.rs:1085) and an empty declaration key (normalize.rs:1968). Fix: give the three node-holding sites `IntakeMalformedDeclaration { node, member }` with their member path in this PR, or record a ticket for them and narrow the AC. | qsl-semantics/src/model/systems.rs:303-311; qsl-semantics/src/model/normalize.rs:2223-2230; qsl-semantics/src/model/normalize.rs:2414-2421 |
| FND-003 | medium | Parts of FR-056-AC-32 have no test under its trace. The only test tagged FR-056-AC-32 (the variant test) covers `Tier` alone. The undeclared-type clause is asserted only by `operation_signature_type_meaning_matrix`, which is tagged `FR-103-AC-6` and `QSpec-TC-235`, not `FR-056-AC-32`/`TC-997`, so the matrix shows that clause as unbacked. The FR paragraph also names an interface-featured object type as a forbidden meaning, but no test anywhere declares one in a signature. Fix: add `FR-056-AC-32` and `TC-997` to the matrix test's trace, and add an interface-featured object row to the matrix. | qsl-semantics/tests/it/business_meanings.rs:629; spec/functional/FR-056-admit-domain-package-model-declarations.md:523-533 |
| FND-004 | low | QSL-742 AC 2 says "no detail-text assertions". The matrix test still asserts `refusal.detail.contains(...)` for the value type, the operation node and the position text. These lines were there before, but the PR rewrote this loop and kept them. Fix: drop the detail assertions, now that the typed node and member cover the same facts. | qsl-semantics/tests/it/business_meanings.rs:722-733 |

## Verdict

The core of the ticket is delivered. `signature_type_refusal` keeps the operation node and the typed member path for both causes and both positions, and the result path and the index-0 parameter path are asserted on typed fields. The reader-unreachable dangling case is justified: through the reader, an undeclared `typeRef` is refused earlier, by the semantic-IR `UNRESOLVED_TYPE_REF` mapping to `missing_declaration` (intake.rs:1736-1739), so `unknown-value-type` is reached only by a package built without the reader. That is what FR-056 now says.

The ticket ACs are only partly met: index 1 (FND-001) and the sibling sites (FND-002). FND-003 is a trace and coverage gap against FR-056-AC-32.
