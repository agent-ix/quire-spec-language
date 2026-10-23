---
id: TC-109
title: "Run a selected Markdown clause through the spine clause runner"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: verifies
---
## Description

Integration, P1, in `qsl-replay`'s tests built with
`cargo test -p qsl-replay --features quire-extraction`: extract licensed
generated Markdown through `qsl_source::extract` and run the extracted body
through `qsl_replay::spine::run_clause` over FR-108's fixtures; and, in the
root crate's tests built with `--features qsl-replay/quire-extraction`, run
the actual `quire-spec` binary's extraction mode. Scope: FR-031-AC-1 to
FR-031-AC-4, FR-031-AC-6 to FR-031-AC-10 and FR-267-AC-17.

## Test Procedure

1. Embed FR-108's unit in one `ix:native` fence of an LF document and of a
   CRLF document; extract each and run healthy-parent, violating-parent and
   missing-model.
2. Extract with a clause id the document does not hold; with an extraction
   Quire reports unavailable; with an original whose bytes differ from its
   verified digest; then run a body taken from an `ix:formal` fence.
3. Extract with an original byte limit one below the document's size and a
   line limit one below its line count; run healthy-parent with `work_units`
   0, and compile it under `SpineLimits` whose checking limit is one below
   the body's checking work; repeat each with default limits.
4. Read step 1's healthy report's source and extraction record; run it
   again with an expected `package_id` taken from compiling another unit.
5. Through the binary, with every model file the request names absent:
   `extraction` with `package`; `extraction` with no `clauses` binding and
   with two; `extraction` with `call` in place of `clause`; a compile request
   carrying `extraction`; a `null` `extraction`, a positional `body` and an
   unknown member in `body`.
6. Through the binary, a binding whose package gives a clause-only context
   Quire's validator rejects; then a request whose extraction Quire reports
   unavailable.
7. Embed in an indented CRLF fence, preceded on its lines by `é`, a body
   whose second line is ill-typed; a body whose refused span ends on the
   final CRLF; a body refused at package encoding; and a malformed
   original-to-body map.
8. Run a body declaring two clauses, a body with a syntax error, and a
   selection naming a clause the body does not declare.
9. Through the binary, run step 1's healthy request and read the `/2`
   document back with the FR-267 strict reader; run the same unit given as
   bytes; read back documents whose `extraction` has an unknown top-level
   member `source`, no `map`, `binding` `null`, a numeric `body.digest`, and
   `map.segments` `[]`.

Tag the tests `#[trace("TC-109", "FR-031-AC-n")]`: steps 1 to 4 for AC-1 to
AC-4 and steps 5 to 9 for AC-6 to AC-10 (AC-5 is TC-430 step 5); step 9 also
`#[trace("TC-109", "FR-267-AC-17")]`.

## Expected Results

- Step 1: `success`, exit 0, with the `package_id` of compiling the body
  alone, for LF and CRLF alike; `violation`, exit 10; `compile`,
  `missing_import`/`missing-selection`, exit 20, no `package_id`.
- Step 2: each of the first three ends with FR-030's error, retaining the
  original identity, the selection and Quire's outcome, with no compile and
  no truth; the `ix:formal` body refuses `unknown_language`, exit 20.
- Step 3: each limited run reports incomplete with that limit's typed
  cause; each default run succeeds.
- Step 4: the source names the body identity with the extracted body's
  digest and the original identity with its digest, and the extraction
  record holds Quire's unchanged outcome and the verified map; the second
  run refuses `stale_dependency`, naming both identities.
- Step 5: `extraction-package-conflict`; `extraction-clause-count` with
  `{"actual": 0}` and `{"actual": 2}`; `extraction-requires-run` twice;
  `invalid-request` naming the member three times; each at stage `request`,
  exit 20, empty stdout, with no model file read.
- Step 6: `invalid-quire-context` at stage `quire-context`, exit 20, with
  Quire's diagnostics unchanged in `details`; then a command-error envelope
  at stage `quire` retaining the original identity, the binding and Quire's
  outcome.
- Step 7: `ill_typed` at compile with the body span unchanged and original
  regions whose byte offsets are the body offsets plus the fence's original
  offset and indentation; every original region of the CRLF-ending span; no
  original region for the package-encoding refusal; FR-030's correspondence
  error before compile for the malformed map.
- Step 8: `extraction-clause-count` at compile with `{"actual": 2}` and no
  `package_id`; FR-109's `source` refusal; `missing_declaration` at `select`.
- Step 9: one `/2` document whose `extraction` equals the in-process record
  and reads back equal; no `extraction` for the bytes unit; each malformed
  `extraction` refuses naming `extraction`.
