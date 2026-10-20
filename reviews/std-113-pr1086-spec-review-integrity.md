---
id: SR-13423
title: 'Spec review of PR #1086: QSL cites the Quantity<u> type reference, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@626b08692; git diff refs/remotes/origin/main...626b08692 (PR #1086, base main 812c95430): spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md, spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md, spec/test-cases/TC-906-composite-arguments-replay-and-refuse-by-kind-domain-and-limit.md; read against FR-056, ADR-024 7.2 and the TC-620 test fixture on the head, QSpec PR #296 (agent-ix/quire-specification, head f789815): spec/functional/type-model/FR-142-evaluate-quantities-and-units.md, spec/objects/interfaces/FR-322-checked-package-artifact.md, spec/test-cases/TC-187-quantity-and-unit-conversion.md, proposals/quire-v1/shared-grammar.md, QSpec FR-405; ticket STD-113'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-185
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-620
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-091
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-098
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-906
  type: reviews
---

## Summary

Ticket: STD-113. PR: quire-spec-language#1086, head 626b08692, scoped to `git diff refs/remotes/origin/main...626b08692` (five files, +19/-16). Spec-only. It cites QSpec PR #296's `Quantity<u>` type form: FR-185's 'Quantity-typed parameters' section types a quantity operation parameter as `Quantity<u>` with quantity-literal supports, keeps integer supports for the `Millis = Int[1, 12]` fixture, and adds FR-185-AC-8 and TC-620 step 8 (waiting on QSL intake); FR-091's type-form lookup paragraph and OQ-11(d) now name `Quantity<u>`; FR-098 and TC-906's dependency notes cite QSpec FR-142. Integrity review: traceability (TC-620 scope and steps against FR-185's ACs), relationships, and the consistency of the dependency notes and the OQ-11 ruling row with the new text. TC-620's Scope now runs to FR-185-AC-8 and step 8 and its expected result trace AC-8; TC-620 `verifies` FR-185. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in the changed files. tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Methods: spec-review/base (SR-13421), spec-review/ears (SR-13422), spec-review/integrity (SR-13423). code-review and gap-analysis were not run, because no code or test file changed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| TC-620-scope | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| TC-620-step-8 | spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md | examined |
| FR-185-relationships | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md | examined |
| FR-091-OQ-11 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md | examined |
| FR-091-dependency-STD-113 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md | examined |
| FR-098-dependency-note | spec/functional/FR-098-execute-a-replay-request.md | examined |
| TC-906-dependency-note | spec/test-cases/TC-906-composite-arguments-replay-and-refuse-by-kind-domain-and-limit.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-091 still lists 'STD-113 (a quantity type reference in complete-V1 source)' as an open dependency, and OQ-11's Reopen cell still reads 'QSpec publishes ... STD-113's quantity type reference', the fact the new ruling (d) now relies on. Fix: cite QSpec FR-142 (Quantity type reference) in the Dependencies bullet, and give OQ-11(d) a reopen condition such as 'QSpec changes or withdraws `Quantity<u>`'. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1001 |
| FND-002 | low | FR-185, FR-091, FR-098 and TC-906 cite QSpec FR-142 for the behaviour they now depend on, but none has a relationship to ix://agent-ix/quire-specification/FR-142 (FR-185 depends_on only QSpec FR-272). Fix: add `depends_on ix://agent-ix/quire-specification/FR-142` to FR-185 and FR-091 (and FR-098 if its relationships list QSpec dependencies), so the link graph shows the dependency the prose states. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:1-13 |

## Verdict

Mergeable from the integrity side once the two low findings are fixed; TC-620's traceability to FR-185-AC-8 is complete. The PR as a whole waits on SR-13421.

## New findings (disposition pass 1)

