---
id: SR-12148
title: "Gap analysis of quire-spec-language PR #983: FR-150 symmetric population annotation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5e191733f2bd0688a14d30214131c4345c07f8b4; PR #983: FR-150-AC-1..4 (TC-565, TC-566) against qsl-semantics/tests/it/symmetric_population.rs and the qsl-semantics changes; QSpec FR-381 / TC-320 by id"
review_set: subset
---
# Gap analysis of quire-spec-language PR #983

## Summary

Ticket: QSL-539. Plan completion: not assessed.

Checked each `#[trace]` tag in `symmetric_population.rs` against the AC it names, and the FR-150 Behavior items against the code:

- FR-150-AC-1 / TC-565: `an_annotated_population_checks_and_changes_the_package_selection` checks the unit over the annotated and unannotated package, asserts the model selections differ, and reads `symmetric` back from the records. Correct. It can fail on each clause.
- FR-150-AC-2 / TC-565: the annotated half is asserted (cause, span at the `fold`, `fold`, `config_history`). The unannotated half asserts the opposite of the AC (FND-001).
- FR-150-AC-3 / TC-566: conversion, `reduce` and `fold` in an invariant and a `terminal when`, and two refusals in source order are asserted. Reference literal, temporal atom and state constraint have no S3 form today (PR body items 2-4); those clauses stay uncovered (FND-003).
- FR-150-AC-4 / TC-566: equality, `deref`, `reaches`, `size`, `contains`, `count` check. The AC's `forall` invariant, `holds` atom and integer `fold` are not in the test (FND-002).
- FR-150 Behavior item 1, "other traversal" over a `Set`/`Bag`: a `Set`/`Bag` to `Sequence` conversion is not refused and has no test. Recorded once, in SR-12147 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-150-AC-2's second clause ("The same unit without the annotation checks") contradicts FR-145's catalogue rule: an uncatalogued `fold` over an unordered `Set` already refuses `operator-ineligible`, and a step that chooses a reference is never catalogued. QSpec FR-381-AC-2 states the same clause. `a_fold_over_references_into_an_annotated_population_refuses` is tagged `FR-150-AC-2` but asserts the refusal, the opposite of the AC. Restate FR-150-AC-2 so the unannotated unit keeps `operator-ineligible` (and raise the same edit on the QSpec side), or pick an unannotated form that does check. Until then the tag claims a clause the test contradicts. | qsl-semantics/tests/it/symmetric_population.rs:181-197; spec/functional/FR-150-check-the-symmetric-population-annotation.md:93 |
| FND-002 | medium | `transparent_uses_of_an_annotated_population_check` is the only test tagged FR-150-AC-4. Its doc comment says it checks "a `fold` over a set of integers", but the body has no `fold`. It also omits the AC's `forall p in config_history: p.parent = self implies p != self` invariant and the `holds(c.parent = a.parent)` atom. The tag claims all of AC-4; the test can fail only on the equality / `deref` / `reaches` / `size` / `contains` / `count` clauses. Add the `forall` invariant and a `fold` over a `Set<Int[0, 3]>` field, and fix the doc comment. | qsl-semantics/tests/it/symmetric_population.rs:262-276; spec/functional/FR-150-check-the-symmetric-population-annotation.md:95 |
| FND-003 | medium | The two tests tagged FR-150-AC-3 cover the conversion only in an invariant (the AC puts it in a temporal atom) and `reduce` only over a `Set` (the AC says a `Bag`). Reference literal, temporal atom and state constraint have no S3 form (PR body items 2-4), so those clauses cannot be tested here and need follow-up tickets. Add a `Bag` field for the `reduce` case now; record the other three as remaining work on QSL-539 or a follow-up ticket rather than under this tag. | qsl-semantics/tests/it/symmetric_population.rs:199-260; spec/functional/FR-150-check-the-symmetric-population-annotation.md:94 |
| FND-004 | medium | FR-150 defines a read as a type that "is, or contains" a reference, through `Option`, collections and records. `symmetric_population_in` walks `Option`, collection elements, record fields, tuple positions and union payloads, but every test reaches it only through a direct `Reference` or `Set<Reference>`. No test covers a reference inside an `Option`, a record field or a union payload, so a broken arm would go unnoticed. Add one refusing case through a nested type (for example `convert<String>` of `self.parent`, an `Option<Reference<ConfigVersion>>`, which is also TC-320 step 3's form). | qsl-semantics/src/check/check.rs:883-925; spec/functional/FR-150-check-the-symmetric-population-annotation.md:60-63 |
| FND-005 | low | Two intake branches have no test: a non-Boolean `symmetric` refusing `malformed-declaration` (intake.rs:3979-3985), and the schema filter leaving another extra population member refused. Add both to the TC-565 test file. | qsl-semantics/src/model/intake.rs:3979-3985; qsl-semantics/src/model/intake.rs:1486-1504 |

## Verdict

Changes requested at medium: FND-001 to FND-004 are medium and FND-005 is low; none blocks on its own. The blocking gap for this PR is SR-12147 FND-001 (Set/Bag to Sequence conversion unrefused and untested).

Clean: FR-150-AC-1's test can fail on each clause; the annotated half of AC-2, the `reduce` and conversion refusals, the `terminal when` refusal and source-order reporting are asserted with code, cause, payload and exact span.
