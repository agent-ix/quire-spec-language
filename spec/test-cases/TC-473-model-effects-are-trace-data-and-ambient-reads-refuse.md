---
id: TC-473
title: "Model effects and results are trace data, and ambient-state reads refuse at S3"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: verifies
---
# TC-473: Model effects and results are trace data, and ambient-state reads refuse at S3

## Description

Verify that simulating a model performs none of its effects: each step's
creations, deletions, field changes and result appear as typed data, the
run's only input is its request and it keeps every input's value, and
source forms that read ambient state refuse at S3 with their location.

Scope: FR-120-AC-10, FR-120-AC-11, FR-120-AC-12.

## Test Procedure

Integration tests in `qsl-eval/tests/it/`, with TC-471's package and
compile path. Each step effect is read from a `ModelTrace` built by
`replay` of a hand-written FR-101 `Trace` (its transition identities and
state-key digests written out in the test), which is the path
`sample_model` takes after sampling.

1. `isZero(): Boolean` (empty frame, `post Z { result = (self.value = 0)
   }`): expand `v0` and `v1`, and read `domains()`. Replay the one-step
   trace `v0 --isZero[<c1>]--> v0` and read its effect.
2. `increment` with `post Inc`, `spawn` and `remove`, no `pre` clause.
   Replay each one-step trace:
   - `v0 --increment[<c1>]--> v1`;
   - `s0 --spawn[<c1>]--> s0 ∪ {c2: value 1, label 0, next absent}`;
   - `w --remove[<c1>]--> {c2: value 0, label 0, next absent}`, where `w`
     is {`c1`, `c2`}, each `value` 0, `label` 0, `next` absent, as an
     initial snapshot.
   Read each effect.
3. Explore step 2's package in the test process, reading `s0`'s key bytes
   before and after. Then run the same exploration in a child process of
   the test binary (`std::process::Command` on `std::env::current_exe()`
   with an ignored test filter), started with `current_dir` set to a fresh
   empty `tempfile::TempDir`, which prints the `Exploration`'s `Debug` text;
   compare it with the in-process `Exploration`'s `Debug` text.
4. Compile through S1 to S3, in turn, units holding: a function whose body
   reads `self`; an invariant whose body reads `result`; an invariant whose
   body reads `pre(self.value)`; a function whose body calls `reaches(x, y,
   next)`.

Tag the tests `#[trace("TC-473", "FR-120-AC-n")]`.

## Expected Results

- Step 1: `v0` has one `isZero` successor, `v0`; `v1` has one, `v1`;
  `domains()` holds the root `{"operation":<isZero's preimage>,"result":
  "result"}` typed `Boolean`. The effect is `created: []`, `deleted: []`,
  `changed: []`, result `{"type":"boolean","value":true}`.
- Step 2: `increment`: `created: []`, `deleted: []`, `changed: [{object:
  c1, field: value, pre: {"type":"integer","value":"0"}, post:
  {"type":"integer","value":"1"}}]`, result none. `spawn`: `created:
  [<c2>]`, `deleted: []`, `changed: []`, result none. `remove`:
  `created: []`, `deleted: [<c1>]`, `changed: []`, result none.
- Step 3: `s0`'s key bytes are equal before and after; the child's
  `Debug` text equals the in-process one.
- Step 4: `missing_declaration`/`missing-name` at `self`;
  `wrong_snapshot`/`wrong-anchor` at `result`; `wrong_snapshot`/
  `forbidden-pre-read` at the `pre`; `ill_typed`/`operator-ineligible` at
  the `reaches`. Each refusal's span is that form's source bytes, and no
  `CheckedPackage` is produced.
