---
id: SR-1716002
title: "Gap analysis of quire-spec-language PR #1716: the model-abstract refinement class catalog (QSL-1181 slice 2)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e39172d9e; PR #1716 diff against qsl-1181a-typed-refinement (12 files), against FR-375-AC-23 (operation part), FR-375-AC-27, FR-322-AC-130 (declared part), FR-322-AC-133, FR-464-AC-18 and FR-432 (QSpec main 308596b0)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-375
    type: reviews
---
# Gap analysis of quire-spec-language PR #1716

## Summary

Ticket: QSL-1181 (slice 2 of 6). Plan completion: not assessed.

**Covered.**

- **FR-375-AC-23, operation part.** Covered by `a_model_abstract_catalog_lists_effective_operations_with_attempt_records`, `an_ambiguous_inherited_operation_is_no_class_of_the_subtype` and `the_catalog_lowers_to_nodes_of_the_package_graph`:
  - effective (inherited) operation classes keyed by their owner;
  - the receiver/params/result record;
  - an ambiguous name gives no class;
  - the abstract model's operation classes on the abstract side.
- **FR-375-AC-23, row-free contribution** ("including when no step rows name those classes"). `a_concrete_protocol_gives_a_model_refinement_its_class_catalog` covers it: a row-empty refinement gets the clause's classes.
- **FR-322-AC-130, declared part.** The same package test covers:
  - the class spellings on both sides;
  - local node references;
  - every catalog node and binder type in `dependencies`;
  - a different catalog giving a different node id.
- **FR-322-AC-133, order by catalog key.** Covered by the byte-ordered spelling assertions on both sides.
- **FR-464-AC-18.** Catalog selection resolves the clause owner from the same `over`-type rule as the `owner_model` edge, so the join is by exact clause owner.

**Not covered by any test in this PR. My probes show each behaviour is correct, so these gaps need tests, not fixes.**

- **FR-375-AC-27: the `activate` entry.**
  - This PR changes its `binder_types` from `[]` to the trigger's binder record and names the clause node, but no test asserts either.
  - Probe: `on each (t: Boolean)` gives `[Boolean]`, and `on each (k: Impl::Counter)` gives one reference type.
  - The package probe emits `activate Local` with one binder.
- **FR-375-AC-23: foreign clauses are excluded** ("excludes clauses whose edge names a model owned by another declaration").
  - The package test checks a clause over the *abstract* model.
  - It does not check a clause over a third model the package also selects.
  - Probe with a third model `T`: no catalog, and the refinement emits.
- **The not-emitted path.** No test checks the PR's claim that a contributing clause without a node leaves the refinement not emitted (`class_catalog_keys` returns `None`, so emission refuses `RefinementNotEmitted`).

**The read-back test.** `a_model_refinement_class_catalog_reads_back` asserts the FR-375 outcome and is `#[ignore]`d citing IR-874. I re-measured the claim: IR refuses `invalid_semantic_graph` at `class_catalog/concrete/1`. IR-874's comments assign B2/B3 to luna:IR:codex3.

The ignore is acceptable; SR-1716001 gives the reasons. But neither QSL-1181 nor IR-874 lists un-ignoring this test as a completion item. An ignored test never runs in `make ci`, so the ignore can outlive the IR fix unnoticed.

**A spec gap.** QSpec fixes the `activate` entry fully (FR-375 "Checked class catalog", FR-375-AC-27). It does not fix a *model operation class*'s spelling, node or binder record:

- FR-375 says only that the concrete catalog "covers the selected concrete model's operation classes".
- FR-432's `attempt` row describes a protocol attempt node, not a model operation class.

QSL's choices are `<Owner>::<op>`, the owner's model node, and the attempt record. IR's rebuild accepts `Lifecycle::tick` at `concrete/0`, so the two agree in practice. But the spec should state the rule so that a third reader agrees too.

## Verdict

**The slice's ACs are met for the constructs it lowers.** One medium test gap and two low tracking/spec items remain. None of them is a wrong result.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three behaviours this PR adds or changes have no test. (a) FR-375-AC-27: the `activate <P>` entry's node is the clause node, and its `binder_types` are the trigger's binder record, `[T]` for a non-record trigger. This PR changed it from `[]`. (b) FR-375-AC-23's exclusion of a clause over a third selected model. (c) The not-emitted refusal when a contributing clause has no node. Probes for (a) and (b) pass. Add the tests in this PR. | qsl-semantics/tests/it/refinement_class_catalog.rs; qsl-package/src/emit/tests/admission_corpus.rs; probe_1716.diff |
| FND-002 | low | The `#[ignore]` on `a_model_refinement_class_catalog_reads_back` has no tracked exit. Neither QSL-1181 nor IR-874 lists un-ignoring it, and ignored tests do not run in `make ci`. Record "remove the ignore when the IR dependency moves past IR-874's B2/B3" on QSL-1181, in the slice that moves `quire-contract-model` off the `qsl-1181-refinement-exports` branch. | qsl-package/src/emit/tests/admission_corpus.rs:1319-1337; QSL-1181; IR-874 |
| FND-003 | low | Spec gap: FR-375 and FR-432 do not state a model operation class's spelling, `node` or `binder_types`. QSL uses `<Owner>::<op>`, the owner's model node, and FR-432's attempt record (receiver, parameters, result). File it on STD-493 in place of the `activate` item, which QSpec already states (SR-1716001 FND-002). | QSpec FR-375 "Checked class catalog"; FR-432 "The binder record of an abstract node" |

## Dispositions

Round 1, fix commit 32cbba757. The gates are those in SR-1716001's dispositions.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | (a) Covered by `an_each_clause_has_an_activate_class_with_the_trigger_record` and `an_each_clause_writes_its_activate_class`. They check `[Boolean]` and one reference type, `Trigger("Local")` lowered to the clause node, the wire `activate Local` whose node is the `protocol_clause` node and whose binder is a `boolean` node, and an `origin` clause with no `activate` class. (b) Covered by `a_clause_over_a_third_model_contributes_no_class`: the clause over `T::Widget` is emitted, and the refinement has no catalog. (c) Covered by `a_clause_class_with_no_node_leaves_the_catalog_unlowered`. A `parallel` (`fork Local::par`) gives a checked catalog with `class_catalog_keys` `None`, and `v2_empty_model_body()` is false. This is equivalent to testing the refusal, because that predicate is the whole input to the not-emitted gate: check/mod.rs gives `CheckCause::RefinementNotEmitted` exactly when `!v2_empty_model_body()`, and emit.rs gives `EmitRefusal::RefinementNotEmitted`. `a_checked_refinement_refuses_not_emitted_at_package_build` already tests that predicate-to-refusal mapping. The test has to switch on unwritten protocols and refinements because, in a default build, the `parallel` clause refuses as not emitted first. So the refinement's own refusal cannot be reached through this path end to end, and the predicate is the right thing to assert. |
| FND-002 | fixed | QSL-1181 comment 3b349555 records the completion item: remove the IR-874 ignores (this test and slice 3's) in the slice that moves `quire-contract-model` to an IR main containing B2 and B3, and each test must then pass as written. |
| FND-003 | fixed | Filed on STD-493 (comment b1db40e4): the spelling, `node` and `binder_types` of a model operation class, with QSL's rules and the ask to state them in FR-375 "Checked class catalog". |
