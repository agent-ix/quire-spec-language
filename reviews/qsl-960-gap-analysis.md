---
id: SR-12308
title: "Gap analysis of quire-spec-language PR #1197 against QSL-960 and FR-399 (QSL-960)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ae46f1869a9416e06a7ae44519c7e306a902b4c3; FR-399-AC-1..AC-4; TC-1093; qsl-replay/src/request.rs; qsl-replay/tests/stage_limit_named_root_only.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1197

## Summary

Ticket: QSL-960. The ticket text is treated as data. It asks for a
replay-root way to set a named stage limit, so that CG can intake its
2,027,895-byte and 2,599,335-byte fixture sources. It names
S1InputBytes.

The four ACs and the TC-1093 tests agree:

- AC-1: refused at the default.
- AC-2: AboveReader with only the S1 limit raised. Compiled when the reader
  limit is raised as well.
- AC-3: the unknown name, the empty name, `replay.input_bytes` and
  `format.output_bytes` all refuse, typed, and carry the name.
- AC-4: the default is empty, and a later entry replaces an earlier one.

All four tests pass, and the code does what the ACs say.

The capability reaches every limit that titan's sources hit, but the ticket
and the ACs only cover `s1.input_bytes`. I measured this with throwaway
real-declaration sources of 2.0 MB and 2.6 MB through `compile_admitted`,
using only root APIs. In order, the source trips:

1. `s1.tokens` (limit 100000, reached at byte 266678)
2. `s1.nodes` (limit 50000, reached at byte 299813)

It then compiles. Titan needs four settings:

- `with_named("s1.input_bytes", n)`
- `with_named("s1.tokens", t)`
- `with_named("s1.nodes", k)`
- `ReplayLimits::with_input_bytes(n)`

At this density, 2.6 MB is roughly 1M tokens and 430k nodes. S3, model and
admission limits did not fire for these sources. CG's real fixtures may hit
other limits; every FR-255 stage and accounting name is settable by name.

## Verdict

Mergeable. The capability is complete for the stated goal. FND-001 is a
verification and documentation gap, not missing behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No AC or test covers a large source of real declarations. TC-1093 pads one enum with comment lines, which produce no tokens or nodes, so it never reaches `s1.tokens` or `s1.nodes`. Measured: a real 2.0 MB or 2.6 MB source raised only on `s1.input_bytes` and the reader limit is refused at `s1.tokens` (100000), then at `s1.nodes` (50000). It compiles only when both are also raised. The ticket's goal, CG intaking its 2.0 MB and 2.6 MB fixtures, is reachable, but nothing tested shows it. | qsl-replay/tests/stage_limit_named_root_only.rs:18-26 |
| FND-002 | low | FR-399-AC-4's "default `s1.input_bytes` is unchanged" rests on AC-1 alone: a source of 1 MiB + 4 KiB is refused. Nothing shows a source at or just under 1 MiB still admitted at the default, so a lowered default would also pass. | qsl-replay/tests/stage_limit_named_root_only.rs:98-116 |

## Dispositions

| FND | Outcome | Sha |
| --- | --- | --- |
| FND-001 | fixed | 9ffbfbb16 |
| FND-002 | fixed | 9ffbfbb16 |
