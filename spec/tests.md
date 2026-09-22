---
id: TM-001
title: "Rust fixture audit test matrix"
type: TestMatrix
---

## Overview

Scoped to FR-012/NFR-005 and IT-004. Status: locally verified after
implementation. This matrix does not assert coverage of the earlier
compiler/evaluator scope. US-004/StR-001 are the driving lineage; their full
operational validation remains outside this audit-only plan.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-012 | FR-012-AC-1 | TC-001 | ✅ Passed locally |
| FR-012 | FR-012-AC-2 | TC-002 | ✅ Passed locally |
| FR-012 | FR-012-AC-3 | TC-003 | ✅ Passed locally |
| FR-012 | FR-012-AC-5 | TC-005 | ✅ Passed locally |
| FR-012 | FR-012-AC-6 | TC-006 | ✅ Passed locally |
| FR-012 | FR-012-AC-7 | TC-007 | ✅ Passed locally |
| FR-012 | FR-012-AC-8 | TC-005 | ✅ Passed locally |
| FR-012 | FR-012-AC-9 | TC-008 | ✅ Passed locally |
| FR-012 | FR-012-AC-10 | TC-009 | ✅ Passed locally |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-001 | Self-contained identity negative controls | Unit | P1 | FR-012-AC-1 | ✅ Passed locally |
| TC-002 | Selected invocation packet audit | Integration | P1 | FR-012-AC-2 | ✅ Passed locally |
| TC-003 | Selected role and source correspondence audit | Integration | P1 | FR-012-AC-3 | ✅ Passed locally |
| TC-005 | Real native rule syntax outcomes | Integration | P1 | FR-012-AC-5, FR-012-AC-8 | ✅ Passed locally |
| TC-006 | Malformed JSON and field types | Property | P1 | FR-012-AC-6 | ✅ Passed locally |
| TC-007 | Fixture root containment | Property | P1 | FR-012-AC-7 | ✅ Passed locally |
| TC-008 | Audit resource ceilings | Property | P1 | FR-012-AC-9 | ✅ Passed locally |
| TC-009 | CLI encoding and Rust-only execution | E2E | P1 | FR-012-AC-10, NFR-005-M-2 | ✅ Passed locally |
| TC-010 | Owned verification language inventory | Manual | P1 | NFR-005-M-1 | ✅ Inspected locally |
| TC-156 | Report FB-05 and FB-11 violations over the four-repository backend dependency graph | Integration | P1 | FR-059-AC-1..FR-059-AC-7 | ✅ Passed locally |
| TC-157 | Report pending, passing and failing T-12 API-surface rules | Integration | P1 | FR-060-AC-1..FR-060-AC-4 | ✅ Passed locally |
| TC-158 | Report a quire-ecosystem crate resolved to more than one source | Integration | P1 | FR-061-AC-1..FR-061-AC-4 | ✅ Passed locally |
| TC-159 | Run the current-head integration lane against real and intentionally incompatible heads | Manual | P1 | FR-058-AC-1..FR-058-AC-4 | ✅ Passed locally |
| TC-160 | Every family implements the six-part checked contract with no bypass | Unit | P1 | FR-062-AC-1..FR-062-AC-7, FR-062-AC-9 | 🚧 Planned; #214 |
| TC-161 | The seam probe demonstrates exhaustiveness at every S1-S4 seam | Integration | P1 | FR-063-AC-1..FR-063-AC-7, FR-062-AC-8, FR-067-AC-4 | 🚧 Planned; #214 |
| TC-162 | The string-edge scan reports every unmarked string dispatch | Integration | P1 | FR-064-AC-1..FR-064-AC-6 | 🚧 Planned; #214 |
| TC-163 | Function identity and provenance survive checking and package conversion | Integration | P1 | FR-065-AC-1..FR-065-AC-4 | 🚧 Planned; #214 |
| TC-164 | The composed function checker is deleted in the same change as the S3 function checker | Unit | P1 | FR-065-AC-5 | 🚧 Planned; #214 |
| TC-165 | The migration recipe names every required test, conversion, removal condition and remaining family | Manual | P1 | FR-066-AC-1..FR-066-AC-4 | 🚧 Planned; #214 |
| TC-166 | The replay executor selects a function by typed QualifiedName, never by string | Unit | P1 | FR-062-AC-10, FR-065-AC-6 | 🚧 Planned; #214 |
| TC-167 | The S2 forms stage refuses on a recovering CST and mints no identity | Unit | P1 | FR-067-AC-1, FR-067-AC-2, FR-067-AC-3, FR-067-AC-7, FR-067-AC-8 | ✅ Passed locally |
| TC-168 | SEAM-5 (LoweredSourceGraph) is deleted in the same change as the S2 forms core | Integration | P1 | FR-067-AC-5, FR-067-AC-6 | ✅ Passed locally |
| TC-169 | value::expression::syntax moves to forms with no duplicate definition | Unit | P1 | FR-067-AC-9, FR-067-CON-3 | ✅ Passed locally |
| TC-170 | check-stage modules move to check with no duplication | Unit | P1 | FR-068-AC-1, FR-068-CON-1, FR-068-CON-3 | ✅ Passed locally |
| TC-171 | CheckedPackage/CheckedExpression/CheckedFunction constructors are private to check | Manual | P1 | FR-068-AC-2 | ✅ Inspected locally |
| TC-172 | check's real import graph has no edge into value::expression or into checking, and its re-export back is bounded | Integration | P1 | FR-068-AC-3, FR-068-AC-10 | ✅ Passed locally |
| TC-173 | Refusal split: check causes in check, InputRefusal in value::expression | Unit | P1 | FR-068-AC-4, FR-068-AC-8 | ✅ Passed locally |
| TC-174 | Checking and evaluation produce identical results before and after the split | Integration | P1 | FR-068-AC-5 | ✅ Passed locally |
| TC-175 | The move stays inside M-5: no early M-2 work, no edge widening | Integration | P1 | FR-068-AC-6, FR-068-AC-7 | ⚠️ Partially retired — FR-068-AC-6 steps ✅ Passed locally; FR-068-AC-7 steps retired by FR-074, superseded by TC-261 |
| TC-176 | The interim `model` -> `check` edge stays bounded to two files and thirteen names, imported directly | Unit | P1 | FR-068-AC-9, FR-068-CON-5 | ❌ Retired by FR-074, superseded by TC-262 |
| TC-193 | Candidate set matches registered backends advertising the requested kind | Unit | P1 | FR-075-AC-1, FR-075-AC-5 | 🚧 Planned; #185 |
| TC-194 | Registry candidate sets are invariant under registration-order permutation | Property | P1 | FR-075-AC-2, FR-080-AC-1 | 🚧 Planned; #185 |
| TC-195 | An unregistered named backend yields a distinct unknown-backend marker | Unit | P1 | FR-075-AC-3 | 🚧 Planned; #185 |
| TC-196 | A duplicate backend identity registration is refused and the original stands | Unit | P1 | FR-075-AC-4 | 🚧 Planned; #185 |
| TC-197 | Empty candidate set carries the data an unsupported warning needs | Unit | P1 | FR-076-AC-1, FR-076-AC-2 | 🚧 Planned; #185 |
| TC-198 | Backend absence never settles as a refusal or a hold at the registry | Unit | P1 | FR-076-AC-3 | 🚧 Planned; #185 |
| TC-199 | requests::report takes no Backend parameter and has no capability/family disposition | Unit | P1 | FR-077-AC-1, FR-077-AC-2 | 🚧 Planned; #185 |
| TC-200 | Requests are still recorded as data after negotiation removal | Unit | P1 | FR-077-AC-3 | 🚧 Planned; #185 |
| TC-201 | value::ieee and value::division carry no negotiate_* function | Unit | P1 | FR-078-AC-1, FR-078-AC-2 | ✅ Passed locally; QSL-131, `cargo test --doc -p quire-spec-language` |
| TC-202 | value::ieee and value::division evaluation is unchanged by negotiate_* removal | Unit | P1 | FR-078-AC-3 | ✅ Passed locally; QSL-131, `tests/ieee_profiles.rs` + `tests/integer_division.rs` |
| TC-203 | Existing Kani lowering corpus output is byte-identical before and after the registry swap | Integration | P1 | FR-079-AC-1 | 🚧 Planned; #185 |
| TC-204 | The three legacy lowering target names resolve identically through the registry | Unit | P1 | FR-079-AC-2 | 🚧 Planned; #185 |
| TC-205 | cargo-deny denies inventory, linkme and ctor | Integration | P1 | FR-080-AC-2 | 🚧 Planned; #185 |
| TC-206 | The registry module lint gate finds no static, OnceLock or thread_local | Integration | P1 | FR-080-AC-3 | 🚧 Planned; #185 |
| TC-207 | One unit test exists and passes per ADR-012 §5.2 row | Unit | P1 | FR-080-AC-4 | 🚧 Planned; #185 |
| TC-208 | The S7 seam probe fails to compile the registry arm on an unhandled capability-kind variant | Integration | P1 | FR-080-AC-5 | 🚧 Planned; #185 |
| TC-177 | The proof-result envelope maps every FR-331 outcome to its exact O-16 category | Property | P1 | FR-069-AC-1 | ✅ Passed locally |
| TC-178 | The proof-result reader refuses an unknown version, vocabulary, or oversized envelope before consumption | Unit | P1 | FR-069-AC-2, FR-069-AC-4 | ✅ Passed locally |
| TC-179 | A positive proof-result envelope round-trips its backend identity, tool pin and dispositions exactly | Unit | P1 | FR-069-AC-3 | ✅ Passed locally |
| TC-180 | The witness envelope stores the transcript once and derives every other fact from it | Unit | P1 | FR-070-AC-1 | ✅ Passed locally |
| TC-181 | The witness envelope refuses a malformed transcript, an out-of-domain digest, or an oversized encoding | Property | P1 | FR-070-AC-2, FR-070-AC-6, FR-070-AC-7 | ✅ Passed locally |
| TC-182 | A positive witness envelope round-trips its transcript and every O-25 member exactly | Unit | P1 | FR-070-AC-3 | ✅ Passed locally |
| TC-183 | The witness envelope refuses reconstruction when any one O-25 member is missing | Property | P1 | FR-070-AC-4 | ✅ Passed locally |
| TC-184 | A family adds a typed witness payload through a typed extension point, not an untyped map | Unit | P1 | FR-070-AC-5 | ✅ Passed locally |
| TC-185 | The replay request carries exactly the O-26 members and round-trips them exactly | Unit | P1 | FR-071-AC-1 | ✅ Passed locally |
| TC-186 | The replay request's byte provision is reachable only by digest, never by path, is complete, and stays within the size bound | Property | P1 | FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7 | ✅ Passed locally |
| TC-187 | The replay request's function selection accepts only a typed QualifiedName, never a bare string | Unit | P1 | FR-071-AC-3 | 🚧 Planned; #231 |
| TC-188 | The replay request refuses an unknown version or an out-of-set profile/capability identifier before recompilation | Unit | P1 | FR-071-AC-4 | ✅ Passed locally |
| TC-189 | The replay result keeps the Witness arm and Input arm distinct, each with its own settlement | Unit | P1 | FR-072-AC-1 | ✅ Passed locally |
| TC-190 | A replay disagreement settles inconclusive with a typed cause and is never repairable | Unit | P1 | FR-072-AC-2 | ✅ Passed locally |
| TC-191 | A replay result's nested witness record round-trips exactly, compares without display-text interpretation, and refuses an oversized encoding | Unit | P1 | FR-072-AC-3, FR-072-AC-5 | ✅ Passed locally |
| TC-192 | #217's function exemplar builds on the existing result/request/witness types with no new type | Integration | P1 | FR-072-AC-4 | 🚧 Planned; #217 |
| TC-209 | The witness envelope's Debug and Display rendering never reproduces the full transcript | Unit | P1 | FR-073-AC-1 | ✅ Passed locally |
| TC-210 | The replay request's Debug and Display rendering never reproduces a byte-provision entry's raw bytes | Unit | P1 | FR-073-AC-2 | ✅ Passed locally |
| TC-211 | A refusal cause from any of the four envelopes renders with no unredacted transcript, byte or value content, while the typed accessor stays fully readable | Unit | P1 | FR-073-AC-3 | ✅ Passed locally |
| TC-213 | Original declaration keys survive normalization unchanged | Unit | P1 | FR-081-AC-1 | ✅ Passed locally |
| TC-214 | Structurally identical declarations from distinct originals never collapse to one effective identity | Unit | P1 | FR-081-AC-2 | 🚧 Planned; #120 |
| TC-215 | A dominated redefinition is retained for provenance, not deleted | Unit | P1 | FR-081-AC-3 | ✅ Passed locally |
| TC-216 | Changing only a display name leaves every key, identity and ordering unchanged | Property | P1 | FR-081-AC-4 | 🚧 Planned; #120 |
| TC-217 | The model binder is a pure function with no cross-run ambient state | Unit | P1 | FR-081-AC-5 | 🚧 Planned; #120 |
| TC-218 | Redefinition variance checking reports every failing axis, not only the first | Unit | P1 | FR-082-AC-1 | ✅ Passed locally |
| TC-219 | A missing redefinition target and a supertype cycle each refuse with a named cause | Unit | P1 | FR-082-AC-2 | ✅ Passed locally |
| TC-220 | A conformance ancestor walk past the bound refuses instead of truncating | Unit | P1 | FR-082-AC-3 | 🚧 Planned; #120 |
| TC-221 | A narrowing field redefinition requires an established postcondition | Unit | P1 | FR-082-AC-4 | ✅ Passed locally |
| TC-222 | Dispatch selects the descendant candidate independent of declaration order | Property | P1 | FR-083-AC-1 | ✅ Passed locally |
| TC-223 | No-applicable-candidate and multiple-undominated-candidates are named separately | Unit | P1 | FR-083-AC-2 | ✅ Passed locally |
| TC-224 | An ambiguous family produces no dispatch table, not even for subtypes that resolved cleanly | Unit | P1 | FR-083-AC-3 | ✅ Passed locally |
| TC-225 | A dispatch family deeper than the bound refuses instead of reporting a false unique winner | Unit | P1 | FR-083-AC-4 | 🚧 Planned; #120 |
| TC-226 | Population admission distinguishes unknown closure from a genuine refusal | Unit | P1 | FR-084-AC-1 | ✅ Passed locally |
| TC-227 | allInstances requires both object and subtype closure, never a partial set | Unit | P1 | FR-084-AC-2 | 🚧 Planned; #120 |
| TC-228 | lookup returns the declared absence mode for a genuinely unmatched key | Unit | P1 | FR-084-AC-3 | ✅ Passed locally |
| TC-229 | Two members sharing universe and object identity but differing type refuse admission outright | Unit | P1 | FR-084-AC-4 | ✅ Passed locally |
| TC-230 | A relationship end naming an undeclared type refuses the whole relationship record | Unit | P1 | FR-085-AC-1 | 🚧 Planned; #120 |
| TC-231 | A resolved relationship end retains its declared role and multiplicity exactly | Unit | P1 | FR-085-AC-2 | 🚧 Planned; #120 |
| TC-232 | A relationship between two object types carries no systems-model kind | Unit | P1 | FR-085-AC-3 | 🚧 Planned; #120 |
| TC-233 | Systems classification reports every no-kind cascade, not only the first | Unit | P1 | FR-086-AC-1 | 🚧 Planned; #120 |
| TC-234 | Connection admission checks all three conditions independently and reports every failure | Unit | P1 | FR-086-AC-2 | 🚧 Planned; #120 |
| TC-235 | An allocation whose target does not classify as Part refuses wrong-export | Unit | P1 | FR-086-AC-3 | ✅ Passed locally |
| TC-236 | Two declarations sharing a display title classify and resolve independently by key | Unit | P1 | FR-086-AC-4 | 🚧 Planned; #120 |
| TC-237 | A derivation conflict with no descendant redefiner exposes no effective member for either path | Unit | P1 | FR-081-AC-6 | ✅ Passed locally |
| TC-238 | Replaying normalization over a permuted IR node order reproduces the same correspondence | Property | P1 | FR-081-AC-7 | ✅ Passed locally |
| TC-239 | An arity mismatch refuses without checking per-parameter axes, while result and effect axes are still checked | Unit | P1 | FR-082-AC-5 | ✅ Passed locally |
| TC-240 | allInstances and lookup return the FR-153 typed result shape and its bound/foreign/ineligible refusals | Unit | P1 | FR-084-AC-5 | 🚧 Planned; #120 |
| TC-241 | Kind mapping runs interfaces first, and a port whose interface type is not an Interface refuses wrong-export | Unit | P1 | FR-086-AC-5 | 🚧 Planned; #120 |
| TC-242 | A selected object's reference key names the same most-specific type through every conforming query | Unit | P1 | FR-084-AC-6 | ✅ Passed locally |
| TC-261 | M-2's items are relocated to check and absent from model | Unit | P1 | FR-074-AC-1, FR-074-AC-2 | ✅ Passed locally |
| TC-262 | The model -> check edge is fully closed after M-2 | Unit | P1 | FR-074-AC-3 | ✅ Passed locally |

