---
id: SR-13622
title: 'Spec review of PR #1126: protocol refinement rows, failure domain'
type: SpecReview
analysis: failure-domain
scope: 'agent-ix/quire-spec-language@74e87f43e; git diff origin/main...74e87f43e (PR #1126, branch qsl-563-spec, base main 6eceb158b): ADR-015, FR-027, FR-098, FR-099, FR-135, FR-136, FR-146, FR-147, FR-214, FR-310, TC-444, TC-446, TC-540, TC-541, TC-551, TC-659, TC-889; read against origin/main code (qsl-semantics check, qsl-cst grammar, qsl-replay request, src/command/wire.rs) and specs (FR-071, FR-141 to FR-148, ADR-015, ADR-027) and QSpec main 8736706 (FR-322, FR-323, FR-375, FR-432, shared grammar); ticket QSL-563'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-015
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-027
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-098
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-099
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-135
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-136
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-146
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-214
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-310
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-444
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-446
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-540
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-541
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-551
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-659
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-889
  type: reviews
---

## Summary

Ticket: QSL-563. PR quire-spec-language#1126, head 74e87f43e. Failure-domain lens: unstated outcomes and edge cases of the new rules. That covers inputs to coverage that only the check request knows, refusals of the internals value for libraries that are never compiled, more than two protocols, and a refused clause next to other refinement defects.

Covered: no row naming a protocol (model subject, `concrete_protocol` None); an unknown protocol name; a clause on another model; a `memory` row with no protocol selected; an operation row no attempt uses; a refused selected clause (no coverage refusal, no `CheckedRefinement`); declaration order either way; `null`, `{}` and `7` values (S3's shape refusal at pointer ""); a dropped replay value (`PackageIdMismatch`); and a unit value never applied to a library's refinements.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-136 coverage inputs | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 memory row | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 two protocols | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 order refused | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-8 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-099 library defects | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 carry | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 no refuse | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099 own value | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-136 one protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 select rows | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Under ADR-027 SE-4 the memory model is a request input, so the memory-step classes are not known at S3. FR-136 says 'S3 coverage reads the selected clause and its resolved memory model only', and that the request inputs 'none changes which rows are required'. SE-4 says 'The memory model a `parallel` runs under is its resolved model: the request's selection for its outermost `parallel` when present, else the source default'. So a request selecting `tso` for a source-`sc` `parallel` adds a `flush` class that S3 never required a row for, and the reverse makes a `memory flush` row refuse at S3 although the run has that class. A protocol can also have several outermost `parallel` nodes with different resolved models, but FR-136 speaks of 'the memory model of the selected protocol'. State which model S3 checks memory rows against (the source default of each outermost `parallel`), and what the check request does when its selection adds or removes a memory-step kind: refuse, or settle the item as undecided. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:136-140; spec/functional/FR-136-check-a-refinement-s-step-rows.md:132-135; spec/functional/FR-136-check-a-refinement-s-step-rows.md:231 |
| FND-002 | medium | No stage owns the source-free checks of an unreached library's value. FR-099 says 'the checks of shape, spelling and repetition need no source and SHALL run for every supplied library's value', and that a shape defect 'refuses the compilation ... whether or not the unit imports it'. The input builder 'SHALL NOT refuse' the value, FR-147 assigns every defect to S3, and S3 checks per refinement of the source it compiles. An unreached library 'is not compiled' (ADR-015 D-1), so its S3 never runs and the unit's S3 does not own the value. Name the stage and order: for example, the unit's S3 checks the shape of every supplied library's value before its refinements, or a spine step after S4 resolution does. Say which locus and stage the refusal reports. | spec/functional/FR-099-compile-against-supplied-libraries.md:83-89; spec/functional/FR-099-compile-against-supplied-libraries.md:82; spec/functional/FR-099-compile-against-supplied-libraries.md:77-81; spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:166-167 |
| FND-003 | low | The two-protocol refusal does not say which rows it names, or how many refusals three protocols give. The rule refuses 'at the first row, in source order, that names the second protocol, naming both rows'. 'Both rows' presumably means the first row of each protocol, but it does not say so. With rows naming `P`, `Q` and `R`, it is unclear whether there is one refusal (at `Q`) or two (at `Q` and at `R`, each against `P`). Say that the first row names the selected protocol and each later protocol gives one refusal at its first row, naming that row and the first row of the first protocol. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:123-126; spec/functional/FR-136-check-a-refinement-s-step-rows.md:231 |
| FND-004 | low | AC-8 says a refused clause 'reports the clause's refusals alone'. The behaviour bullet suppresses only 'coverage refusals for classes drawn from it'. The model-only rows of the same refinement are still 'checked wherever the refinement is reached', and their refusals are still reported. Read literally, 'alone' forbids them. Today the code returns at mod.rs:1711 on any refinement refusal, before protocols are checked, so the fixture's contents matter. Say 'no coverage refusal for a class drawn from it', or say that the fixture's refinement has no other defect. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:233; spec/functional/FR-136-check-a-refinement-s-step-rows.md:154-156; spec/functional/FR-136-check-a-refinement-s-step-rows.md:144-146 |

## Verdict

Not mergeable on this lens. FND-001 says S3 decides coverage from a memory model that the request can still change (ADR-027 SE-4), so a check can meet a memory step with no row. FND-002 leaves a required refusal with no owning stage. FND-003 and FND-004 are low wording fixes.

## New findings (disposition pass 1)

Head 3ca0af76e (PR #1126 round 1).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The new compile of an unreached library does not say which of its refusals count, or how its own imports are resolved. FR-099 compiles it 'through S1 to S3 ... solely to resolve that value by FR-147' and says 'Its refusals are reported as for any library: `CompileRefusal::Dependency`'. This can be read two ways. Only the value's refusals count. Or every S1 to S3 refusal of the library's source counts, so an ill-typed function in a library nobody imports would refuse the unit because a value was supplied for it. The library's S3 also needs its own imports, which S4 resolves (D-1). The bullet does not say whether those imports are resolved against the supplied set, charged, and refused when missing. Say which refusals of this compile refuse the compilation, and whether its imports resolve as an imported library's do. This is a precision gap inside the owner question the PR body raises, not the choice itself. | spec/functional/FR-099-compile-against-supplied-libraries.md:87 |

## Dispositions

Round 1, head 3ca0af76e (origin/main merged at 4f0c65d6e), checked against origin/main code and QSpec main 8736706.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in 3ca0af76e: S3 SHALL decide memory-step coverage against the memory models the selected clause declares: each outermost `parallel` has the model of its source memory clause, else the source default (ADR-027 SE-4; the weak-memory record's MM-2), which S3 resolves without request input. The protocol's memory step |
| FND-002 | fixed | Fixed in 3ca0af76e: The S4 source resolution owns the refusal for a library no import reaches. For each supplied library that no import reaches and for which a refinement internals value was supplied, it SHALL compile the library's source through S1 to S3, charged as a library compile to `DependencyLimits`, solely to r |
| FND-003 | fixed | Fixed in 3ca0af76e: When rows name more than one protocol clause, the clause named by the first such row in source order SHALL be the selected one, and the checker SHALL refuse each other protocol once, `invalid_model_binding`/`conflicting-binding`, at the first row, in source order, that names it, naming that row and  |
| FND-004 | fixed | Fixed in 3ca0af76e: FR-136-AC-8 \| A protocol clause that fails its own checks, named by a refinement's rows, reports the clause's refusals and no coverage refusal for a class drawn from it, and gives no `CheckedRefinement` for the refinement; a refusal of a row not drawn from the clause is still reported. The same clau |
