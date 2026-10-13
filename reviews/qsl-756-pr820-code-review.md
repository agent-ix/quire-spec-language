---
id: SR-9057
title: "Code review of quire-spec-language PR #820: specialization-cycle refusals per declaration and cycle"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@65cc4b55e0c01e7ea2a0031c4a22d60284d61d23; PR #820: qsl-semantics/src/model/normalize.rs, qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/refusal.rs, qsl-semantics/tests/it/model_normalization.rs, qsl-semantics/tests/it/tc235_record_event.rs, qsl-semantics/tests/it/business_meanings.rs"
review_set: subset
---
# Code review of quire-spec-language PR #820

## Summary

Ticket: QSL-756. Code review with the rust-review lane folded in.

The object-type and process path is correct. `build` still deduplicates cycle
candidates by their rotated directed-edge list, sorted by `(type_key, path)`,
so detection order and charge accounting are unchanged. Only the refusal
construction moved, into `specialization_cycle_refusals`, which emits one
refusal per declaration on the cycle in declaration-key order. Each refusal
carries the same rotated cycle. The `work_units=46` vector, `h2`, the
two-distinct-cycles vector and the phase-3-wins vector all keep their charge
expectations and now assert the per-declaration shape.

Consumer audit, re-run by grep over every crate, `xtask`, `tools`, `src/`,
`tests/`, `fuzz/` and `examples/`: `SpecializationCycle` is used only in
qsl-semantics. The uses are the definition, three `{ .. }` mappings in
`refusal.rs` (catalog fields, tag, code), the exhaustive fixture, two test
tables, the two constructions and the tests. `ModelRefusalCause` derives only
`Clone, Debug, Eq, PartialEq`; it has no serde and no Display that consumers
parse. The catalog-field mapping serializes no fields for this cause, before
and after. No code parses either the old or the new detail text. The only
matches for the old detail fragments are in this PR's own removed lines and an
unrelated ADR sentence.

The `{ declaration, cycle }` shape is sound. Every refusal names the
declaration it is raised at, and `cycle` is identical across a cycle's
refusals. The added code has no `unwrap`, `expect` or indexing that can panic.
`min_by_key` returns an `Option` that is handled, and the existing
`chain[(index + 1) % chain.len()]` runs only on a non-empty closing chain.

The record-value path in `intake.rs` does not follow the rule this PR states
(FND-001).

## Verdict

Changes requested: one high finding (record path), two medium public-repo
findings and two low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The record-value path refuses each declaration at most once, not once per cycle. `business_generalization_refusal` runs once per upstream supertype-cycle diagnostic, and upstream emits one diagnostic per type on any cycle, so each record declaration gets one refusal. That refusal lists the first cycle a LIFO DFS reaches, which comes from the last `supertypes` entry first. The refusals then follow diagnostic (type-position) order, not cycle detection order and then declaration-key order. Records `A` (`supertypes: [B, C]`), `B` (`[A]`) and `C` (`[A]`) therefore give three refusals, with `A` listing `[A, C]`. The rule this PR adds to FR-082 requires four: `A` and `B` listing `[A, B]`, then `A` and `C` listing `[A, C]`. Record value types accept several same-meaning supertypes, so this shape is reachable. Fix: enumerate the distinct cycles of the record supertype graph with the same procedure normalization uses (closing extensions in walk order, deduplicated by edge set) and emit them through `specialization_cycle_refusals`. Alternatively, filter record cycles out at intake and let normalization report them, as the process path already does. Add a record multi-cycle vector. | qsl-semantics/src/model/intake.rs:2071-2130; qsl-semantics/src/model/intake.rs:1405-1445 |
| FND-002 | medium | Public-repo rule. The doc comments this PR rewrites in `normalize.rs` still cite private QSpec definition files by path and line and quote their sentences verbatim. The PR removed one such source's quotations from these hunks but kept the others. Cite QSpec by id only, or state the rule in QSL terms with a QSL FR id (FR-082 now states it). | qsl-semantics/src/model/normalize.rs:56-72; qsl-semantics/src/model/normalize.rs:874-883; qsl-semantics/src/model/normalize.rs:2864-2867 |
| FND-003 | medium | Same public-repo rule, wider than the touched hunks. About 135 lines in `qsl-semantics/src` cite private QSpec definition files by path and line, many with verbatim quotations, and `normalize.rs` and `intake.rs` carry most of them. This predates the PR. It is listed because the PR edits both files, and a found leak is not an out-of-scope note. It needs its own sweep, which the lead should raise with the owner, because the text is already in public history. | qsl-semantics/src/model/normalize.rs; qsl-semantics/src/model/intake.rs |
| FND-004 | low | Once rebased on the current base, which now includes #817 and merges cleanly, the M17 doc comment in `tc235_record_event.rs` describes a "deviation" from the TC row that no longer exists. Its mutation note also names `cycle_refusals.len() == 2`, which this PR replaces with `assert_specialization_cycle_refusals`. Rewrite the comment on rebase, and do not describe the upstream row's content in it. | qsl-semantics/tests/it/tc235_record_event.rs:1123-1140 (base after #817) |
| FND-005 | low | `cycle: cycle.clone()` in the record path clones only because `listing` still borrows `cycle`. Build the `detail` string first and then move `cycle` into the cause. | qsl-semantics/src/model/intake.rs:2110-2121 |
