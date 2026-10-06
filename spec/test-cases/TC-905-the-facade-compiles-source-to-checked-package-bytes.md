---
id: TC-905
title: "The replay facade compiles QSL source to the checked-package bytes the spine emits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: verifies
---
# TC-905: The replay facade compiles QSL source to the checked-package bytes the spine emits

## Description

Verify `qsl_replay::compile_package` emits the same package bytes and
`package_id` as the spine, and refuses as the spine does.

Scope: FR-060-AC-5 and FR-060-AC-6.

## Test Procedure

From an integration test that reaches only the `qsl_replay` root:

1. Compile `record List { next?: List; }` and `record Tree { kids:
   Sequence<Tree>[0, 3]; }` through `compile_package`. Run the spine's
   `parse`, `select`, `check` and `package` over the same source.
2. Compile a malformed source, and the spine over it.
3. Compile a unit against a library with the unit's own source owner.
4. Compile with S1 `text_input_bytes` of 4, and with a value above the reader
   limit.

## Expected Results

1. For each source the bytes and the `package_id` equal the spine's.
2. The refusal is `ReplayRefusal::Recompile` with the spine refusal's code
   and stage.
3. The refusal is `ReplayRefusal::DependencyInput`.
4. The refusals are `stage_limit_exceeded` and `LimitAboveReader`.
