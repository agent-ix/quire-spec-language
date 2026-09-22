---
id: TC-249
title: "Clause identity is the checked node id; the occurrence key disambiguates structurally identical clauses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: verifies
---
# TC-249: Clause identity is the checked node id; the occurrence key disambiguates structurally identical clauses

## Description

Verify that a clause's identity is the checked node id of its `claim`,
`temporal` or `protocol` node, and that two occurrences of a structurally
identical clause carry the same node id but distinct occurrence keys (node
id, role, ordinal), so a consumer keying on the pair — never the node id
alone — tells them apart. This is R-05's adverse-test requirement applied
to clause identity: display text, diagnostic text and iteration order must
not be load-bearing. Scope: FR-088-AC-3.

## Test Procedure

1. Author a source package with the identical clause expression written
   twice, at two distinct source positions (for example, the same
   postcondition text repeated on two different functions).
2. Check the package and record each clause's checked node id and
   occurrence key.
3. Confirm both clauses' node ids are equal (structurally identical clauses
   share one id, ADR-013 O-04) and their occurrence keys differ.
4. Confirm a lookup keyed on (node id, occurrence key) returns the correct
   clause for each occurrence, while a lookup keyed on node id alone cannot
   distinguish them (and this requirement's own consumer code never
   performs the latter lookup where the two must be told apart).
5. Adverse test: rename the two functions (changing display/diagnostic
   text) and re-check; confirm both clauses' node ids and occurrence keys
   are unchanged. Separately, iterate the two clauses in each of two
   different collection orders and confirm neither clause's (node id,
   occurrence key) identity depends on the order it is iterated in.

## Expected Results

- Step 3: equal node ids, distinct occurrence keys.
- Step 4: (node id, occurrence key) disambiguates; node id alone does not,
  and no production code path relies on node id alone where disambiguation
  is required.
- Step 5: neither renaming nor iteration order changes either clause's
  identity or occurrence key; a test that shows either changing fails this
  step.
