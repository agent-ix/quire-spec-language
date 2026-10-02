---
id: SR-1218
title: "Gap analysis of quire-spec-language PR #600: identity precondition test traced to FR-258-AC-5"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0657b2b9a9065d4517a0c99ecbceedda8a414afd; PR #600 tests qsl-semantics/src/check/lowering/tests/identity_depth.rs, qsl-package/src/emit/tests/identity_depth.rs against FR-258, FR-259, ADR-030 D-4.3/D-4.4, spec/tests.md TC-725"
review_set: subset
---
# Gap analysis of quire-spec-language PR #600

## Summary

The four depth tests are traced to TC-725 / FR-258-AC-5. AC-5 states the
stratified body grammar: no argument holds a term of its own stratum, and
every composite subterm is a `reference`. The tests prove something else:
the precondition from ADR-030 D-4.3/D-4.4 that every identity preimage has a
schema-fixed depth.

The grammar implies the fixed depth only through an argument that no AC
spells out. No AC in FR-258, FR-259 or FR-264 states the fixed depth. Yet it
is the precondition for deleting `MAX_CHECKING_DEPTH` and the node-key depth
guard, so it should have its own AC.

D-4.4 lists the identity preimages that must be fixed-depth:
- node keys, type nodes and the v2 package identity: covered;
- nominal preimages: covered where the fixtures declare them;
- compound-unit ids, enum preimages and the checked package identity: not
  exercised.

Those three are flat term or member lists by type, so a source-depth
regression is not reachable there today.

## Verdict

Changes requested: one medium trace finding and one low coverage finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The four depth tests are traced to FR-258-AC-5, the stratified-grammar AC. No AC states the B0 precondition they test, which is a schema-fixed preimage depth. Add an AC, for example FR-259-AC-4, plus a TC, and retag the four tests to it. Proposed wording below. | qsl-semantics/src/check/lowering/tests/identity_depth.rs:46,70,98; qsl-package/src/emit/tests/identity_depth.rs:31 |
| FND-002 | low | Of the D-4.4 preimages, compound-unit ids, enum declaration and member preimages, and the checked (non-v2) package identity are not compared across depths. Each is a flat list by type, so this is low. Either name them in the new AC as fixed by type, or add a case. | spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md:289-300 |

### Proposed AC wording for FND-001

FR-259-AC-4: "The deepest JSON array or object nesting is the same at both
depths of each pair below. It is measured over every node preimage of the
checked package (node keys, type nodes and nominal preimages) and over the
emitted v2 package's identity preimage. The pairs are:
- each FR-258 expression form at 4 and at 63 levels;
- a parameter typed with 4 and with 100 nested `Option`s;
- a chain of 4 and of 30 records."

## Dispositions

Round 1, reviewed at d48ce4286bedc187447f31e33b35e73cc86ef3e7.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d48ce428: FR-259-AC-4 and TC-728 step 5 added; the four tests are retagged to TC-728 / FR-259-AC-4. |
| FND-002 | fixed | d48ce428: FR-259-AC-4 names compound-unit ids, enum preimages and the checked package identity as fixed-depth by type. |
