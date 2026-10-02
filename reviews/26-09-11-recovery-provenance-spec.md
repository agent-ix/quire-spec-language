---
id: SR-356
title: "Base specification review of the shared recovery provenance rule in FR-042 and TC-121"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

`/spec-review` base checklist over the specification half of
`origin/main...a33a3c1`: one new normative paragraph in FR-042, a rewritten
AC-6, one extended TC-121 step and a matching wire-contract paragraph. Selected
analyses are base and failure-domain (SR-357); the optional semantic extension
was declined. ID formats, uniqueness, sequence and cross-references are clean —
no ID is added, FR-042 keeps AC-1..AC-10, and AC-6 still traces to TC-121. The
substantive gain is real: the derivation is now stated in the specification
rather than left to whichever implementation the reader happened to read, which
closes SR-349 FND-001. Two residuals concern how the obligation is phrased and
one error path that TC-121 does not require.

## Verdict

**CONDITIONAL** — three mediums, one low, no high. The new paragraph is specific
and testable in substance; the defects are an implementation-coupled SHALL, a
criterion that keeps absorbing conditions, and a stated refusal with no required
test.

## Findings

| ID      | Severity | Summary                                                                                                      | Refs                                                                        | Escape Cause        |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------- |
| FND-001 | medium   | "The compiler and reader SHALL use one shared dependency rule" obliges an implementation-sharing property no independent reader can satisfy or exhibit | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:174            | wrong-requirement   |
| FND-002 | medium   | FR-042-AC-6 absorbed a sixth sentence instead of being split; it now spans providers, cardinality, registration, retries, commit, recovery provenance, typed effects and five refusal classes | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:216            | wrong-requirement   |
| FND-003 | medium   | Error Path coverage rule: the new "another compensation's phase anchor refuses" clause is not among the mutations TC-121 step 6 requires | spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:102; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:181 | missing-requirement |
| FND-004 | low      | The rule is restated in near-identical prose in FR-042 and `docs/compiled-protocol-v1.md` with no stated authority between them | docs/compiled-protocol-v1.md:465                                            | missing-requirement |

### Checklist results

- **ID format and uniqueness** — pass. No new US/FR/TC/AC/CON id; AC ids stay
  `FR-042-AC-1..10`, sequential and unique; TC-121 keeps its step numbering.
- **Functional requirement quality** — behaviour is detailed and specific: the
  traversal roots, the three edge classes, the self-selected exclusion, the
  span/proof-folding exclusion, the call-argument rule, the selection target and
  the ownership boundary with model validation. It names its own refusal
  (`another compensation's phase anchor refuses`) but, as elsewhere in FR-042,
  not the typed cause; the wire contract carries `Invalid::Binding` in the code,
  not the requirement. Consistent with the document's existing style, so noted
  rather than filed.
- **Test coverage — six rules** — Coverage: AC-6 has TC-121. Error path: see
  FND-003. Edge case: TC-121 now names the discriminating cases explicitly (an
  unread capture with an initializer outside `recover`, a discarded conditional
  operand, a neighbouring obligation, a self-selected marker, proof
  simplification), which is the right level of specificity and is what the new
  test implements. State transition and option permutation are not applicable.
- **Cross-referencing** — pass; full ids throughout, terminology consistent with
  the existing "population/closure pair", "original operand" and "phase anchor"
  vocabulary.

### FND-001 — an observable obligation is already available

"The compiler and reader SHALL use one shared dependency rule" states how the
code is organised. A third-party reader written against the wire contract cannot
share this repository's Rust module and so cannot conform, while a repository
that duplicated the rule correctly would conform in effect but not in letter.
The observable statement already exists one paragraph away in AC-6 — "Producer
and reader derive identical recovery population membership" — and the paragraph
that follows supplies the rule itself. Recommend making the SHALL the equality
plus the rule definition, and leaving single-implementation sharing as the
repository's design note.

### FND-002 — AC-6 keeps growing

At `c7275f5` AC-6 was flagged for holding eleven refusal conditions in five
sentences (SR-349 FND-002, carried open). This increment adds a sixth sentence
covering producer/reader membership equality and the span/proof-folding
exclusion. The criterion is still verifiable, but it can no longer fail for one
identifiable reason, and a matrix row backed by it cannot say which condition a
test covers. A split — recovery provenance as its own AC — costs one id and
would make both this increment and the previous one individually traceable.

### FND-003 — the refusal TC-121 does not ask for

TC-121 step 6 gained the positive derivation case, the span/proof
discrimination, the unread-capture and neighbour cases, the call-argument case,
and the omitted/unrelated pair refusals. It did not gain the phase-anchor
refusal that the same edit added to FR-042 line 181. The base checklist's Error
Path rule requires every stated error condition to be tested; this one is
stated, implemented and untested. See SR-355 FND-002 for the evidence side.
