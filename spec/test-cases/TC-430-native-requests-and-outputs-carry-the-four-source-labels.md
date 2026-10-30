---
id: TC-430
title: "Run requests and their outputs carry the two source labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: verifies
---
# TC-430: Run requests and their outputs carry the two source labels

## Description

Verify that the native-run/1 request wire requires both FR-001 labels on a
`1-draft` program source, that `spine-run-result/1` renders them with
no other identity member, and that a request missing a
label refuses as malformed. This catches a label defaulted by the command.
It also verifies that the clause-run request accepts `program.extraction`
only in the `quire-extraction` build and refuses extraction on a function call.

Scope: FR-026-AC-6, FR-027-AC-4, FR-031-AC-5 and FR-031-AC-6.

## Test Procedure

1. Run a native-run/1 request selecting `tests/fixtures/spine-compile.native`
   whose program source, model source and snapshot selection each name
   `authority` `agent-ix` and their own `identity`.
2. Run it again without the program source's `authority`, then without the
   snapshot selection's `identity`, then with a blank `authority`.
3. Compile a native-compile/1 request with the same source labels and validate
   the package bytes against the native-linked-package/1 schema.
4. Compile it again without `identity`.
5. Through the command built with `--features qsl-replay/quire-extraction`,
   run a `1-draft` FR-312 clause-run request whose program selects an original
   Markdown document and whose extraction body record names `authority`
   `agent-ix`, `identity` `b`, and `document` `B`. Then run it without the body's `authority`. Run the valid request
   through the command built with no features.
6. Run an ordinary `1-draft` function-call request carrying `program.extraction`
   through the command built with no features and with
   `--features qsl-replay/quire-extraction`.

Tag steps 1 and 2 `#[trace("TC-430", "FR-026-AC-6")]`, steps 3 and 4
`#[trace("TC-430", "FR-027-AC-4")]`, step 5 `#[trace("TC-430", "FR-031-AC-5")]`, and step 6
`#[trace("TC-430", "FR-031-AC-6")]`.

## Expected Results

- Step 1: the result renders each selection with its two labels and no
  other identity member.
- Step 2: the first two refuse at the request stage with `invalid-request`,
  exit 20; the blank label refuses with `invalid_source_identity`, cause
  `blank-label`, `label` `authority`.
- Step 3: the package `source` names the two labels, and the
  schema accepts it.
- Step 4: `invalid-request`, exit 20.
- Step 5: the valid feature-enabled clause run produces the FR-267 `/2`
  document with the extraction member rendering the native identity
  (`agent-ix`, `b`) and formal identity (`B`); missing
  authority refuses `invalid-request`. The no-feature
  build refuses `invalid-request` at stage `request`, naming `extraction`,
  exit 20, with empty stdout.
- Step 6: the no-feature call request refuses `invalid-request` naming
  `extraction`; the feature-enabled call request refuses
  `extraction-requires-run`; neither reads source or model files.

Step 2's blank-label refusal renders a byte-0 span in the run output: the
native `Diagnostic`'s retained debt (FR-001, "Where an S0 refusal is located"),
not this requirement's behaviour.
