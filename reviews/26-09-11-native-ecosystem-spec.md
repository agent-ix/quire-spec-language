---
id: SR-353
title: "Base specification review — FR-042/TC-121 producer handoff updates"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md, spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md, examples/protocol-handoff/README.md, README.md"
review_set: base
---

## Summary

Base checklist review of the scoped FR-042 and TC-121 edits and the two READMEs that
carry their narrative. The selected set is **base only**: the change composes already
admitted behaviors into the real example and adds no new library, wire or family
semantics, so none of the seven optional analyses applies and the semantic extension
was declined. No `AssuranceProfile` exists in `spec/`, so no profile-driven
`review_selection` overrides the choice. The edits are accurate about what is and is
not delivered; the one substantive issue is a narrative claim the fixture does not
support.

## Verdict

**CONDITIONAL** — one `medium` unsupported capability claim and two `low` items; no
`high` finding.

## Checklist result

- **ID format and uniqueness** — no IDs added, renamed or renumbered. FR-042's ten
  `FR-042-AC-N` criteria and TC-121's numbered steps are unchanged in shape; the edits
  touch only the Description/Notes prose and step bodies. No duplicates, no gaps.
- **FR quality** — FR-042 keeps its `implements` link to US-004 and its full
  `depends_on` / `references` set including `quire-protocol/IT-001`. The new prose
  narrows rather than widens the claim: it names what the recipe retains, then states
  "General dynamic choice/progress proofs, first-class relationship exports, runtime
  recovery and the actual B public consumer handoff remain open" and "A's emission and
  local reader check do not satisfy FR-042-AC-10 by themselves." Definedness is not
  presented as protocol choice validity anywhere in the new text.
- **TC quality** — TC-121 step 1 and step 10 gain detail without changing Type,
  Priority or Trace. Step 10's addition ("Retain all four original source files and the
  actual executing producer binary… Reconstruct its admitted model through the existing
  located rule-model frontend; do not replace the model or binary with invented
  metadata") correctly keeps `expected.json` inspection-only and forbids a
  self-authorizing inventory, consistent with the existing "neither the payload nor the
  fixture's `expected.json` authorizes its own selections".
- **Six coverage rules** — unchanged. No new AC was introduced, so the
  every-AC-has-a-TC rule is not disturbed; option-permutation, constraint-boundary,
  error-path, state-transition and edge-case obligations continue to sit on the
  existing AC-1…AC-9 groups. The new narrative capabilities (four units, cross-unit
  dependencies, queries, population/reference roles, admitted model operations) fall
  under existing AC-1 and AC-4 and need no new criterion.
- **Cross-referencing** — relative targets (`../../examples/protocol-handoff/README.md`,
  `../../docs/compiled-protocol-v1.md`) resolve. Terminology is consistent with the
  wire contract: "population"/"reference"/"operation"/"compensation" are used as the
  emitted export and record kinds, not loosely.
- **Relationship discipline** — the edits explicitly refuse to let ordinary exports
  stand in for first-class producer relationships ("Ordinary object/reference exports
  do not supply those relationship authorities"). Verified against the emitted package:
  all 42 `Flow` bindings carry `relation: null`, so the spec text matches the artifact.

## Findings

| ID      | Severity | Summary                                                                            | Refs                                                             | Escape Cause      |
| ------- | -------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ----------------- |
| FND-001 | medium   | "ordered queries" overstates the recipe: one of six query operators, order unobservable | spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:38 | wrong-requirement |
| FND-002 | low      | Root README asserts the recipe as delivered evidence without naming its manual-only gate | README.md:48                                                     | wrong-requirement |
| FND-003 | low      | "static compensation registration, activation, retries" reads as new capability, not composition | README.md:48                                                     | wrong-requirement |

### FND-001 — "ordered queries"

TC-121 step 1 and the example README both say the recipe exercises "ordered queries".
The emitted package contains three aggregate sites: one `size` in `Healthy` and three
`query` values with `operator: "sum"` (one in `Healthy`, one per compensation
recovery). The wire's query operator set is `forall | exists | filter | map | count |
sum` (`src/protocol_artifact/wire.rs:336`), so the recipe covers one operator of six,
and FR-042-AC-4's "all eight query forms" obligation stays with the library tests, not
the recipe. More importantly `sum` and `size` are both order-insensitive: nothing in
the fixture distinguishes an ordered sequence from an unordered one, so "ordered"
describes the model's `sequence` declaration rather than anything the recipe
demonstrates. The same overstatement appears in the authored source comment
"Receipt order and duplicates survive the bounded aggregate"
(`examples/protocol-handoff/state.body.native:2`) — duplicates do survive `sum`, order
is not observed. Suggested wording: "a bounded `sum` aggregate and a `size` over the
declared receipt sequence".

### FND-002 — delivered evidence with a manual-only gate

`README.md` now presents the Rust producer recipe alongside "Real source-to-reader
tests exercise native predicate, state, temporal and protocol emission". The tests
sentence is accurate and backed by `tests/native_compensation_emission.rs` and
`tests/native_protocol_emission.rs`; the recipe sentence is placed as if it carried the
same standing, but no automated gate executes the recipe (SR-351 FND-001, SR-352
FND-004). A half-sentence noting that the recipe is run on demand rather than by the
default suite would keep the README honest about the difference.

### FND-003 — composition read as capability

"including static compensation registration, activation, retries and full/partial
recovery requirements" appears in the root README's description of what the tests
exercise. Those behaviors landed in PR57 and are already admitted; the selected review
set for this change is base precisely because nothing new is being claimed. The
sentence is not wrong, but a reader diffing the README will attribute the compensation
capability to this change. Not worth blocking on; noted so the attribution is on
record.

## Coverage note

Base checklist only. The seven optional analyses (failure-domain, integrity,
dependency, evidence, risk-complexity, scope-boundary, ears-conformance) were not
selected and were not run; the optional semantic gap extension was declined. Spec
grammar was validated by the root's completed gate — 398/398 documents grammar-clean,
0 grammar findings (`/tmp/quire-native-ecosystem-final-spec.log`) — and was not
re-run for ceremony.
