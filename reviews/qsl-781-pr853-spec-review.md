---
id: SR-11086
title: "base spec review of QSL-781 ADR-033 and FR-362 single source text for the checked graph and the type report"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@260ab806f54692defa8ef7c0a19e4687dca8f43a; spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md, spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md, spec/test-cases/TC-1027-no-label-only-match-and-no-composed-parser-remain.md, spec/spec.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-781. PR quire-spec-language#853 adds ADR-033 (Option A adopted),
FR-362 (9 items, AC-1..AC-7), TC-1026, TC-1027 and their index rows. Claims
were re-measured on origin/main. These hold: the profile-line grammar
difference (complete-V1 `profile A = "id";` per FR-110 and the shared grammar;
composed `... version ".." digest "..";`, `src/parser.rs:277`,
`tests/support/composed_types/mod.rs:230`), `build_unit` refusing a recovered
CST (`FormsCause::RecoveringCst` -> `invalid_syntax`), the type path's
non-test caller (`src/protocol_artifact/handoff/writer.rs`), 29 files that spell
a composed profile line, the `UnitLabel`/`UnitLabelMismatch`/`CheckedUnit::new`
names, and ADR-011 SEAM-2/B1. The ids FR-362, ADR-033, TC-1026 and TC-1027 are
free on main and in every open PR. The existing "QSpec FR-362" citations all
name the other repo. No Rust file parses the new files. No ticket ids, SHAs or
private details appear in the spec text, and no dual grammar or bridge is
proposed.

Four claims are wrong or contradict merged text, and AC-7 has no vector that
can be produced.

## Verdict

FAIL. Fix FND-001 to FND-005. FND-006 to FND-008 are recorded only.

## Examined Scope

