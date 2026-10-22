---
id: TC-1160
title: "A temporal clause run travels the clause-run request and result wires"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-312
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: verifies
---
# TC-1160: A temporal clause run travels the clause-run request and result wires

## Description

Verify that a temporal clause is selected by the FR-312 request wire, that
every FR-330 temporal disposition is written and read as
`native-run-result/2` under FR-267's temporal disposition wire, and that a
refused `holds` condition settles as a refusal at its position.

Scope: FR-312-AC-7, FR-267-AC-13, FR-267-AC-14, FR-327-AC-6, FR-330-AC-7.

## Test Procedure

Use TC-840's `Counter` unit (clauses `Bounded`, `Reaches`, `Fair`) and
snapshots with `c.value` 0, 1, 2, and TC-847's undefined clause.

1. Through `run`, send FR-312-AC-7's temporal request over the lasso with
   loop `s0`, `s1`, then over loop `s0`, `s1`, `s2`; build the same
   `ClauseRunRequest` in process and compare. Send each malformed request
   of FR-312-AC-7.
2. Write FR-330-AC-2's lasso violation report; check its bytes against
   FR-267-AC-13 and their RFC 8785 re-encoding; wrap its `counterexample`
   in an FR-070 envelope naming `Reaches` and replay it through FR-331.
   Write FR-330-AC-1's `Bounded` violation and read its `interval`.
3. Write and read back each report FR-267-AC-14 lists; apply each edit
   FR-267-AC-14 lists to the documents and read them.
4. Evaluate FR-327-AC-6's clause with a stub `holds` atom over the finite
   trace, the infinite-trace prefix and the lasso.
5. Run FR-330-AC-7's clause from source over `c.value` 0, 2.

Tag the tests `#[trace("TC-1160", "<criterion>")]` with the criterion each
step asserts.

## Expected Results

- Step 1: exit 10 then exit 0, stdout byte-identical to the in-process
  report's document; each malformed request refuses `invalid-request` at
  stage `request`, exit 20, empty stdout, naming `trace` or the member; the
  empty `finite` array reports stage `evaluate`, `refusal`.
- Step 2: the document of FR-267-AC-13; the replay settles
  `reproduced-with-evaluated-witness`; `interval` is
  `{"lower": "0", "upper": "1"}`.
- Step 3: each report reads back to an equal disposition with its table
  row's members; each edited document refuses with a located diagnostic.
- Step 4: the refused condition at position 1 with
  `integer_out_of_domain`/`outside-domain`; `Completed(false)` at position 0
  when position 0 is false; an internal failure for `CheckedInvariant`.
- Step 5: stage `evaluate`, `refusal`, exit 20, position 1, `outcome`
  `refused` with the record and locus; no `truth`, no `internal_fault`.
