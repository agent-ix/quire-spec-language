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
