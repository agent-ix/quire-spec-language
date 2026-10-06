---
id: FR-263
title: "Replay deep sources and values under every limit the request carries"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: depends_on
---
# FR-263: Replay deep sources and values under every limit the request carries

## Description

The layer-6 replay facade (FR-098) SHALL replay a request whose source holds
expressions of any depth and whose assignment carries values of any depth,
under the proving run's stage limits and accounting limits carried in the
request (ADR-030 D-4.8, D-3).

## Behavior

1. **Every stage limit passes through.** The request's `stage_limits` SHALL
   be a map from setting name (FR-255) to a non-negative bound. Replay SHALL
   run each stage of its recompile with every entry naming that stage's
   settings at the entry's bound, and every setting with no entry at its
   published default.
2. **Request decode.** When a `stage_limits` entry names a setting that is
   not in FR-255's table, or names `replay.input_bytes`, request decode SHALL
   refuse it, naming the entry (FR-071).
3. **Values.** Replay SHALL decode each canonical assignment value into a
   `quire_exact::Value` from `quire-canonical`'s reader tree (FR-259), over
   an explicit heap stack.
4. **Value identities.** Replay SHALL encode every identity and digest over
   a value through the event API (FR-259).
5. **Limit outcomes.** When a recompile stage reaches a limit, replay SHALL
   return the refusal FR-098 defines for a recompile stage limit, carrying
   the limit, its bound, the count reached and its setting. When the request
   or envelope bytes exceed `replay.input_bytes`, replay SHALL refuse with
   `BoundExceeded` naming that setting. When a call's charge would exceed an
   accounting limit, replay SHALL settle as FR-098 defines for an evaluation
   that completes no value.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-263-AC-1 | On a thread with a 512 KiB stack, a request whose source holds a function with a 100,000-term sum over a recursive list parameter, whose assignment carries a 100,000-long recursive list value, whose `stage_limits` carries the proving run's raised `s1.*` and `s3.*` settings, and whose `replay.input_bytes` is raised to fit the request, replays to the same verdict as the proving run, and its recompiled `package_id` equals the request's. | Test (TC-736) |
| FR-263-AC-2 | For each `s1.*` and `s3.*` setting of FR-255's table, a request whose source reaches that limit at the bound its `stage_limits` entry gives refuses with the recompile stage-limit refusal naming the limit, the bound, the count reached and that setting; the same request with that entry raised to fit recompiles. A request with no `stage_limits` entries recompiles at the published defaults. | Test (TC-737) |
| FR-263-AC-3 | A request whose `stage_limits` holds an entry `s9.nodes`, or an entry `replay.input_bytes`, refuses at decode naming the entry. A request one byte longer than `replay.input_bytes` at bound `B` refuses with `BoundExceeded`, bound `B`, actual `B + 1` and setting `replay.input_bytes`, and decodes once the library replay entry's limit or FR-255's settings operation given `replay.input_bytes=<B + 1>` (FR-255), which the driver CLI exposes as `--limit` (ADR-029 CB-1) raises it. | Test (TC-737) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-3 and D-4.8.
- [FR-071](FR-071-implement-typed-replay-request.md) defines the typed
  replay request and its decode refusals.
- [FR-098](FR-098-execute-a-replay-request.md) defines replay execution and
  its outcomes.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) defines
  the setting names.
- [FR-258](FR-258-check-and-lower-expressions-at-any-depth.md),
  [FR-259](FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md)
  and [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md) define the
  recompile, identity and value behaviour replay inherits.

## Status

Specified. FR-263-AC-1 (TC-736) moved from B5 (QSL-486) to QSL-640: a
recursive list assignment needs the composite `WitnessValue` of
[FR-070](FR-070-implement-typed-counterexample-witness-envelope.md) and
[FR-098](FR-098-execute-a-replay-request.md), and the request's accounting
limits must fit its occurrence and node counts (FR-098-AC-9). FR-263-AC-1
(a 100,000-term source and a 100,000-long list replayed under raised `s1.*`,
`s3.*` and `replay.input_bytes` on a 512 KiB stack) passes locally, and
Behaviors 1 and 2 are implemented for the S1, S3, type-environment and
model-normalization settings, as B5 (QSL-486) implemented them
(`StageLimits` is the setting-keyed map; an entry naming no setting, or
`replay.input_bytes`, refuses at decode).

## References

- QSpec FR-460, the ecosystem depth rule, and FR-323 `stage_limits`, the
  replay request's limits (Linear STD-143, which supersedes STD-125).
- Linear QSL-381.
