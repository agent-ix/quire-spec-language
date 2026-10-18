---
id: SR-12137
title: "QSL-515 PR #930 delta code review since 49ad03552: per-layer form tests, layer fixtures, check imports"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@54ff5705807d8ea9820959148d428056f247d0a3; delta 49ad03552..HEAD, non-merge commits 74a01ecc5, d2b72ea75, 31237c084, 18a5d5c77, 4b84a2ab5, b79448146 and the conflict resolutions of merges 3deb15d4a, 8cddcfc78, 1511a7f37, 1ba9c6ac7, 54ff57058: qsl-replay/src/spine/clause/tests/layers.rs, qsl-semantics/src/value/layer.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/layer.rs, qsl-semantics/src/check/profile.rs, qsl-replay/src/spine/refinement.rs, src/complete/editor.rs, xtask/src/import_graph.rs, qsl-semantics/tests/it/complete_value_lock.rs"
review_set: subset
---

## Summary

Ticket: QSL-515. PR quire-spec-language#930. This is a delta review over
`49ad03552..HEAD`. The full review at 49ad03552 is SR-12063. Merges of
origin/main were not reviewed as content. Only their conflict resolutions
were read (`git show --cc`). The rust-review lane
(agent-skills/rust-review/SKILL.md) is folded in. The review was static: the
brief allowed no cargo and no make, so no gate was run. The local HEAD
54ff57058 is the PR head 1ba9c6ac7 plus one merge of origin/main with no
conflict resolution.

Checks:

1. `a_layer_refusal_is_prohibited` (FR-341-AC-6): the expected cause
   changed from `None` to `Some("declaration-form")`. `RefusalCause.cause` is
   `Option<&'static str>`, and the unit refuses a predicate declaration under
   state core, whose catalog pair the neighbouring test pins as
   `unsupported_construct/declaration-form`. The class stays `Prohibited`.
   The assertion now pins the cause, so it is stronger. The
   `Integer` to `Total` change (a prelude `type Total = Integer;`) changes only
   the type argument of count, sum, fold and reduce. The form under test is
   unchanged. Clean.
2. `value::layer` was added to `LAYER_PERMITTED_MODULES`. The module has no
   `use` and no `crate::` path, so it is a leaf like `value::definition`. A
   `value::<submodule>` entry permits that one submodule only. The lint is
   not widened for any other module, so this is a legitimate edge. Clean.
3. `#[trace]` and doc comments of the new tests: see FND-001 and FND-002.
4. The import re-spelling (`crate::value::{..}` to
   `crate::value::layer::{..}` / `crate::value::definition::DefinitionLock`)
   is consistent across assemble.rs, assemble/tests.rs, check.rs, mod.rs,
   layer.rs and profile.rs.
5. Merge resolutions keep both sides. 1511a7f37 keeps main's
   `UnresolvedPopulationBound`, `AmbiguousPopulationBound` and
   `UnsupportedDeclarationForm` arms beside the branch's
   `UnsupportedLayerForm` in `assembly_limit`. It keeps both enum variants,
   and both the `AuthoredBound` import and the `LayerAdmission` import. It
   keeps the FR-380 population bounds before `state_clauses(.., (&lock,
   &predicates))`. 3deb15d4a keeps the temporal alias loop beside the
   predicate set, and keeps `TemporalProfile::from_identity` beside
   `HeaderLayer::from_identity` in `resolve_profiles`. 54ff57058,
   1ba9c6ac7 and 8cddcfc78 have no conflict resolution.

Rust idioms: the `let [error] = .. else` slice pattern is used. Panics
appear only in tests and carry context strings. No new production code
besides comment text in value/layer.rs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The per-form cases never compile under any layer. Every expression case names `c` (`filter(x in c: ..)`, `Id(c)`, `R { f: c }`, `case c {..}`, `g(c)`), but no prelude declaration binds it, and `test/nodes` `Node` has only `next`. `every_form_under` keeps only `UnsupportedLayerForm` errors and treats `Ok`, any other refusal and any other assembly error alike, so an admitted case checks only that no layer refusal was raised. Several doc comments claim more. `model_complete_admits_every_form` says the model layer "admits every form", and the state-core test says a form "is admitted exactly when state core writes it". Neither is verified. Under `quire.model.complete/v1` nothing is refused, so that test only proves no layer error appears. Fix: bind `c` (for example as a parameter of a prelude function used as the body) and assert `Ok` for admitted layers, or reword the docs to "raises no layer refusal". | qsl-replay/src/spine/clause/tests/layers.rs:416-482 |
| FND-002 | medium | Two traces bind criteria the tests do not verify. (a) `the_five_header_layers_are_accepted_by_a_catalog_that_holds_them` is tagged `FR-110-AC-7`, but it builds its own `ProfileCatalog` from `HeaderLayer::ALL` and checks the editor accepts what that catalog holds. AC-7 is a spine compile and lock-contents criterion. The test supports the FR-110 bullet on the editor's `unsupported-selection` pair, which has no AC. (b) `state_graph_admits_only_its_closure`, `value_complete_admits_its_forms` and `model_complete_admits_every_form` are tagged `FR-110-AC-8`, which states only the state-core refusals and state-queries compiles. These three verify nothing AC-8 states. | src/complete/editor.rs:401-417; qsl-replay/src/spine/clause/tests/layers.rs:493-516 |
| FND-003 | low | `every_form_under` picks whether to prepend the prelude by comparing `case.name` with two literals. A `prelude: bool` field on `FormCase` would not break when a case is renamed. | qsl-replay/src/spine/clause/tests/layers.rs:428-433 |

## Verdict

PASS with mediums. No major bug. Checks 1, 2 and 5 are clean. The
FR-341-AC-6 expectation is right and stronger, the `value::layer` edge is
legitimate, and every merge resolution kept both sides. FND-001 and FND-002
answer check 3: some docs and traces claim more than the tests verify. They
are recorded under the major-only bar for this last round. Deleting the
literal row-listing tests (added in d2b72ea75, removed in 74a01ecc5 with
the matching FR-110-CON-1 sentence) was correct. They mirrored the table
and tested nothing behavioural, and the behavioural per-layer tests replace
them. SR-12064 FND-001 (untested classifier arms) is addressed by
74a01ecc5's cases on the refusal side.
