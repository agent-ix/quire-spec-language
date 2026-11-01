---
id: TC-150
title: "Preserve authenticated temporal selections on the checked route"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-044, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/TC-138, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1278, type: references }
---
# TC-150: Preserve authenticated temporal selections on the checked route

## Description

Verify FR-050's producer and strict-reader contract on the replacement checked
route. The test begins with the actual native `/2` emitter bytes, admits those
bytes against independent source, declaration, definition and clock selections,
then passes the constructor-private admitted view and the matching S3 checked
package to the checked temporal adapter. TC-138 remains the historical emitted
artifact suite; this test case names the checked successor and its strict
admission inputs.
The checked progress case also verifies FR-044-AC-6/9 capture restoration,
including separation by checked clause identity under the same admitted clock
and typed incompleteness after capture eviction.

## Test Procedure

Run the following named checks in the integration suite:

1. `checked_successor_authenticates_each_emitted_profile_and_clock_before_evaluation`
   produces a real `/2` artifact for event-position, fixed-sample and
   timestamped-event declarations, reads the producer's canonical bytes with
   `v2::read` using independently derived expectations, compiles each exact
   source into an S1–S4 checked package, and evaluates the checked clause using
   the strict admitted package. For each profile, assert the retained
   definition identity/revision, exact selected definition ArtifactRef,
   declaration index, clock name and full parameter map. Also exercise
   `evaluate_checked_with_progress_v2` and `mapping_support_checked_v2` with
   the same admitted artifact and check their retained authentication evidence.
   Substitute a wrong profile, clock name, missing parameter key, extra
   parameter key, each event/fixed-sample/timestamped clock field, and require
   each exact typed binding refusal with zero position work and no
   authenticated result despite the position-dependent formula. Submit those
   same invalid traces to the checked progress entry point with a higher
   watermark, then submit a matching trace below the previously admitted
   watermark and require the retained watermark contradiction. Independently
   substitute source authority, source identity, declaration name with the
   same span, and byte span with the same source identity; each must return a
   typed reference refusal before work with no authenticated result.
   Exercise a capture-activating clause through the checked progress adapter:
   captures evaluate once and restore across admitted re-evaluation; a refused
   profile does not replace retained progress or capture state; a second
   checked clause with the same declaration index, source identity, admitted
   definition and clock but a different checked node retains separate progress
   and capture values. Mark the retained capture evicted and require the typed
   capture-incomplete result while later wrong-type input cannot re-evaluate
   the initializer.
2. `producer_requires_one_exact_source_definition_and_clock_selection`,
   `strict_reader_rejects_structural_and_identity_substitutions`, and
   `headers_indices_and_each_clock_member_have_stable_v2_refusals` exercise
   producer, strict-reader and typed refusal axes from actual emitter output.
   Preserve the independently authored refusal code for each mutation and
   assert no partial admitted package. The checked adapter is invoked only
   after strict admission succeeds; malformed selections stop at admission.
3. `added_v2_work_is_exactly_bounded_and_a_fresh_retry_is_reproducible`
   measures native producer and strict-reader work, succeeds at exact bounds,
   refuses at one-short bounds for every exposed non-zero dimension (including
   entries, string/number content and canonical output bytes), and retries
   each path with a fresh invocation to reproduce the same bytes and admitted
   bindings; compare the producer's full usage vector across retries. Use the
   admitted retry result in the checked adapter for a matching selection.
4. `native_v2_emission_reaches_the_strict_reader_and_authenticated_l5_adapter`
   and the new checked-successor test above exercise strict `/1` and `/2`
   separation. Preserve the frozen `/1` bytes and require both cross-version
   reader directions to refuse without inserting or removing a temporal
   selection.
5. Under `handoff-writer`, run
   `written_handoff_interchange_records_are_complete`,
   `write_v2_handoff_admits_from_its_files_through_the_strict_v2_reader`, and
   `write_v2_is_deterministic_across_runs`, and
   `write_refuses_an_existing_directory`. Resolve filenames and the mutation
   format only through the public constants, read the writer's actual offer
   bytes from disk, strictly admit them against the written independent
   selection and dependency bytes, replay the manifest's recorded refusal,
   compare two written trees byte-for-byte, and verify refusal to overwrite an
   existing directory leaves its sentinel and contents unchanged.

For any positive checked evaluation, use the artifact's exact source locus,
declaration name, span, profile and clock name. The checked formula and
activation come from S3; the admitted artifact contributes only its
authenticated definition and clock configuration. No test supplies a caller
created truth, selection table, outcome or charge report to the checked route.

## Expected Results

The checked adapter accepts a clause only when its exact source authority and
identity, declaration name and byte span match one strictly admitted `/2`
selection. Its checked profile and clock name agree with that selection, and
its trace profile, clock name and complete parameter map are authenticated
before positions are visited. The result retains the exact admitted definition
and clock binding. Any mismatch is a typed refusal with no position work and no
authenticated result. The existing `/1` signatures and bytes remain unchanged.
