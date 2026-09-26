---
id: TC-470
title: "runtime_invariant exits 30 and outranks other diagnostics"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-470: runtime_invariant exits 30 and outranks other diagnostics

## Description

Verify FR-096-AC-12: the FR-301 exit status of `runtime_invariant`.

Scope: FR-096-AC-12.

## Test Procedure

1. For every `Code::all()`, assert `exit_code()` is 30 exactly for
   `RuntimeInvariant` and otherwise in 20..=22.
2. Corrupt a validated evaluation context so `evaluate` refuses with
   `runtime_invariant`; assert the diagnostic's `exit_code()` is 30.

3. Combine the code pairs {30,20}, {30,21}, {30,22}, {20,21}, {21,22}, {20,22} through
   the report's combining function (both orders); assert 30, 30, 30, 20, 21, 20.

Tag the tests `#[trace("TC-470", "FR-096-AC-12")]`.
