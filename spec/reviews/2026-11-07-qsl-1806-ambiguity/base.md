---
id: SR-20261107180601
title: "Base review of the QSL-1806 unit ambiguity and QSL-1093 FR-488 change"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language branch spec/qsl-1806-ambiguity, diff origin/main...HEAD: FR-505 (bare unit resolution, AmbiguousUnitName, AC-9, AC-11, AC-12), FR-091 (Dimension and unit declarations, Catalog codes, AC-21), FR-488 (QSpec FR-402 adoption), TC-1300 rows 19, 27, 28, TC-406, TC-1266; read against QSpec FR-142 Namespaces, FR-272, FR-322 and FR-402 (branch spec/qsl-1093-fr402)"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-505
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-488
    type: reviews
---

## Summary

Tickets QSL-1806 and QSL-1093 (FR-488). This review covers only the
committed diff `origin/main...HEAD` of branch `spec/qsl-1806-ambiguity`. It
uses the base checklist and runs no analysis lenses.

What holds:

- **IDs.** FR-505-AC-12 is new and absent on `origin/main`.
- **Coverage.** FR-505-AC-12 is covered by TC-1300 row 28. The new
  AC-11 sentence is covered by row 27, and the AC-9 change by row 19. The
  amended FR-091-AC-21 is covered by TC-406.
- **Cause reference.** The new text cites QSpec's cause vocabulary as
  "(QSpec FR-272)" only, and `ambiguous_declaration`/`ambiguous-name` is an
  FR-272 code and cause.
- **Ruling (b) is sound.** In the fixture,
  `qsl-semantics/tests/it/model_operations.rs` `config_unit_with_profiles`
  writes `model Config = ...`, so `Config` is the model alias. The old text
  called it a config with a member `m`, which was wrong. `Config::m` naming
  the selected document's unit agrees with QSpec FR-142 "Namespaces": a
  package unit is named through the selection's model alias, `M::ms`.
- **FR-488 against QSpec FR-402.** FR-488 matches FR-402 on the quantifier
  Groups, `model_root` subjects, fairness references, the body wrapper, the
  slot encoding and the constant-conjunct reference. The only exceptions are
  FND-006 and FND-007.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-505 and FR-091 now say a bare unit name the source does not declare reaches a selected package's unit, and is ambiguous when two or more packages declare it. QSpec FR-142 "Namespaces" says the opposite: "A package unit carries no declaration name: a source names it through the selection's model alias, `M::ms`", and a source unit `ms` and a package unit `ms` "are two units and never one ambiguous name". It also says no search across packages applies, at least for dimensions. Under QSpec, a bare `x` with no source `x` is `missing_declaration`/`missing-name` (which `UnresolvedTypeName` maps to), not `ambiguous-name`. The new rule extends a QSL-only divergence that already exists: AC-9's bare `m` reaching the document's unit. Fix (owner decision): either (a) record a QSpec open question in FR-505 that cites FR-142 "Namespaces", and land the ambiguity rule only once QSpec states bare-name lookup of package units with this ambiguity refusal; or (b) align with QSpec, so a bare `x` stays `UnresolvedTypeName` (`missing_declaration`/`missing-name`) and its cause's details name the declaring packages as a hint, with no new cause. | spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md:77-89; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:509-512; QSpec spec/functional/type-model/FR-142-evaluate-quantities-and-units.md "Namespaces" | wrong-requirement |
| FND-002 | medium | The new rule covers bare names in source only. It does not say how a model document's value type `unit` `ix://quire/unit/x` resolves when two selected packages each declare `x` and the source declares none. FR-505's existing text says "the model document fills names the source lacks", but that does not say whether the document's own `x` wins or the name is ambiguous. Fix: state that a value type's `unit` falls back to its own document's unit only and is never ambiguous, and add that case to FR-505-AC-12 and TC-1300 row 28: an `acme/a` value type with `unit` `ix://quire/unit/x` types over `acme/a`'s `x`, with no refusal. | spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md:60-89,244; spec/test-cases/TC-1300-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md:44 | wrong-requirement |
| FND-003 | low | Ruling (a) says a source unit of the same name, of any dimension, replaces a document's unit. This is consistent with QSpec FR-322's `Quantity<u>` row, where `ix://quire/unit/<name>` names a source unit and a package unit is `ix://quire/unit/<pkg>/<name>`. But it lets an unrelated source declaration silently move a document's value type from `Length` to `Time`, and the refusal then shows up at a distance, as the AC-11 conversion's `type-mismatch`. QSpec also disagrees with itself here: the FR-322 row and TC-981 RV-09, where a document's own `m` is reached by `ix://quire/unit/m`, cannot both hold. Fix: keep the ruling. Raise the FR-322 row versus RV-09 question to QSpec, and ask the owner whether a dimension-changing shadow should refuse instead. | spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md:62-75,243 | wrong-requirement |
| FND-004 | low | FR-505's new rule names three positions: a `Quantity<x>` type form, a quantity literal's unit, and a `convert<Quantity<x>>` target. FR-505-AC-12 and TC-1300 row 28 test only the type form. Fix: add `1 x` and `convert<Quantity<x>>(e)` to AC-12 and row 28, each refusing `AmbiguousUnitName` at `x`. | spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md:77-80,244; spec/test-cases/TC-1300-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md:44 | correct-requirement-no-evidence |
| FND-005 | low | TC-1300 rows 27 and 28 sit under the header "Each row below mutates `R0` as stated and admits it", but row 28 is "independent of R0" and both rows expect refusals. Fix: move rows 27 and 28 to their own table with their own lead sentence, or change the header to "unless the row says otherwise". | spec/test-cases/TC-1300-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md:29,43-44 | correct-requirement-no-evidence |
| FND-006 | low | FR-488 "Grammar fit" still says `match` is "an `aggregate` of Leaves (references, or a literal for a constant conjunct)". QSpec FR-402 and FR-488's own "Match conjuncts" say a constant conjunct is a `reference` to a `value`/`literal` node, never an inline literal. Fix: replace the parenthesis with "(references; a constant conjunct is a reference to its `value`/`literal` node)". | spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md:168 | wrong-requirement |
| FND-007 | low | FR-488 refusal 2 now reads "QSpec FR-402 gives such an alias its package's `model_root` as subject, and the emitter does not yet emit one". QSpec FR-322 derives a `model_root` node for every selected package, and FR-488 already references it for aliases that have object parameters, so the stated reason does not hold. Fix: state the real gap (for example, that `aliases()` enumerates aliases from object parameters only, so an alias with none gets no `subjects` binding), or lift refusal 2 in this change. | spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md:370-373 | wrong-requirement |

## Verdict

The FR-488 adoption of QSpec FR-402 is consistent apart from two stale
sentences, FND-006 and FND-007. The QSL-1806 ambiguity cause is well
specified and covered by tests. But it contradicts QSpec FR-142
"Namespaces", which reaches package units only through `M::name` (FND-001).
The owner must decide between a QSpec ruling and QSpec alignment before this
merges. Ruling (b), `Config::m`, is correct. Ruling (a) is consistent with
QSpec's namespace reading but rests on a QSpec inconsistency (FND-003).
FND-002 is a real gap in the new rule. FND-004 and FND-005 are coverage and
layout fixes.