```yaml
scope:
  - {id: ADR-033, path: spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md, role: examined, excerpt: "Adopt Option A: the complete-V1 grammar (qsl-cst, mapped by qsl-forms) is the one source grammar, and the type phase reads that grammar's output."}
  - {id: FR-362, path: spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, role: examined, excerpt: "QSL SHALL read one source text once, with one grammar, and from that single parse produce both the CheckedGraph and the TypeReport."}
  - {id: FR-362-AC-1, path: spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, role: examined, excerpt: "For a source the grammar admits, the producer returns a graph and a type report from one parse."}
  - {id: FR-362-AC-2, path: spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, role: examined, excerpt: "A source whose profile line carries version or digest is refused by the one grammar with invalid_syntax at the line, by the producer and by qsl_cst::parse"}
  - {id: FR-362-AC-3, path: spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, role: examined, excerpt: "the checked-unit constructor refuses with a cause naming the type report side, the selected reference and the differing one"}
  - {id: FR-362-AC-4, path: spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, role: examined, excerpt: "yields the inventory rows of the producer's own unit, and a type report that covers two units is refused as not one unit"}
  - {id: FR-362-AC-5, path: spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, role: examined, excerpt: "the label-only types and the label-only constructor do not exist"}
  - {id: FR-362-AC-6, path: spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, role: examined, excerpt: "The root crate has no composed parser, no composed syntax tree and no ParsedUnit-to-composed-tree conversion"}
  - {id: FR-362-AC-7, path: spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, role: examined, excerpt: "A unit that holds a construct of a family whose type phase has not moved to S2 forms is refused with unsupported_construct at that construct"}
  - {id: TC-1026, path: spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md, role: examined, excerpt: "Verify FR-362-AC-1 to FR-362-AC-4 and FR-362-AC-7."}
  - {id: TC-1027, path: spec/test-cases/TC-1027-no-label-only-match-and-no-composed-parser-remain.md, role: examined, excerpt: "Verify FR-362-AC-5 and FR-362-AC-6 by absence"}
  - {id: ADR-011, path: spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, role: context_only, excerpt: "M-6e composed checker | SEAM-2: each composed family is deleted in the PR that lands its S3 family checker and S4 emission. The composed checker is deleted, not kept (owner ruling)."}
  - {id: FR-110, path: spec/functional/FR-110-resolve-header-profile-selections-at-e3.md, role: context_only, excerpt: "E3 compares the header's identity, the only member a header profile carries"}
  - {id: FR-035, path: spec/functional/FR-035-parse-composed-native-units.md, role: context_only, excerpt: "When a source selects an admitted composed edition, the compiler SHALL construct located syntax for its predicate, state, temporal and choreography declarations using that edition's shared grammar."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ADR-033 Option A keeps the composed checker. It says `linking::composed` and `checking::composed` "read `qsl_forms::ParsedUnit` instead of `ComposedUnit`" and are "re-read over S2 forms; about 15,000 lines". ADR-011 M-6e (line 1165) says "The composed checker is deleted, not kept (owner ruling)", and the B row (line 1195) sends B2 to B4 to the S3 binding phase and B6/B7 to the S3 family checkers. Reason 1 ("ADR-011 already rules it ... Option A is that ruling applied") therefore misstates ADR-011, which rules Option D's shape. ADR-033's own Decision (line 178) says "the composed lane is deleted", which conflicts with Option A as written. Fix: either put the type phase in S3 (D) or record a ruling that amends ADR-011 M-6e, and make the Decision match. | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:35-41,47,101-113,178 |
| FND-002 | high | FR-362 deletes the composed parser and refuses the composed profile line, but FR-035 (parse composed native units), FR-036 (link), FR-040 (check composed values) and FR-049 (composed evaluation inputs) stay normative and unamended. FR-035 still says "When a source selects an admitted composed edition, the compiler SHALL construct located syntax ... using that edition's shared grammar". After merge, two normative texts contradict each other. Fix: retire or amend those FRs and their TCs in this PR, or list them in ADR-033 Consequences with the PR that retires them. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:29-36; spec/functional/FR-035-parse-composed-native-units.md:19 |
| FND-003 | high | ADR-033 lists `src/parser.rs` among the deleted files. On main that file holds the historical `0-draft` parser (`parse`, `parse_source`, re-exported from `src/lib.rs:34`), which SEAM-1 native compile still calls (`src/command/compilation.rs:54`, `src/mapped.rs:185`). ADR-011's SEAM-1 row keeps native `compile` for `0-draft` sources until M-6c. Fix: delete only `parse_native*`, `src/parser/composed*` and `src/syntax/composed*`. | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:61,109 |
| FND-004 | high | The Callers measurement leaves out the main production graph path. `qsl-replay/src/spine/lifecycle.rs` (spine compile, FR-278) runs `qsl_cst::parse_with_cancel` -> `build_unit` -> `PackageDeclarations::assemble` -> `check` over one `Source`, and it already exports a public `spine::CheckedUnit` (source + checked package). `qsl-bench` also calls `build_unit`. FR-362 item 3 then specifies a new public producer and a second `CheckedUnit` concept next to the spine's, and it never says how the producer relates to spine `parse`/`check`. Fix: correct the measurement and say whether the producer is the spine front end extended by the type phase or a separate function, and settle the `CheckedUnit` naming. | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:67-72; spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:40-43 |
| FND-005 | high | FR-362-AC-7 and TC-1026 step 5 name no moved family and no unmoved family, and nothing says which families this slice moves. Under Option A the whole composed type phase is re-read over S2 forms (ADR-033:101-113), which leaves no unmoved family, so the step 5 vector cannot be built and AC-7 has no test that can fail. Once the last family moves, the same is true under any reading. Fix: name the families this slice moves and the unmoved family the vector uses, or drop AC-7 if the slice moves every family. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:76; spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md:21,29 |
| FND-006 | medium | AC-2 and TC-1026 step 2 expect `qsl_cst::parse` to "refuse `invalid_syntax`". On main it returns `Ok(ParsedSource)` with a recovery and an `invalid_syntax`/`unexpected_token` diagnostic at `version` (`qsl-cst/src/lib.rs` doc: "Invalid syntax returns a recovered ParsedSource"). Only `build_unit` refuses. Recorded. | spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md:18,26 |
| FND-007 | low | "about 15,000 lines" for `src/linking/composed/**` + `src/checking/composed/**`: those trees total 12,986 lines (including `composed.rs`). It reaches about 15,100 only with the composed parser and syntax added. Recorded. | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:111 |
| FND-008 | low | TC-1027 steps 1 and 2 "search" without naming the symbols (`UnitLabel`, `UnitLabelMismatch`, `parse_native`, `parse_native_source`, `ComposedUnit`, `syntax::composed`) or the paths, so two runs could grep different things. Recorded. | spec/test-cases/TC-1027-no-label-only-match-and-no-composed-parser-remain.md:16-17 |

## Evidence and Limits

All measurements were taken by `git grep`/`git show` on origin/main. No build
ran. No Rust file references FR-362, ADR-033, TC-1026 or TC-1027 as include_str!
input. The only include_str! of a spec file is FR-092, which this PR does not
touch. The open PRs (852, 845, 723, 674, 672 and the rest) add no FR-36x,
ADR-033 or TC-102x file.
