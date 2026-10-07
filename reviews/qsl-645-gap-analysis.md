---
id: SR-1364
title: "Gap analysis of quire-spec-language PR #653: root native parser caller limits (QSL-645)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5aaaf81cd8778e1ffb83e9cf286888e736ffb538; PR #653 diff against origin/main plus the live callers named in QSL-645: src/main.rs, src/cli.rs, src/lib.rs, src/package/reading.rs, src/linking/composed.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: reviews
---
# Gap analysis of quire-spec-language PR #653

## Summary

Ticket: QSL-645. Plan completion: not assessed.

Intent (QSL-645, read as data): the root native parser, live through the CLI `parse` subcommand, the lib.rs re-exports, `package::reading::read_verified` and `linking::composed::admit_namespace`, takes caller limits as given and parses 100,000-deep sources on a 512 KiB stack through `parse` and `parse_native_source`.

- Re-exports (`parse`, `parse_source`, `parse_native`, `parse_native_source`): limits now used as given. Covered by TC-749 for `parse` and `parse_native_source`.
- `read_verified` (src/package/reading.rs:410): passes `limits.syntax` straight to `parse`, so the clamp removal reaches it. Package, link and check limits on that path still clamp (`PackageLimits::bounded`, `LinkLimits::bounded`, `CheckLimits::bounded`); those are later stages under FR-257/FR-258, not this ticket.
- `admit_namespace`: now stores `parser_limits` as given (src/linking/composed.rs:397) and `inventory.rs:127` passes it to `parse_native_source`.
- CLI `parse`: see FND-001.
- Bindings: all 10 tests in tests/it/deep_sources.rs tagged `#[trace("TC-749", "FR-256-AC-4")]` cover the five shapes in both editions as AC-4 states. Correct, but the criterion does not resolve in the matrix (SR-1365 FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The CLI `quire-spec parse` subcommand runs `parse` under a fixed `Limits::default()` and takes no limit option, so the user cannot raise any limit. A 100,000-deep bracket source is 200,001 tokens, above the 100,000 default, and the CLI refuses it. QSL-645 lists this subcommand as a live path, and FR-256's Description now names `quire-spec parse`, but no AC or test covers it. Either give `parse` a limit option (FR-255 setting names) with a test, or take `quire-spec parse` out of FR-256's statement. Same fixed-default pattern: src/command/compilation.rs:117 and src/protocol_artifact/handoff/writer.rs:1344. | src/main.rs:165-167, src/cli.rs:59-80 |
| FND-002 | low | No test shows raised parser limits reaching `admit_namespace` (or `read_verified`). If a clamp came back at src/linking/composed.rs:397, nothing would fail, because TC-749 calls only `parse` and `parse_native_source`. Add one test that admits a namespace whose source exceeds the default token limit, under raised `parser_limits`. | src/linking/composed.rs:397 |

## Verdict

AC-4 is implemented and tested for the two named entry points. Two gaps: the CLI path cannot use raised limits (medium), and the admit path's pass-through is untested (low). Changes requested.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The ruling that L2 needs 100,000 depth live through `read_verified` is not met, and parser work alone cannot meet it. (1) The new test `read_verified_uses_a_raised_byte_ceiling_as_given` calls `qsl_foundation::Source::read_verified`, which took its byte limit as given before this PR too. It never reaches `package::reading::read`, the path QSL-645 names. Its only check that would fail on main is the trailing `parse`, which repeats `historical_100000_term_sum_parses`. It is also tagged FR-256-AC-4, which does not mention `read_verified`. (2) The coder's premise is wrong. `NativePackage::read_verified` does take parser limits: `PackageReadLimits.syntax` ("Actual native parser limits") goes to `parse` at src/package/reading.rs:410. (3) Right after the parse, the same path runs `link_native`, whose `LinkLimits::bounded()` caps nodes at 10,000 and depth at 64 (src/linking.rs:31-61, preflight at :373), and then `check`, whose `CheckLimits` caps depth at 64. So no source 100,000 deep, or even more than 64 deep, can get through `read_verified`, whatever parser limits the caller gives. The `run` command also hard-codes `PackageReadLimits::default()` (src/command/compilation.rs:146). Either remove the link/check caps on this path in this PR (FR-257/FR-258 territory) and test `NativePackage::read_verified` end to end, or narrow the L2 claim to the parser stage. That is a lead/owner decision. | src/package/reading.rs:410, src/linking.rs:350-373, tests/it/deep_sources.rs:254-287 |

## Dispositions

Round 1, reviewed at 9163ca383852b50150be0ab472efcbb5c6008002.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3430ab929 (wording 1b9555987): `quire-spec parse` takes `--source-bytes`, `--tokens` and `--nodes` (FR-256-AC-5, TC-750). The CLI test parses 100,000-deep brackets with raised flags, exits 22 with `stage_limit_exceeded` without them, and exits 20 on a missing value, a non-numeric value or an unknown option. Leaving compilation.rs:117 and handoff/writer.rs:1344 alone holds, but not for the stated reason: both do parse (`parse_source` and `admit_namespace` under `Limits::default()`). compilation.rs:117 is followed by `link_native` under the capped `LinkLimits` (see FND-003), so raising its parser limits would change nothing. The writer is an internal caller with fixed work limits, outside the three paths in the ruling |
| FND-002 | fixed | 3430ab929: `admit_namespace_uses_raised_parser_limits_as_given` refuses under default limits, then admits under raised ones and checks `parser_limits()` equals them, so a clamp at src/linking/composed.rs:397 would fail it |
| FND-003 | deferred | Out of scope per plan v2 §10(d)1: M6C-S deletes the native read_verified/link/check/package path. 9163ca383 limits FR-256 to the parser plus `admit_namespace`, says that path keeps its bounded limits, and drops the `Source::read_verified` test. No AC, TC or spec.md row claims read_verified carries the depth any more |
