---
id: SR-1338
title: "Spec review of quire-spec-language PR #641 (B4): FR-062, FR-082, FR-083, FR-096, FR-099, FR-111, NFR-012 against the B4 rulings"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@fc90c46a13308f63d21d28549dd9b30e9d745087; spec/functional/FR-062, FR-082, FR-083, FR-096, FR-099, FR-111, FR-255, FR-277; spec/non-functional/NFR-012; spec/decisions/ADR-030; spec/spec.md; spec/tests.md; spec/test-cases/TC-220, TC-225, TC-446"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: reviews
---
# Spec review of quire-spec-language PR #641 (B4)

## Summary

Ticket: QSL-485 (B4). The PR changes no spec file. This review checks that the
spec B4 implements states the decided rulings, and that the spec's own status
text still matches the code at fc90c46a1. Rulings judged as decided: every
depth limit kind is deleted; ancestor and family steps are edge counts with a
16777216 default; a limit refusal is a `LimitExceeded` naming its setting,
incomplete, exit 22; no compat layer; FR-099's `DependencyLimits` belongs to B4.

The normative text agrees with the rulings and with the code:

- FR-082 Behavior: `ancestor_steps` counts the edges of one walk's closure,
  never a depth, defaults to 16777216, and stops `Incomplete`. The type
  environment reports an edge count with actual = limit + 1.
- FR-083 Behavior and AC-4: `family_steps` is one edge count over all of a
  link's walks, defaulting to 16777216.
- FR-096-AC-2: seven kinds, no `nesting-depth-exceeded`.
- FR-099 Behavior and AC-7: `DependencyLimits {libraries 4096, import_edges
  16384, source_bytes 16777216}`, the `dependency.*` settings,
  `stage_limit_exceeded` at stage `intake` at the charging import, and a
  chain of any length. FR-255's table matches.
- FR-111 Closure: an explicit stack charged against `dependency_edges`, and
  a chain of any length.
- NFR-012: both step limits are 16777216 edges.

## Verdict

Changes requested, on status text only. The requirements and ACs are correct
and consistent with the rulings. Several status rows still describe the
depth limits B4 deletes, or call implemented work "planned" (FND-001 to
FND-005). One older inconsistency in setting names, which this PR does not
introduce, sits next to the new `dependency.*` names (FND-006).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-277's index row in spec.md and TC-758's row in tests.md say "`dependencies.depth` stays the import refusal of ADR-015 D-1 until B4 ... owns it" and "`dependencies.depth` belongs to B4". After this PR there is no `dependencies.depth`; the S4 resolution reports `dependency.libraries`, `dependency.import_edges` and `dependency.source_bytes` as named `LimitExceeded`s. A reader would look for a limit that does not exist. Fix: update both rows to the `dependency.*` fields, reached through `check`. | spec/spec.md:1286; spec/tests.md:989 |
| FND-002 | low | FR-099 Status says "TC-446 passes locally for AC-1 to AC-6" and says nothing of AC-7 or `DependencyLimits`. tests.md's TC-446 row says "step 8 pending". This PR backs AC-7 (TC-446's limits step). Fix: record AC-7 as backed in FR-099 Status and the TC-446 row. | spec/functional/FR-099-compile-against-supplied-libraries.md (Status); spec/tests.md:217 |
| FND-003 | low | FR-111 Status still lists "Remaining work: the `depth` field of `PackageLimits` is deleted". This PR deletes it. Fix: remove the item. | spec/functional/FR-111-link-a-complete-v1-definition-bundle.md:168 |
| FND-004 | low | FR-096 Status says "The seven-kind `LimitKind` and the setting field (FR-255) are not yet implemented." After this PR `LimitKind` has exactly the seven kinds of FR-096-AC-2, and `LimitExceeded` carries `LimitsField`. Fix: mark the seven-kind `LimitKind` as implemented. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:424-425 |
| FND-005 | low | tests.md marks TC-220 and TC-225 "Planned", and its notes say the depth-ceiling tests "are replaced when `ancestor_steps` becomes a charged edge-count limit" and when "`family_steps` becomes an edge-count work limit". This PR does both, and the TC-220 and TC-225 tests are rewritten. Fix: update both rows and both notes. | spec/tests.md:98,103,690,695 |
| FND-006 | low | The settings are named two ways. FR-082 and FR-255's table name the type-environment settings `types.ancestor_steps` and `types.work_units`, and the model's `normalization.*`. FR-277's spec.md row and the code (`LimitsField::as_str`) say `environment.*` and `model.*`, so `LimitExceeded` names a setting FR-255 does not list. This PR did not cause it; it adds `dependency.*`, which matches FR-255. Fix: pick one spelling in FR-255 and FR-277 and make `LimitsField` follow it, in the FR-255 owner's change. | spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md:146-147; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:61-62; qsl-foundation/src/diagnostic/stage.rs:130-138 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | FR-099 Behavior says the compile "SHALL wrap every other refusal raised while resolving or compiling a library as `CompileRefusal::Dependency { path, refusal }`". After this round a reached dependency limit is a located `CompileRefusal::Limit`: the lifecycle turns every `StageFailure::Limit` into one (FR-277), and the locus is the library's import, with no library path. The tests assert the located form, which satisfies AC-7's "at test/b's import of test/c", but the Behavior text says otherwise. Fix: add reached limits to the unwrapped list in FR-099 Behavior ("a reached limit is reported as a `LimitExceeded` located at the import that charged it"). | spec/functional/FR-099-compile-against-supplied-libraries.md (Behavior, wrapping bullet); qsl-replay/src/spine/dependency_tests.rs:509-521 |

## Dispositions

Round 1, reviewed at 43053adeb4dcd40c4d7956f17a4135b75a73cfee (spec edits in
21f63b513).

- **FND-001:** the FR-277 row lists `dependency.*` and drops the
  `dependencies.depth` clause. The TC-758 row names the three dependency
  fields.
- **FND-002:** FR-099 Status says AC-1 to AC-7 pass and describes
  `DependencyLimits`. TC-446 reads "Steps 1 to 8 pass locally".
- **FND-003:** the "Remaining work: the `depth` field" item is gone.
- **FND-004:** FR-096 Status records the seven-kind `LimitKind` and the
  `LimitsField` setting name.
- **FND-005:** TC-220 and TC-225 are passed, and their notes list the backing
  tests.
- **FND-006:** FR-255 and FR-082 now spell `environment.*` and `model.*`,
  matching FR-277 and `LimitsField`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 21f63b513 |
| FND-002 | fixed | 21f63b513 |
| FND-003 | fixed | 21f63b513 |
| FND-004 | fixed | 21f63b513 |
| FND-005 | fixed | 21f63b513 |
| FND-006 | fixed | 21f63b513 |
