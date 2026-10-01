---
id: SR-943
title: "QSL-356 code review (with rust-review lane) of PR 554, quire-exact leaf exemption in arch-lint"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f085d7a307f1ffd3504ebb6b3a52d7978b451051; tools/arch-lint/graph.rs (classify, KERNEL_LEAF); tools/arch-lint/metadata.rs (tc_arch_lint_metadata_007); tools/arch-lint/duplicate_revisions.rs (context: second caller of classify); spec/functional/FR-061-check-duplicate-ecosystem-revisions.md (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: reviews
---
## Summary

Ticket: QSL-356. PR: quire-spec-language#554 at f085d7a3, diff `d81193f9...f085d7a3`.

What I checked:

- **The exemption is exact.** `classify` returns `None` only when `name == "quire-exact"`,
  a whole-string compare. I measured it: `quire-exact-foo`, `qsl-replay` and `Quire-Exact`,
  each sourced from the QSL git url, still classify as `Repo::Qsl`. No other QSL crate is
  exempt.
- **The new test is real.** With `graph.rs` reverted to d81193f9,
  `tc_arch_lint_metadata_007_quire_exact_leaf_is_exempt_but_qsl_eval_is_not` fails: the
  edges are `[RT->QSL via qsl-eval, RT->QSL via quire-exact]` against an expected one edge.
  It also asserts the FB-05 report holds exactly the `qsl-eval` edge and no FB-11 cycle.
- **Rust idioms.** A `const &str`, an early return, no panics, no new public surface. The
  trace tag `#[trace("TC-156", "FR-059-AC-8")]` uses bare QSL ids, per the repo rule.
- **Collateral change to FR-061: found.** `classify` is shared with the FR-061
  duplicate-revision check (`duplicate_revisions.rs:94`). Putting the exemption inside
  `classify` removes `quire-exact` from FR-061 too. See FND-001.

Gate: the coder's `make ci` log on f085d7a3 ends `exit=0`. I did not re-run it. Focused
`cargo test -p arch-lint` runs (84 tests) were done in a throwaway worktree with probe
tests added; the worktree and `target/` are deleted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The exemption lives in the shared `classify`, so the FR-061 duplicate-revision check no longer sees `quire-exact`: a lock holding `quire-exact` at two QSL git revisions now reports no duplicate (reproduced; on d81193f9 it reports one). FR-061 Behavior requires classification by `source`, so a QSL-sourced package classifies as QSL. Move the exemption to the FR-059 edge path (`metadata::parse_edges` / `edges_for_manifest`) and add an FR-061 test that two `quire-exact` sources are still reported. | tools/arch-lint/graph.rs:60-63, tools/arch-lint/duplicate_revisions.rs:94 |

## Verdict

The FR-059 half is correct and well tested. Not mergeable until FND-001 is fixed: the
change silently removes the one crate whose duplicate copy would split the kernel types
(`Origin`, `NodeKey`) from the FR-061 guard, which the brief required to behave unchanged.
