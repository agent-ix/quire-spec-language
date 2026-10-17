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

Scope: FR-312-AC-1 to FR-312-AC-6.

## Test Procedure

1. From FR-108's fixtures, write clause-run requests for healthy-parent and
   violating-parent (`ParentOrder`, the case's current snapshot), each with
   `self: child` and the snapshot's `anchor`. Run each
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
5. Build a current clause request with `self` and `anchor`, a pre clause
   request with `self` and `parameters: {}`, and an invocation clause request
   with neither context member. Compare each to the same FR-106 selection
   built in process. For current, change the request anchor to one that
   disagrees with the snapshot. In independent edits omit `anchor`
   from current, omit `parameters` from pre, add `parameters` to
   current, add `anchor` to pre, and add `self`, `anchor` or `parameters` to
   invocation. Select a post snapshot directly, then a document with an
   unrecognized `format`, then a snapshot with an unrecognized `observation`
   role. Omit `self` from a current request whose receiver is an object and
   expect an admission refusal `wrong-observation` at stage `admit`, exit 20.
6. Read a clause request with all ten `accounting` counters set to 1 through
   10, one with `accounting` omitted, and one with a legacy direct
   `work_units` member in `clause`.

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
- Step 5: each valid request gives the same report as its in-process
  selection, using the supplied current or pre `self`, the supplied current
  `anchor`, and the invocation document's own `self`. A mismatched current
  anchor gives FR-106's `wrong_snapshot`/`wrong-anchor` admission report.
  Every missing or forbidden context member refuses `invalid-request` at
  stage `request`, naming that member; a selected post snapshot, unknown
  format or unknown observation role names `observation`. Each request
  refusal writes empty stdout and exits 20 before `run_clause`.
- Step 6: all ten counter values reach `ClauseRunRequest.accounting`
  unchanged; omission uses FR-100's defaults; the legacy `clause.work_units`
  request refuses `invalid-request` at stage `request`, exit 20, empty stdout.
