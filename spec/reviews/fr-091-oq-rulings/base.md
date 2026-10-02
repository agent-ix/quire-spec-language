---
id: SR-509
title: "base review of PR #354 (FR-091 OQ rulings)"
type: SpecReview
analysis: base
scope: "git diff origin/main...HEAD at a0e00cf9: ADR-011, ADR-012, ADR-013, FR-062, FR-091, spec.md, TC-259, TC-392 to TC-396, TC-400, TC-401, TC-405, TC-406, TC-412, tests.md"
review_set: subset
evaluated_revision: "a0e00cf90cadc3cb5bd5cccec787aaa557105932"
review_date: "2026-09-22"
---

## Summary

Base checklist over the PR #354 diff only. The rulings are taken as decided and
are not re-argued. ID formats are correct, the new FR-091-AC-22 has TC-412, and
every FR-091 AC maps to at least one TC. There are no remaining references to
`ForeignFamilyConstruct` or to the ruled OQs as open questions. The findings
cover fixture completeness, AC wording that cannot be checked as written, and
roadmap wording in the FR.

## Verdict

**Changes needed.** FND-001 to FND-004 are medium and should be fixed in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-091-AC-8 and TC-396 step 3 use the fixture `f using v(x: Int[0, 9])` with the body `deref(x)` and similar. That is not a complete-V1 `function`: the QSpec grammar requires `':' type-ref 'pure' block`, so S1 does not admit the unit and S2 never runs. Fix: write the fixture in full, e.g. `function f using v(x: Int[0, 9]): Boolean pure { deref(x) }`, in both AC-8 and TC-396. | FR-091:436; TC-396:35-39; QSpec shared-grammar.md:319-321 |
| FND-002 | medium | FR-091-AC-8 says "the check stage then refuses each", but the `allInstances<M::T>` refusal is an assembler (E3) unresolved-type-name error. The assembler section (FR-091:310-384) resolves only aliases, function signatures and record/tuple names. No text says that the assembler resolves the type forms inside a body (`convert<T>`, `allInstances<T>`, `fold<A>`, `reduce<A>`, `count<N>`, `sum<N>`), so no behaviour text backs the `M::T` refusal. Fix: add a bullet to the assembler section saying it resolves every type form in a function body and measure with the same resolution match. Reword AC-8 as "the assembler refuses `allInstances<M::T>(x)` ...; `PackageDeclarations::check` refuses `pre(x)` ... and `deref(x)` ...". | FR-091:266-267, 318-322, 436; TC-396:38-39 |
| FND-003 | medium | The FR contains roadmap wording, which the rules forbid: "Each of these constructs gains its variant together with its checker and evaluator arms ... A float literal needs a `ValueType::Float` that carries a rounding mode" (FR-091:276-281). Also: the OQ-4 ruling "`ValueType::Float` carries the rounding mode ... While `ValueType::Float` has no mode, FR-091-AC-19's refusal holds"; the OQ-5 ruling "Each gains its variant ..."; and OQ-9's "does not yet represent". Fix: state the current rule only, e.g. "These constructs refuse with `UnrepresentedConstruct`. `reaches(..)` is a `StateModel` construct." Move the target shape (mode-carrying `ValueType::Float`, per-construct slices) to the QSL-131 and QSL-141 tickets. In the OQ-4 row, state the rule: the rounding mode is part of the float type, and a representation that cannot hold it refuses (AC-19). | FR-091:276-281, 515, 516, 536-537 |
| FND-004 | medium | FR-091-AC-22 and TC-412 step 1 check that the assembler "records `f`'s `using` alias as resolved to that selection", but no text names where that record lives. `PackageDeclarations` has no field for it, and the FR says only "records the resolved selection with the declaration". The test has no observable carrier to read. Fix: name the field that holds the resolved selection (e.g. on the `functions` entry) in the assembler section and in AC-22, and have TC-412 read that field. | FR-091:326-330, 450; TC-412:29-31 |
| FND-005 | low | The layering paragraph now bans `ValueType`/`NodeKey` fields on a "parsed-unit selection" (FR-091:299-302), but FR-091-AC-11 still lists only the parsed form, the type form and the mapping-table `Expression` variants. Nothing tests the new rule. Fix: add "of a parsed-unit selection" to AC-11's field clause and to TC-398's scan. | FR-091:299-302, 439 |
| FND-006 | low | The FR says an unchanged declaration keeps its key across revisions of its source (FR-091:336-338), but no AC checks this. AC-18 varies only the owner identity. Fix: extend AC-18 and TC-401 step 4 with a run under (`a`, `u`) at a different revision and digest, and require the same `Point`, `Pair` and `px` keys. | FR-091:336-338, 446; TC-401:34-37 |
| FND-007 | low | The OQ-5 ruling gives FR-008-AC-20 as the reason that the `collect` refusal "belongs to the native lane under `quire.state.queries/v1` without `quire.value.complete/v1`". QSL FR-008 names no profile, and the string `quire.state.queries/v1` appears nowhere else in this repo's spec. The profile-scoped `collect` refusal is QSpec FR-008-AC-5. Fix: cite QSpec `spec/functional/foundation/FR-008-preserve-sequence-semantics.md` FR-008-AC-5 for the profile scope, and QSL FR-008-AC-20 only as the existing native frontend's refusal. | FR-091:516; spec/functional/FR-008 (AC-20, line 76) |
| FND-008 | low | FR-091-AC-19 now carries three separate behaviours: the `Float64[nearest-even]` refusal, S1 admitting a bare `Float64`, and the `Reference<M::T>` refusal. Its S1 clause is a `qsl-cst` obligation inside an S2/assembler requirement (see integrity FND-005). Fix: split the bare-`Float64` S1 admission into its own AC, or put it in the `qsl-cst` grammar's owning requirement. | FR-091:447; TC-405:28-31 |
