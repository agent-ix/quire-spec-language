---
id: TC-891
title: "A clause-run request file runs through run_clause and writes a /2 document"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-312
    type: verifies
---
# TC-891: A clause-run request file runs through run_clause and writes a /2 document

## Description

Verify FR-312: `read_clause_run_request` reads a `1-draft` request carrying
`clause`, the `run` command and the driver's run entry run it through
`run_clause` and write a `native-run-result/2` document, and the reader
refuses malformed requests and requests past the intake limit.

Scope: FR-312-AC-1 to FR-312-AC-4.

## Test Procedure

1. From FR-108's fixtures, write clause-run requests for healthy-parent and
   violating-parent (`ParentOrder`, the case's current snapshot). Run each
   with `quire-spec run`, then pass the same file to the driver's run entry.
2. Edit the healthy-parent request, one edit per run: add a `call`; remove
   `clause`; add `function` beside `clause` in `selection`; add a member
   `note` to `clause`; point the selection at observation key `ghost`.
3. Set `--dependent-bytes` one below the total size of the request's dependent files, then to that
   total.
4. Write a `function` request for `sameIdentity` over the
   distinct-identities snapshot with arguments `{b: child, a: root}`, and a
   `frame` request; build the same `ClauseRunRequest`s in process and call
   `run_clause`. Set `package_id` to another unit's and run again.

Tag the tests `#[trace("TC-891", "FR-312-AC-n")]`.

## Expected Results

- Step 1: one `/2` document each: `success`, `truth: true`, exit 0;
  `violation`, `truth: false`, exit 10. The driver's stdout bytes and exit
  equal the CLI's.
- Step 2: each refuses `invalid-request` at stage `request`, exit 20, empty
  stdout, naming the member or key.
- Step 3: the intake-limit refusal, then a run.
- Step 4: each document's disposition equals the in-process report; the
  `package_id` run writes a `/2` document with stage `compile`,
  `stale_dependency`, exit 20.

