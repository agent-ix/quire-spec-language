---
id: TC-742
title: "native-run-result/2 carries basis and witness, and its strict reader refuses malformed and /1 documents"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: verifies
---
# TC-742: native-run-result/2 carries basis and witness, and its strict reader refuses malformed and /1 documents

## Description

Verify FR-267: the `/2` writer encodes FR-266's `basis` and `witness`
canonically, the reader round-trips them and refuses every malformed
witness, basis and version, and `run`'s command-error envelope is `/2`.

Scope: FR-267-AC-1 to FR-267-AC-3.

## Test Procedure

1. Write the `/2` documents for FR-266-AC-1's `AllBelow` over `high` and
   over `low` reports and FR-108's exhausted-work report. Re-encode each
   document's parsed JSON with the RFC 8785 encoder and compare bytes; read
   each back.
2. Edit the `high` document, one edit per read: remove `witness.index`;
   add a member to `witness`; set `witness.trace_position` to `0`; remove
   `basis`; set `basis` to `closed-scope`, keeping `witness`; remove
   `witness`, keeping `decisive-counterexample`; set `format` to
   `native-run-result/1`; set `format` to `native-run-result/3`.
3. Run `quire-spec run` over a native-run/1 request whose source declares
   `edition "9-draft"`, in the change that lands FR-100's clause runner.

Tag the tests `#[trace("TC-742", "FR-267-AC-n")]`.

## Expected Results

- Step 1: `format` `native-run-result/2`; `decisive-counterexample` with the
  `forall` occurrence key, the reference to `mid`, `index` 1, the
  population-root path and no `trace_position` member; `closed-scope` and
  `unavailable` with no `witness` member. Each document's bytes equal its
  re-encoding, and each read gives the written basis and record.
- Step 2: eight refusals, each naming the edited member; the two `format`
  edits refuse under the unsupported-version rule.
- Step 3: exit 20, empty stdout, a stderr envelope holding exactly
  `format` `native-run-result/2`, `stage` `profile`,
  `code` `unknown_edition`, `message`, `details` and `basis` `unavailable`,
  with no `witness` member.
