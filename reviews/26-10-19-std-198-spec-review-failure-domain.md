---
id: SR-13060
title: "Failure-domain review of PR #1016: refinement_internals replay, malformed spellings and refusal loci"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@6f2f80ba5; spec/functional/FR-147 (The profile's internal list, Internal abstract steps, AC-6); against QSL ADR-015, QSpec FR-323 and FR-272"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
---

## Summary

Ticket: STD-198. Looks for failure modes the new member opens: replay
reproducibility, input it does not classify, and refusals with no locus.
Nothing in the member makes the verdict depend on anything outside the
checked package. The failure modes found are a replay path that cannot
reproduce the list, spellings no refusal covers, a refusal payload with no
node to name, and object order whose effect on identity is unstated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `abstract_internal` enters the `package_id` (Reading), but the member is not in the lock `sources` that replay recompiles from (ADR-015; QSpec FR-323-AC-7). If it lives in a compile request or another side input, replay recompiles with no member. The list is then empty, the `package_id` differs, and every proof with a non-empty internal list fails replay. The spec does not say that the member enters the lock or the replay request, or how replay obtains it. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:99-108,118-125; spec/decisions/ADR-015-compile-and-replay-against-dependencies.md:167-173,309-311 |
| FND-002 | medium | Defects classify the member's shape, an unknown `refinement` and repeats. Internal abstract steps refuses `missing-name` for an entry that names no node, channel or memory kind. Neither says what happens to a string that is not a class spelling: `"frob x"`, `"spawn Spec::Twice::r"`, `"Spec::Twice"`, an empty string, or a wrong alias `"Other::Twice::finish"`. One implementer will refuse `malformed-declaration`, another `missing-name`. Add one rule and an AC-6 vector. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:129-136,142-145,191 |
| FND-003 | medium | Member defects refuse `invalid_model_binding`/`malformed-declaration` "naming the member path". FR-272's payload for `malformed-declaration` is "the IR node identity and invalid member path", and FR-135 refusals are a `CheckRefusal` with a span. A JSON member has no IR node and no source span, and the path format (JSON Pointer, e.g. `/refinement_internals/0/internal`) is not stated. The `missing-name` refusal for `"Nope"` has the same gap: FR-272 asks for an owning scope and a referring locus. State the locus and the path syntax so that AC-6's "naming the member path" can be asserted. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:129-136,191; quire-specification spec/objects/foundation/FR-272-typed-diagnostic-cause.md:71,75 |
| FND-004 | low | The spec does not say whether the order of objects in the array, or of entries within one `internal` array, affects anything. Under Reading it should not, because the resolved list is sorted. `dependency_selections` instead refuses a member that is out of order. State that input order is free, and add a permuted member to TC-552 step 5's equal set. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:110-125 |

## Verdict

Not mergeable on FND-001: whether the member is replayable depends on its
host document, which the PR leaves open (SR-13059 FND-001). FND-002 to
FND-004 are classification and locus gaps that AC-6 needs filled to be
testable. Clean: absence is empty and not a refusal, which matches QSpec
FR-432-AC-5. A model-subject refinement with a non-empty list refuses. A
member object for a refinement that does not exist refuses `missing-name`.
