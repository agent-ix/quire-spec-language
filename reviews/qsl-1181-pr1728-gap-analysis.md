---
id: SR-1728002
title: "gap-analysis of PR #1728 (QSL-1181): typed refusal for two refinements of one meaning"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1728 git diff origin/main...59b5bdb35; qsl-package/src/emit.rs, emit/tests.rs, emit/tests/admission_corpus.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1728002: gap-analysis of PR #1728

## Summary

Ticket: QSL-1181. Head reviewed: 59b5bdb35, on main 2d901bee4, 3 files. Interim for the planner ruling that the authored name is part of refinement identity.

The probe results (scratchpad, reverted):

| Unit | Result |
| --- | --- |
| `Zed` then `Alpha`, same meaning | `SameMeaningRefinements{first: "Zed", second: "Alpha"}`. The graph keeps source order, so `first` is the earlier declaration. |
| `R`, `R2`, `R3`, same meaning | Refuses at the first collision, `{R, R2}`. |
| One refinement | Emits. |
| `R` (`abstract A concrete C`) and `R2` (`abstract C concrete A`), distinct meanings | Both emit. |

- **Code and cause:** `unsupported_construct`/`not-yet-implemented`.
- **Locus:** the two names. Other emit refusals name their declaration the same way (`ProbabilisticModelNotEmitted { declaration }`).
- **No masking:** the check keys on equal `node_id` only, and runs after `row_empty` node construction and before the source map is built. The three other `SourceMapInvariant` sites (emit.rs:1176, 1187, 1190, 1195) are unchanged.

Gates at 59b5bdb35 (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification 48373530):
- fmt: exit 0.
- `cargo clippy -p qsl-package --all-targets -D warnings`: exit 0.
- qsl-package lib: 366 pass, 2 fail. The 2 are c14/i2, the pre-existing `positive-model-tuple-union.json` `MalformedWire` at `/semantic_graph/nodes/0/owner` from QSpec #468. They are identical on main and not touched by this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | **The "never for distinct meanings" half is not tested.** The PR's test covers only the same-meaning pair, and no test on main emits two refinements of different meanings. A check keyed too broadly (say, on the model pair) would still pass every test. Add the companion my probe ran: `abstract A concrete C` beside `abstract C concrete A` emits both. A one-line case; fix in this PR or with the spec delta. | qsl-package/src/emit/tests/admission_corpus.rs:1012-1044 |

## Verdict

Mergeable. FND-001 is low and does not block.
