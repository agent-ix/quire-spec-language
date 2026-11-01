---
id: SR-1951002
title: "Gap analysis of PR #1951 (QSL-1315): FR-322 anchor ordinal, foreign unit owner, nominal regions"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1951 diff (three FR-322-AC-51-tagged tests and the code they exercise); ticket QSL-1315; QSpec FR-322"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-322
  type: reviews
---
# SR-1951002: gap analysis of PR #1951 (QSL-1315)

## Summary

Ticket: QSL-1315. Plan completion: not assessed. The criteria-to-test check was done by hand over the diff, comparing each new test with FR-322's acceptance criteria in QSpec. All three new tests are tagged `FR-322-AC-51`. AC-51 is the owner-presence criterion, and only one of the three tests asserts an owner. No live FR-322 AC covers the anchor ordinal or the nominal declaration region: AC-118 and AC-139 are withdrawn.

Reverted-code runs: with the PR's source reverted and its tests kept, each of the three tests fails on the old code.
- The anchor test gets ordinals `[(0, ..), (0, ..)]`.
- The ModelOwner test panics at tests/it/dimension_equivalence.rs:589 (no declaration-less dimension).
- The region test reads `"time Config dense unit ms;"` where it expects `"Time"`.

All three pass at the head.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-322-AC-51 | spec/objects/interfaces/FR-322-checked-package-artifact.md:3317 | examined |
| FR-322 dimension equivalence anchor rule | spec/objects/interfaces/FR-322-checked-package-artifact.md:1598-1601 | examined |
| FR-322-AC-1 | spec/objects/interfaces/FR-322-checked-package-artifact.md:3272 | context_only |
| FR-322-AC-81 | spec/objects/interfaces/FR-322-checked-package-artifact.md:3347 | context_only |
| equivalence_anchor_ordinal_is_the_declaration_index | qsl-semantics/src/check/lowering/tests/dimension_equivalence.rs:200-239 | examined |
| a_package_dimension_node_is_model_owned_with_no_declaration | qsl-semantics/tests/it/dimension_equivalence.rs:563-605 | examined |
| a_source_time_nominal_is_located_at_its_own_declaration | qsl-package/src/emit/tests/timed_source.rs:167-208 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `equivalence_anchor_ordinal_is_the_declaration_index` is tagged FR-322-AC-51, but it asserts the anchor ordinal and region and no owner. AC-51 is about owners only. The criterion it actually checks is the "Nominal nodes"/equivalence anchor rule, which has no live AC because AC-118 is withdrawn. The matrix therefore counts AC-51 as covered by a test that cannot fail on an owner defect. | qsl-semantics/src/check/lowering/tests/dimension_equivalence.rs:200 |
| FND-002 | medium | `a_source_time_nominal_is_located_at_its_own_declaration` is tagged FR-322-AC-51, but it asserts only a source-map region. No live AC states the nominal declaration region; AC-1 requires only that some region exists. This is the same mis-trace as FND-001. | qsl-package/src/emit/tests/timed_source.rs:167 |
| FND-003 | medium | FR-322's equivalence paragraph requires the anchor ordinal to be the declaration's index among the source's equivalence declarations, with the declaration's region, but no AC states it. The behaviour this PR fixes is unverifiable from the matrix. QSpec needs an AC, for example "two equivalence declarations E0, E1 in source order carry anchor ordinals 0 and 1, each with its declaration's region". Until it has one, the two tests above have nothing correct to trace to. | spec/objects/interfaces/FR-322-checked-package-artifact.md:1598-1601 |

## Verdict

**CONDITIONAL.** The code changes are real and not stubs, and each test fails on the reverted code. The gaps are traceability gaps:
- Two tests are tagged to an owner criterion they do not check.
- The anchor-ordinal rule has no AC.
- The ModelOwner test checks only the in-memory graph, not the wire (see SR-1951001 FND-001).

Binding check: `a_package_dimension_node_is_model_owned_with_no_declaration` → FR-322-AC-51 is correct. It asserts the `ModelOwner {identity, node}` on a model-owned dimension, and the `SourceOwner` on the unit's declared dimensions.

## Coverage

Plan completion: not assessed. The criteria-to-test check was done by hand over the PR diff and FR-322's AC table. The `quire matrix` run was not used. Semantic review: done over the diff, comparing FR-322's text with the tests and the code.

## Dispositions

Round 1 was checked against the PR head after the first fix round, with QSpec PR #552 for FR-322-AC-287.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `equivalence_anchor_ordinal_is_the_declaration_index` is now tagged `FR-322-AC-287`, which QSpec #552 adds. The tag resolves only once #552 merges. |
| FND-002 | fixed | `a_source_time_nominal_is_located_at_its_own_declaration` is now tagged `FR-322-AC-1`. |
| FND-003 | fixed | QSpec #552 adds FR-322-AC-287 and TC-233 EA-01 for the anchor ordinal (see SR-552001 for its wording defect). |

