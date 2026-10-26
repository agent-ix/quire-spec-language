---
id: TC-1268
title: "Repeat decisions read attempt binders from the latest completed body iteration"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: verifies
---
# TC-1268: Repeat decisions read attempt binders from the latest completed body iteration

## Description

Verify that S3 types a repeat's `visible` entries and guard using eligible
attempt binders declared in that repeat body, that the guard uses the most
recent completed iteration, and that ordinary repeats retain their pre-test
behavior.

Scope: FR-218-AC-33.

## Test Procedure

1. Check a protocol whose declared roles include `w` on `M::Cell` and `c` on
   `M::Core`, with a parallel branch for `w` and a branch for `c` containing:

   ```text
   repeat Spin by c visible (loaded) while { not loaded }
   sequence Poll {
     attempt Load by c on M::Core::look ordering relaxed
       contracts [LooksX, Looked] as (loaded: Boolean) { true };
   }
   ```

   The `look` operation returns whether its relaxed load of `x` read 1, and
   its frame writes `c.r`.
2. Run the protocol with `x` initially 0 and the `w` branch storing 1. Inspect
   the trace to confirm the first `Load` attempt occurs before the first guard
   decision and that a later guard reads the binder value from the most
   recently completed iteration.
3. Run a post-test repeat whose guard is `false and not loaded`; verify its
   body completes once despite the false guard. Run a repeat whose guard is
   `false` and which reads no body binder; verify its body is skipped.
4. Compile a variant of the post-test repeat with `max 0`.
5. Inspect S3's checked repeat slots and binder reader set. Compile variants
   where `loaded` is produced by a different role, where its producer is in a
   nested branch that does not join before the repeat decision, and where the
   referenced binder is produced by a sibling outside the repeat body.

## Expected Results

- The original protocol checks. The repeat's `visible` entry and guard each
  read the `Load` attempt binder, and the read is recorded for the `Spin`
  decision with owner `c`.
- The first body pass of a repeat whose guard reads `loaded` is unconditional;
  each later guard uses the value from the most recently completed body
  iteration. The post-test repeat cannot complete zero iterations, and its
  `max 0` variant refuses `invalid_package`/`invalid-value` at the repeat
  declaration. A repeat whose guard reads no body binder keeps the pre-test
  rule and skips its body when the entry guard is false. Values produced
  through nested body controls follow their normal sequence and join scope,
  and remain eligible only to their producing role.
- The different-role variant refuses `wrong_snapshot`/`wrong-observation` at
  the read with reason `other-role`; the unjoined or external sibling variant
  refuses `missing_declaration`/`missing-name` at each out-of-scope use. No
  refused variant produces a checked package.
