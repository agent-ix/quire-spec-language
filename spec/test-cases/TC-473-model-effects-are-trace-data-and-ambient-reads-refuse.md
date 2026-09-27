---
id: TC-473
title: "Model effects are trace data, and ambient-state reads refuse at S3"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: verifies
---
# TC-473: Model effects are trace data, and ambient-state reads refuse at S3

## Description

Verify that simulating a model performs none of its effects: each step's
creations, deletions and field changes appear as typed data, the run reads
nothing ambient and changes no input, and source forms that read ambient
state refuse at S3 with their location.

Scope: FR-120-AC-9, FR-120-AC-10.

## Test Procedure

Integration tests in `qsl-eval/tests/it/`, with TC-471's package and
compile path.

1. With `increment` (post `Inc`), `spawn` and `remove`, no `pre` clause,
   call `sample_model` over `s0` with seeds until the traces include an
   `increment` step from `v0`, a `spawn` step creating `c2` and a `remove`
   step. Read each step's `StepEffect`.
2. Explore and sample step 1's package in a process whose working
   directory is an empty temporary directory, and compare with step 1 and
   with an exploration run in the repository directory. Read `s0`'s key
   before and after exploring.
3. Compile through S1 to S3, in turn, units holding: a function whose body
   reads `self`; an invariant whose body reads `result`; an invariant whose
   body reads `pre(self.value)`; a function whose body calls `reaches(x, y,
   next)`.

Tag the tests `#[trace("TC-473", "FR-120-AC-n")]`.

## Expected Results

- Step 1: the `increment` step's effect is `created: []`, `deleted: []`,
  `changed: [{object: c1, field: value, pre: {"type":"integer","value":"0"},
  post: {"type":"integer","value":"1"}}]`, result none. The `spawn` step
  has `created: [c2]`, no deletions and no `changed` entry for `c1`. The
  `remove` step's `deleted` lists exactly the objects absent from its
  post-state.
- Step 2: equal `Exploration`s and traces in both directories; `s0`'s key
  bytes are equal before and after.
- Step 3: `missing_declaration`/`missing-name` at `self`;
  `wrong_snapshot`/`wrong-anchor` at `result`; `wrong_snapshot`/
  `forbidden-pre-read` at the `pre`; `ill_typed`/`operator-ineligible` at
  the `reaches`. Each refusal's span is that form's source bytes, and no
  `CheckedPackage` is produced.

## Status

Planned (QSL-274).