## Typed replay envelopes (FR-069–073) coverage

[FR-069](../spec/functional/FR-069-implement-typed-proof-result-envelope.md)
through [FR-073](../spec/functional/FR-073-implement-redacted-safe-diagnostic-rendering.md)
are implemented under issue #231, in `src/replay/{bounds,identity,proof_result,
witness,request,result}.rs`. Every TC below was red/green falsified during
implementation: the targeted behavior was independently removed (one code
mutation per test — e.g. the vacuous-proof category branch, the
`contract_version` refusal, an `Option`-member's `?` refusal, the redacted
`Debug` impl), the suite was run once and only the intended test(s) failed,
the mutation was reverted, and the suite was confirmed green again. This was
done as one batched mutation-and-revert pair (not one recompile per test) to
respect this session's shared build-lock load; ten of the nineteen TC rows —
TC-177, 178, 179, 183, 185, 186, 189, 190, 191, 209 — were each independently
mutated this way. The remaining nine (TC-180, 181, 182, 184, 187, 188, 192,
210, 211) were not independently mutated but were read in full and share the
identical code shape (the same `.ok_or(Refusal::X)?` early-refusal pattern,
the same round-trip-equality pattern, or the same redacted-`Debug` pattern)
as a row that was.

- TC-177 (FR-069-AC-1): `src/replay/proof_result.rs::tests::tc_177_every_fr331_value_maps_to_its_exact_category`
- TC-178 (FR-069-AC-2, FR-069-AC-4): `src/replay/proof_result.rs::tests::tc_178_refuses_unknown_version_vocabulary_or_oversized_envelope`
- TC-179 (FR-069-AC-3): `src/replay/proof_result.rs::tests::tc_179_round_trip_preserves_backend_tool_pin_and_dispositions`
- TC-180 (FR-070-AC-1): `src/replay/witness.rs::witness_tests::tc_180_exactly_one_field_and_derived_facts_track_the_stored_transcript`
- TC-181 (FR-070-AC-2, FR-070-AC-6, FR-070-AC-7): `src/replay/witness.rs::witness_tests::tc_181_refuses_malformed_transcripts`, `::envelope_tests::tc_181_refuses_an_out_of_domain_digest`, `::envelope_tests::tc_181_refuses_an_oversized_encoding`
- TC-182 (FR-070-AC-3): `src/replay/witness.rs::envelope_tests::tc_182_round_trip_preserves_every_o25_member_and_the_transcript`
- TC-183 (FR-070-AC-4): `src/replay/witness.rs::envelope_tests::tc_183_refuses_reconstruction_when_any_o25_member_is_missing` — caveat: this is a `Property`-typed row, but the test asserts only four of the roughly thirteen O-25 members individually (`backend`, `trace_position`, `source_digests`, `obligation_identity`); the rest share the identical `.ok_or(WitnessRefusal::MissingMember(...))?` pattern but are not each individually exercised.
- TC-184 (FR-070-AC-5): `src/replay/witness.rs::envelope_tests::tc_184_family_payload_is_a_typed_extension_point` — caveat: the "typed extension point" half is asserted by attaching and round-tripping a new payload type; the "not an untyped map" half is a source-inspection fact (no `get_extra`/string-keyed accessor exists on `WitnessEnvelope`), not itself a runtime assertion.
- TC-185 (FR-071-AC-1): `src/replay/request.rs::tests::tc_185_carries_exactly_o26_members_and_round_trips`
- TC-186 (FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7): `src/replay/request.rs::tests::tc_186_byte_provision_is_digest_only_complete_and_bounded`
- TC-188 (FR-071-AC-4): `src/replay/request.rs::tests::tc_188_refuses_unknown_version_or_profile_before_recompilation`
- TC-189 (FR-072-AC-1): `src/replay/result.rs::tests::tc_189_witness_and_input_arms_stay_distinct`
- TC-190 (FR-072-AC-2): `src/replay/result.rs::tests::tc_190_disagreement_settles_inconclusive_and_is_never_repaired`
- TC-191 (FR-072-AC-3, FR-072-AC-5): `src/replay/result.rs::tests::tc_191_round_trips_the_fr351_record_and_compares_structurally`
- TC-209 (FR-073-AC-1): `src/replay/witness.rs::witness_tests::tc_209_debug_and_display_never_reproduce_the_full_transcript`
- TC-210 (FR-073-AC-2): `src/replay/request.rs::tests::tc_210_debug_never_reproduces_byte_provision_raw_bytes`
- TC-211 (FR-073-AC-3): `src/replay/mod.rs::redaction_tests::tc_211_refusal_causes_redact_while_typed_accessors_stay_readable`

TC-187 stays `🚧 Planned; #231` and TC-192 is now attributed
`🚧 Planned; #217`: both rows' core claim is an absence of something (no
bare-`&str`/`String` entry point exists anywhere on `ReplayRequest`'s public
API for TC-187; no fifth type is defined for TC-192's exemplar) that only a
source-level inspection can establish, not a runtime assertion inside the
test itself, so neither test carries a `#[trace]` tag naming the AC it
cannot fail on. The positive half of each (a multi-segment name round-trips
its segments; two structurally different functions reuse the four #231
types) is exercised by
`src/replay/request.rs::tests::tc_187_selection_is_always_a_typed_qualified_name`
and `src/replay/result.rs::tests::tc_192_function_exemplar_reuses_the_four_types_with_none_new`
respectively, but the row's literal claim is broader than what either test
asserts at runtime. TC-192's row is attributed to #217 rather than #231
because its claim ("no fifth type is defined in #217's repository scope") is
a property of #217's own, separate repository -- nothing in this repo's test
suite can observe another repository's type definitions, so this repo's
suite can never discharge that row regardless of what #231 builds; #231's
own obligation (FR-072-AC-4's "using this type together with FR-070's
witness envelope and FR-071's request type") is what
`tc_192_function_exemplar_reuses_the_four_types_with_none_new` actually
demonstrates.

## Model graph binding (FR-081–086) retrospective coverage

[FR-081](../spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md)
through [FR-086](../spec/functional/FR-086-bind-systems-model-references.md)
retrospectively scope model-binder behavior under issue #120: the model
normalization, conformance, dispatch, population and systems-classification
code these requirements bind, and most of the Rust tests exercising it,
predate this specification slice (from #131 domain-package intake and the
FR-150–153 binding work it carries). Sixteen of the thirty TC-213–242 rows
above cite real, currently passing tests rather than planned work, naming
the exact backing test:

- TC-213 (FR-081-AC-1): `tests/model_normalization.rs::n01_normalizes_f1_to_the_exact_ground_truth_identities`
- TC-215 (FR-081-AC-3): `tests/model_normalization.rs::n06_a_strictly_more_derived_redefiner_resolves_the_conflict_and_hides_every_contender`
- TC-218 (FR-082-AC-1): `tests/model_conformance.rs::r03_an_incompatible_operation_redefinition_reports_every_failing_axis`
- TC-219 (FR-082-AC-2): `tests/model_conformance.rs::r07_zero_inherited_targets_refuses_redefinition_target` and `tests/model_normalization.rs::r01_a_closing_generalization_cycle_names_the_full_rotated_chain`
- TC-221 (FR-082-AC-4): `tests/model_conformance.rs::r08a_*_refuses` and `::r08b_*_discharges_the_obligation`
- TC-222 (FR-083-AC-1): `tests/model_dispatch.rs::d04_registration_order_does_not_change_the_linked_table`
- TC-223 (FR-083-AC-2): `tests/model_dispatch.rs::d03_no_candidate_*_no_applicable` and `::d02_an_undominated_multi_way_tie_*`
- TC-224 (FR-083-AC-3): `tests/model_dispatch.rs::d02_an_undominated_multi_way_tie_*` (the same `Ambiguous` outcome carries no table for the family's other, cleanly-resolving subtype)
- TC-226 (FR-084-AC-1): `tests/model_population.rs::l02_unknown_closure_is_incomplete_not_refused` (both halves: `open` extent and an unclosed generalization graph) and `::l05_foreign_type_refuses`
- TC-228 (FR-084-AC-3): `tests/model_population.rs::l03_lookup_undefined_mode`, `::l03_lookup_empty_mode`, `::l03_lookup_refused_mode`
- TC-229 (FR-084-AC-4): `tests/model_population.rs::l05_conflicting_identity_refuses_after_fourth_member_charge` and `::l05_duplicate_collapses_and_recovers_l01`
- TC-235 (FR-086-AC-3): `tests/model_systems.rs::y02_wrong_export_substitutions_name_the_required_and_actual_kind`
- TC-237 (FR-081-AC-6): `tests/model_normalization.rs::n06_two_undominated_redefiners_of_the_same_target_refuse_as_a_conflict`
- TC-238 (FR-081-AC-7): `tests/model_normalization.rs::n07_record_order_does_not_affect_identity_or_view` (asserts the whole view's identity digest is order-independent; does not separately assert a per-entry iteration sequence — see the TC's own caveat)
- TC-239 (FR-082-AC-5, Part A — the arity carve-out itself): `tests/model_conformance.rs::r04_an_arity_mismatch_refuses_without_checking_parameter_axes`. Part B (a combined arity-and-result-multiplicity failure) is not backed by any existing test; see the TC's own gap note.
- TC-242 (FR-084-AC-6): `tests/model_population.rs::l01_all_instances_selects_subtype_population_once`, which already carries the quire-specification `FR-153-AC-6` trace tag and asserts this exact guarantee (`b1_via_a == b1_via_b`)

TC-234 (FR-086-AC-2) was corrected back from a prior, mistaken "Passed
locally" this revision: `check_connection` really is exhaustive today, but
no existing test constructs the combined port-direction-and-multiplicity
fixture FR-086-AC-2 states — every one of `tests/model_systems.rs::y03a`
through `::y05` violates exactly one condition. See FR-086's Dependencies
note.

The remaining rows (TC-214, TC-216, TC-217, TC-220, TC-225, TC-227, TC-230
through TC-234, TC-236, TC-240 and TC-241) name real behavior with no
existing test asserting their exact content — `title`/`displayName`
independence, cross-process purity, the `MAX_CONFORMANCE_DEPTH`/
`MAX_DISPATCH_DEPTH` walk bounds, dangling relationship ends, the
combined-condition Connection fixture, and the newly owned FR-084/FR-086
acceptance criteria this PR adds — and stay `🚧 Planned; #120` honestly.
This is a retrospective spec in the same sense
[FR-047](../spec/functional/FR-047-evaluate-finite-object-reference-graphs.md)
already is for this repo: it states the coverage that exists rather than
treating the whole slice as unbuilt.

## Six coverage rules

All nine FR-012 ACs and both NFR-005 metric obligations are verified above.
Modes are mutually exclusive, not combinable options. Valid/unknown modes,
missing/extra arguments, contained/foreign paths, zero/exact/over resource
limits, malformed fields and identity changes are covered. No asynchronous
state machine is introduced; the identity registry's first binding/repeated
binding/conflicting binding transitions have explicit controls. The generated
input rows require actual generation and retained failures, not a Property
label on one example. The implemented Property cases enumerate bounded JSON,
field, path and budget families deterministically; no randomized campaign or
fuzzing result is claimed. Independent fixture mutations reach real audit code.

## Integration Test Matrix

IT-004 is a command/file boundary, not a service/browser/event/database system.
TC-002, TC-003 and TC-005 execute real filesystem/parser integration, and TC-009
executes the compiled binary. The private standard packet remains an explicit
local lane; the default local suite uses self-contained negative controls. No substitute network/daemon classification is invented.

## Coverage gaps

The preexisting 21 Rust tests and older FR/NFR obligations still need their own
formal TC/evidence remediation. TC-010's inspection is recorded in
[the remediation inventory](../docs/rust-verification-remediation.md); the module
classifies Manual as no_source_symbol. Hosted CI is manual-dispatch only.

## Execution record

The audit unit and default audit integration tests pass locally. All three named
private-packet tests were explicitly executed against specification revision
36293bae7f5bcb7ca3b2389ed166e525dc9dba87 and passed. Quire resolves every audit
test symbol, every FR-012 AC and every executable TC. TC-010 has manual evidence. These counts establish
the selected scope; they are not full compiler or semantic qualification.