Round 2 was checked against the PR head after the commit that rebuilds the FR-062-AC-9 omission test.

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | `a_function_naming_an_omitted_node_is_omitted_and_the_other_is_written` (union_nodes.rs:233-294) rebuilds the two-function case on the undecodable union, one of the two options the finding named. TC-160 step 8 now describes that union. What it still misses is FND-007. |
| FND-005 | fixed | The anchor test now declares `B ~ C`, then `C ~ D`, then `A ~ B`. That source order is the reverse of the node-id order, with a third declaration in between, and the test expects ordinals 0, 1 and 2 by source position (dimension_equivalence.rs:204-247). |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Deleting `a_compound_unit_is_omitted_only_for_its_omitted_unit` removes the only test of FR-062-AC-9's first half and of TC-160 step 8. That half: one function names an omitted node and is omitted with `NamesOmittedNode` along its path, the other is written, and the I2 read exports only the second. The tests the coder points to exist: the foreign enum's `UnlockedOwner` at emit/tests.rs:1068, and `NamesOmittedNode` closure at union_nodes.rs:93-160, traced FR-320-AC-2. But neither asserts AC-9's two-function split or the I2 export, and FR-062-AC-9's remaining tag is on the second-half test only. TC-160 still describes the definition-owned `metre` scenario (spec/test-cases/TC-160-checked-family-contract-shape.md:71-73, 137-142). The claim that no source-text producer reaches a definition-owned unit is true: the assembler builds the table from source declarations and imported model bases only. The public `PackageDeclarations.types` path still can reach one, and now refuses `UnheldNominal` at check. The fix is to rebuild the two-function case on an omission that still exists (the unlocked foreign enum, or the undecodable union) and update TC-160 step 8. | qsl-package/src/emit/tests.rs:1887-1903; spec/test-cases/TC-160-checked-family-contract-shape.md:137-142 |
| FND-005 | medium | The anchor test cannot tell source order from node-id order. `A ~ B` is declared before `B ~ C`, and the equivalence node ids sort the same way, so a producer that numbered anchors by node-id order would pass. AC-287's "whatever their node-id order" and its third-equivalence-in-between case are both unexercised. Declare `B ~ C` first, then `A ~ B`, and add a declaration between two others. | qsl-semantics/src/check/lowering/tests/dimension_equivalence.rs:200-239 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The new tests use bare `FR-322-AC-*` trace tags, and under AGENTS.md "Trace tag ids" a bare id names a QSL artifact. QSL's FR-322 is "Evaluate union construction and case", with AC-1 to AC-4. As a result, `a_source_time_nominal_is_located_at_its_own_declaration` binds QSL FR-322-AC-1 (`area` returns 0, 27 and 6), which it does not test. That tag was the round-1 fix for FND-002. `FR-322-AC-51`, `-81` and `-287` on the other three new tests resolve to no QSL criterion, so they bind nothing. The QSpec criteria they mean are `QSpec-FR-322-AC-1`, `-51`, `-81` and `-287`; QSpec #552, which adds AC-287, has merged. The same bare form already exists on main at dimension_equivalence.rs:57 and :161 and admission_corpus.rs:2289. | qsl-package/src/emit/tests/timed_source.rs:167; qsl-package/src/emit/tests/timed_source.rs:215; qsl-semantics/tests/it/dimension_equivalence.rs:563; qsl-semantics/src/check/lowering/tests/dimension_equivalence.rs:202 |
| FND-007 | medium | FR-062-AC-9 is still not verified as written. (a) AC-9's second sentence requires "the same package" to refuse with `UnlocatedOccurrence`. The only test of that half, `the_q_and_t_package_with_an_unplaced_occurrence_refuses`, still emits the `q`/`t` metre package, not the `area`/`t` package of the first half. (b) The first half goes through `emit_package_decoding` with an injected decoder, not through `emit_checked` as AC-9 states (see SR-1951003 FND-001). (c) TC-160 step 8 expects the union node to be omitted with `UnsupportedForm`; the new test never asserts the union's cause. It also checks "every node on its path" only as at least one `parameter` and one `case` node. Fix: assert the union's `UnsupportedForm`, and emit the `area`/`t` package through a region conversion that places nothing, expecting `UnlocatedOccurrence`. | qsl-package/src/emit/tests/union_nodes.rs:233-294; qsl-package/src/emit/tests.rs:1887-1903; spec/functional/FR-062-implement-checked-family-contract.md:375 |

Disposition pass 2 checks, run as focused tests on the PR head:
- qsl-semantics lib: 7 passed, including the reversed-order anchor test.
- qsl-semantics `--test it` filtered to `dimension_equivalence` with `test-support`: 3 passed.
- qsl-package `emit::tests`: see the report.
