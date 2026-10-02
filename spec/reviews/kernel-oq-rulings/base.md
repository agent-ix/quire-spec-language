---
id: SR-509
title: "Kernel-convergence rulings OQ-A to OQ-F base review (PR #351)"
type: SpecReview
analysis: base
scope: "PR #351 diff (origin/main...spec/kernel-oq-rulings): spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-067, FR-084, FR-088, FR-091, spec/test-cases/TC-252, TC-398, TC-409, TC-410, spec/spec.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
---
# SR-509: Kernel-convergence rulings OQ-A to OQ-F base review

## Summary

Base-checklist review of how PR #351 records the QSL lead's rulings OQ-A to
OQ-F. The rulings themselves are decided and are not reviewed here. Scope is
the diff only (`git diff origin/main...HEAD`, one commit `7076b34b`). The
checks were: ID format and uniqueness, AC testability, TC-to-AC trace,
cross-references, and the repo's authoring rules (state what is, no roadmap
in FRs, no compatibility wording, cite QSpec and do not copy it). QSpec
citations were checked against `agent-ix/quire-specification` at `2449ceb`.

ID hygiene passes. TC-409 and TC-410 are new and unused. FR-084-AC-7,
FR-088-AC-11 and FR-091-OQ-9 continue their sequences. SR-509 to SR-512 were
unused. `quire validate` over `spec/**/*.md` reports no errors. Every new AC
has a TC, and each TC traces back to its AC in `spec/tests.md` (lines
191-192). The code citations in the diff (`src/model/normalize.rs:548-550`,
`:1819-1832`, `src/model/population.rs:389`, `:501`,
`quire-exact/src/identity.rs:105-119`, `quire-exact/src/key.rs:103`,
`src/value/unit.rs:36`, `:39`, `src/value/enumeration.rs:34`, `:195`,
`src/check/identity.rs:570`, `src/check/family.rs:349-365`) all resolve to
the code they describe.

## Verdict

**ACCEPT WITH FINDINGS.** Two medium findings: QSpec definitions are restated
rather than cited, and the OQ-B ruling has no criterion. Three low findings
cover wording and citations. The substantive defects are in SR-510
(integrity) and SR-511 (failure-domain).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSpec text is restated instead of cited, against the repo rule "never copy QSpec text, cite by reference". ADR-013 `:197-223` (the "Reference identity components" block) paraphrases quire-specification `model-complete.md` "Object universe" and "Reference identity key" (`model-complete.md:228-251` at `2449ceb`) almost line by line, including "no parsing, trimming or normalization", "the domain string followed by the 32 digest bytes" and "unsigned bytewise with a proper prefix first". FR-084 `:153-167` restates the same universe definition and preimage again. The O-14 "Sum types" cell (`:360`) and FR-088 `:169-186` restate the FR-144 enumeration key row. **Fix:** keep only the QSL decision in each place: which kernel type carries which component (`UniverseId`, `EffectiveId`, `ObjectId`), that `model` computes the universe, and that the rank carries the FR-144 key. Replace the restated definitions with citations, e.g. "as `model-complete.md` 'Object universe' defines it". FR-084-AC-7 and FR-088-AC-11 may keep their concrete expected values, because an AC needs them. | ADR-013:197-223, :360; FR-084:153-167; FR-088:169-186 |
| FND-002 | medium | The OQ-B ruling (`UnitId` is a two-domain digest record, and a declared unit is never equal to a compound unit) has no acceptance criterion and no TC. `spec/tests.md:246-247` says so: "OQ-B's `UnitId` shape has no QSL criterion; ADR-013 T-6 states it". The ruling has testable consequences: (a) a compound `UnitId`'s bytes match the QSpec compound-unit vectors (`value-compound-unit-vectors.json`, FR-142-AC-8); (b) a declared unit `m` and the compound unit `{m: 1}` are unequal `UnitId`s, so adding them refuses `ill_typed` (FR-142-AC-2); (c) `km` and `m` are distinct `UnitId`s, the case the OQ-B reason cites. The same applies to `ObjectId` as unparsed, untrimmed bytes, which FR-084-AC-7 covers only with plain ASCII. **Fix:** add an AC for (a) to (c) to the FR that owns the kernel `Quantity` payload (FR-088 or the QSL-131 kernel FR) and a planned TC under QSL-131. Then delete the "has no QSL criterion" sentence from `tests.md:246-247`. | ADR-013:848 (T-6), :1093 (OQ-B); spec/tests.md:246-247 |
| FND-003 | low | `spec/tests.md:238-248` opens with narration ("The QSL lead's kernel-convergence rulings of 2026-09-22 ... add two criteria"). The coverage section should say what is covered. **Fix:** "FR-088-AC-11 (TC-409) and FR-084-AC-7 (TC-410) cover ADR-013 OQ-D/OQ-F and OQ-C/OQ-E; both are `🚧 Planned` under QSL-131. TC-398 covers OQ-A's layer-2 edge." | spec/tests.md:238-248 |
| FND-004 | low | FR-091 `:242-247` says the builder "builds kernel values without calling a kernel operation", and ADR-011 `:693-694` says "The edge admits kernel types, not kernel operations". Neither defines "kernel operation". Building a `quire_exact::Integer` from literal digits calls a kernel constructor or parser, and `Expression::Rational(Integer, Integer)` (`src/forms/syntax.rs:133`) takes two of them. No AC observes the claim: FR-091-AC-11 and TC-398 check `use` edges only. The OQ-A reopen condition ("`forms` needs a kernel operation") therefore cannot be checked. **Fix:** in ADR-011 `:693`, define a kernel operation as ADR-011 §6.1's K-row "scalar and collection operations" (evaluation, metering, comparison), and state that constructors and parsers of kernel types are not operations. Either drop the FR-091 sentence or add a TC-398 step that scans `forms` for calls into `quire_exact` operation modules. | ADR-011:690-698; FR-091:242-247, :346; TC-398 |
| FND-005 | low | ADR-013 OQ-C (`:1094`), T-6 (`:848`) and QC-15 (`:1026`) attribute "non-empty" UTF-8 to QSpec FR-204. FR-204 at `2449ceb` does not say non-empty. The source is FR-035 (`spec/functional/foundation/FR-035-bind-ecosystem-subjects.md:49`: "non-empty object-identity bytes supplied by the runtime input"). **Fix:** cite "FR-204, FR-035" wherever non-emptiness is stated. | ADR-013:848, :1026, :1094 |