Round 1, head 02ef49faf (fix commit 02ef49faf over 626b08692; it also commits this review's round-0 SR files under reviews/ unchanged, so this file supersedes them). Read against QSpec #296 head 4112320 (unit `typeRef` `ix://quire/unit/<name>`, three namespaces, the support carve-out, FR-142-AC-11 to AC-14, FR-322-AC-79/80). Gates: make check-spec-validation exits 0 with 17 warnings, none in the changed files; tools/check-index-completeness.sh passes. Engine probes on main d879da52 (scratch clone, TC-620 fixture with `unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;`; log sr-1086-r1-probe.log): over the `Millis` parameter `1 s` and `true` each refuse invalid_probabilistic_model/value-outside-type at the value, and `1 ms`, `3 ms`, `12 ms` still refuse value-outside-type, so steps 1 to 7 hold; with the parameter's `typeRef` set to `ix://quire/unit/ms` (and to `ix://quire/unit/Time/ms`) intake I1 refuses the package, invalid_model_binding/IntakeMalformedDeclaration at `Operation.params[0].typeRef`, from the agent-ix-semantic-ir reader (UNRESOLVED_TYPE_REF, 'the field type reference resolves to no declared type'), before S1 or the assembler run; `function f using v(d: Quantity<ms>)` is refused by S1 (invalid_syntax, 'expected `)`'); `d: ms` and `d: Time` refuse UnresolvedTypeName at assembly (FR-091-AC-33 already).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | FR-091-AC-44 and FR-185-AC-8 describe behaviour the engine refuses today (S1 for `Quantity<u>`, intake for the unit `typeRef`), and neither FR names a code ticket that owns building it. FR-185's References still name QSL-371 as its owning ticket, and QSL-920 (which listed `Quantity<Time>` parameters) is Done. The TC steps will stay untagged with no owner in Linear. Fix: name the owning code ticket in each FR's References (an existing ticket, since new Linear issues cannot be created now), or ask the lead for one. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:962 |

## Dispositions

Round 1, head 02ef49faf (fix commit 02ef49faf over 626b08692; it also commits this review's round-0 SR files under reviews/ unchanged, so this file supersedes them). Read against QSpec #296 head 4112320 (unit `typeRef` `ix://quire/unit/<name>`, three namespaces, the support carve-out, FR-142-AC-11 to AC-14, FR-322-AC-79/80). Gates: make check-spec-validation exits 0 with 17 warnings, none in the changed files; tools/check-index-completeness.sh passes. Engine probes on main d879da52 (scratch clone, TC-620 fixture with `unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;`; log sr-1086-r1-probe.log): over the `Millis` parameter `1 s` and `true` each refuse invalid_probabilistic_model/value-outside-type at the value, and `1 ms`, `3 ms`, `12 ms` still refuse value-outside-type, so steps 1 to 7 hold; with the parameter's `typeRef` set to `ix://quire/unit/ms` (and to `ix://quire/unit/Time/ms`) intake I1 refuses the package, invalid_model_binding/IntakeMalformedDeclaration at `Operation.params[0].typeRef`, from the agent-ix-semantic-ir reader (UNRESOLVED_TYPE_REF, 'the field type reference resolves to no declared type'), before S1 or the assembler run; `function f using v(d: Quantity<ms>)` is refused by S1 (invalid_syntax, 'expected `)`'); `d: ms` and `d: Time` refuse UnresolvedTypeName at assembly (FR-091-AC-33 already).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |

Round 1 verdict: not mergeable yet. Every round-0 finding is fixed (SR-13421 FND-001 to FND-006, SR-13423 FND-001 and FND-002): QSpec #296 now defines the domain-package unit `typeRef`, FR-185 cites it, AC-8 and TC-620 declare `Time`, `s` and `ms`, FR-091-AC-44 binds source `Quantity<u>`, the support carve-out matches QSpec, and the dependency notes and relationships are right. One new medium finding remains: SR-13421 FND-007 (FR-185 names S1 and FR-091 as the path for the domain-package unit `typeRef`, which intake refuses today, and FR-056 has no rule for it). Two new low findings: SR-13421 FND-008 (TC-483 step 5 assembles a check-stage refusal) and SR-13423 FND-003 (no owning code ticket). Merge order: after QSpec #296.
