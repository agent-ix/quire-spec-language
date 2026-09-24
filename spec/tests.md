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
| TC-157 | Report pending, passing and failing T-12 API-surface rules | Integration | P1 | FR-060-AC-1..FR-060-AC-4 | ✅ Passed locally for FR-060-AC-1..AC-3; 🚧 FR-060-AC-4's T12-B and T12-C clauses amended to allow-lists plus debt lists (2026-09-22), gate rewrite planned |
| TC-158 | Report a quire-ecosystem crate resolved to more than one source | Integration | P1 | FR-061-AC-1..FR-061-AC-4 | ✅ Passed locally |
| TC-159 | Run the current-head integration lane against real and intentionally incompatible heads | Manual | P1 | FR-058-AC-1..FR-058-AC-4 | ✅ Passed locally |
| TC-160 | Every family implements the six-part checked contract with no bypass | Unit | P1 | FR-062-AC-1..FR-062-AC-7, FR-062-AC-9 | 🚧 Planned; #214 |
| TC-161 | The seam probe demonstrates exhaustiveness at every S1-S4 seam | Integration | P1 | FR-063-AC-1..FR-063-AC-7, FR-062-AC-8, FR-067-AC-4 | 🚧 Planned; #214 |
| TC-162 | The string-edge scan reports every unmarked string dispatch | Integration | P1 | FR-064-AC-1..FR-064-AC-6 | 🚧 Planned; #214 |
| TC-163 | Function identity and provenance survive checking and package conversion | Integration | P1 | FR-065-AC-1..FR-065-AC-4, FR-065-AC-8 | 🚧 Planned; #214, QSL-156 |
| TC-164 | The composed function checker is deleted in the same change as the S3 function checker | Unit | P1 | FR-065-AC-5 | 🚧 Planned; #214 |
| TC-165 | The migration recipe names every required test, conversion, removal condition and remaining family | Manual | P1 | FR-066-AC-1..FR-066-AC-4 | 🚧 Planned; #214 |
| TC-166 | The replay executor selects a function by typed QualifiedName, never by string | Unit | P1 | FR-062-AC-10, FR-065-AC-6 | 🚧 Planned; #214 |
| TC-167 | The S2 forms stage refuses on a recovering CST and mints no identity | Unit | P1 | FR-067-AC-1, FR-067-AC-2, FR-067-AC-3, FR-067-AC-7, FR-067-AC-8 | ✅ Passed locally |
| TC-168 | SEAM-5 (LoweredSourceGraph) is deleted in the same change as the S2 forms core | Integration | P1 | FR-067-AC-5, FR-067-AC-6 | ✅ Passed locally |
| TC-169 | value::expression::syntax moves to forms with no duplicate definition | Unit | P1 | FR-067-AC-9, FR-067-CON-3 | ✅ Passed locally |
| TC-170 | check-stage modules move to check with no duplication | Unit | P1 | FR-068-AC-1, FR-068-CON-1, FR-068-CON-3 | ✅ Passed locally |
| TC-171 | CheckedPackage/CheckedExpression/CheckedFunction constructors are private to check | Manual | P1 | FR-068-AC-2 | ✅ Inspected locally |
| TC-172 | check's real import graph has no edge into value::expression or into checking, and value::expression re-exports no check item | Integration | P1 | FR-068-AC-3, FR-068-AC-10 (retired) | ✅ Passed locally; steps 1-5 retired by QSL-182, enforced by Cargo's `qsl-semantics` → root-crate refusal and TC-390 (`no_crate_below_layer_three_depends_on_the_check_core`); step 6 by `tests/it/layer_crate_reexports.rs` |
| TC-173 | Refusal split: check causes in check, InputRefusal in value::expression | Unit | P1 | FR-068-AC-4, FR-068-AC-8 | ✅ Passed locally |
| TC-174 | Checking and evaluation produce identical results before and after the split | Integration | P1 | FR-068-AC-5 | ✅ Passed locally |
| TC-175 | The move stays inside M-5: no early M-2 work, no edge widening | Integration | P1 | FR-068-AC-6, FR-068-AC-7 | 🚧 FR-068-AC-6 steps amended to the module-level layer rule (2026-09-22), gate rewrite planned; FR-068-AC-7 steps retired by FR-074, superseded by TC-261 |
| TC-176 | The interim `model` -> `check` edge stays bounded to two files and thirteen names, imported directly | Unit | P1 | FR-068-AC-9, FR-068-CON-5 | ❌ Retired by FR-074, superseded by TC-262 |
| TC-193 | Candidate set matches registered backends advertising the requested kind | Unit | P1 | FR-075-AC-1, FR-075-AC-5 | ✅ Passed locally; QSL-46 (PR #305); AC-5's no-second-enum half by inspection |
| TC-194 | Registry candidate sets are invariant under registration-order permutation | Property | P1 | FR-075-AC-2, FR-080-AC-1 | ✅ Passed locally; QSL-46 (PR #305) |
| TC-195 | An unregistered named backend yields a distinct unknown-backend marker | Unit | P1 | FR-075-AC-3 | ✅ Passed locally; QSL-46 (PR #305) |
| TC-196 | A duplicate backend identity registration is refused and the original stands | Unit | P1 | FR-075-AC-4 | ✅ Passed locally; QSL-46 (PR #305) |
| TC-197 | Empty candidate set carries the data an unsupported warning needs | Unit | P1 | FR-076-AC-1, FR-076-AC-2 | ✅ Passed locally; QSL-46 (PR #305), `qsl-route/tests/it/route_registry.rs` |
| TC-198 | Backend absence never settles as a refusal or a hold at the registry | Unit | P1 | FR-076-AC-3 | ✅ Passed locally; QSL-46 (PR #305), `qsl-route/tests/it/route_registry.rs` |
| TC-199 | requests::report takes no Backend parameter and has no capability/family disposition | Unit | P1 | FR-077-AC-1, FR-077-AC-2 | ✅ Passed locally; QSL-46 (PR #305), `compile_fail` doctests traced on `FR077Doctests` (`src/linking/composed/requests.rs`) |
| TC-200 | Requests are still recorded as data after negotiation removal | Unit | P1 | FR-077-AC-3 | ✅ Passed locally; QSL-46 (PR #305), `tests/it/composed_admission_stages.rs` |
| TC-201 | value::ieee and value::division carry no negotiate_* function | Unit | P1 | FR-078-AC-1, FR-078-AC-2 | ✅ Passed locally; QSL-131, `cargo test --doc -p quire-spec-language` |
| TC-202 | value::ieee and value::division evaluation is unchanged by negotiate_* removal | Unit | P1 | FR-078-AC-3 | ✅ Passed locally; QSL-131, `tests/ieee_profiles.rs` + `tests/integer_division.rs` |
| TC-203 | Existing Kani lowering corpus output is byte-identical before and after the registry swap | Integration | P1 | FR-079-AC-1 | 🚧 Planned; retires with `lowering` (ADR-011 §7.3 M-6a) |
| TC-204 | The three legacy lowering target names resolve identically through the registry | Unit | P1 | FR-079-AC-2 | ✅ Passed locally for the current catalog; QSL-46 (PR #305); retires with `lowering` (ADR-011 §7.3 M-6a) |
| TC-205 | cargo-deny denies inventory, linkme and ctor | Integration | P1 | FR-080-AC-2 | ✅ Passed locally with `cargo-deny` installed; QSL-46 (PR #305), `qsl-route/tests/it/route_registry.rs` (the test skips when `cargo-deny` is absent; `make cargo-deny-bans` does not) |
| TC-206 | The registry module lint gate finds no static, OnceLock or thread_local | Integration | P1 | FR-080-AC-3 | ✅ Passed locally; QSL-46 (PR #305), `xtask/src/route_lint.rs` |
| TC-207 | One unit test exists and passes per ADR-012 §5.2 row | Unit | P1 | FR-080-AC-4 | ✅ Passed locally; QSL-46 (PR #305), `qsl-route/tests/it/route_registry.rs` |
| TC-208 | The S7 seam probe fails to compile the registry arm on an unhandled capability-kind variant | Integration | P1 | FR-080-AC-5 | 🚧 No tagged test; `make seam-probe` (part of `make ci`) confirms the S7 location `qsl-route/src/lib.rs::same_kind`; QSL-46 (PR #305) |
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
| TC-220 | A conformance ancestor walk past the bound refuses instead of truncating | Unit | P1 | FR-082-AC-3 | ✅ Passed locally |
| TC-221 | A narrowing field redefinition requires an established postcondition | Unit | P1 | FR-082-AC-4 | ✅ Passed locally |
| TC-222 | Dispatch selects the descendant candidate independent of declaration order | Property | P1 | FR-083-AC-1 | ✅ Passed locally |
| TC-223 | No-applicable-candidate and multiple-undominated-candidates are named separately | Unit | P1 | FR-083-AC-2 | ✅ Passed locally |
| TC-224 | An ambiguous family produces no dispatch table, not even for subtypes that resolved cleanly | Unit | P1 | FR-083-AC-3 | ✅ Passed locally |
| TC-225 | A dispatch family deeper than the bound refuses instead of reporting a false unique winner | Unit | P1 | FR-083-AC-4 | ✅ Passed locally |
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
| TC-243 | Typestate constructors are private to their stage module | Manual | P1 | FR-087-AC-1 | 🚧 Planned; QSL-158 |
| TC-244 | compile_fail matrix over every forbidden typestate construction (R-10, O-15) | Unit | P1 | FR-087-AC-2 | 🚧 Planned; QSL-158 |
| TC-245 | PackageNodeKey has exactly one shape and declared equality | Unit | P1 | FR-087-AC-5 | ✅ Passed locally |
| TC-246 | ResolvedSourcePackage is retired, with no dangling caller | Integration | P1 | FR-087-AC-7, FR-087-CON-4 | 🚧 Planned; QSL-158 |
| TC-247 | The canonical EmittedPackage/CheckedPackage stay distinct from their pre-existing namesakes | Integration | P1 | FR-087-AC-8, FR-087-AC-10 | 🚧 Planned; QSL-158 |
| TC-248 | Frame identity's subject sets resolve to DeclarationKey through the model correspondence | Unit | P1 | FR-088-AC-2 | ✅ Passed locally; #300 |
| TC-249 | Clause identity is the checked node id; the occurrence key disambiguates structurally identical clauses | Unit | P1 | FR-088-AC-3 | ✅ Passed locally; #300 |
| TC-250 | Clause-kind wire-string totality in both directions, with mutation coverage | Property | P1 | FR-088-AC-4 | ✅ Passed locally; #300 |
| TC-251 | Qualified-name resolution is confined to the check stage (R-06) | Integration | P1 | FR-088-AC-5 | ✅ Passed locally; #300 (the `QualifiedName` half only -- AC-5's "or a bare string" half is investigated, not enforced; see `tests/it/name_resolution_confinement.rs`'s own module doc) |
| TC-252 | Checked type node to kernel ValueType is total, tested per type-node form including a sum | Unit | P1 | FR-088-AC-9, FR-088-AC-10 | ✅ Passed locally; #300; step 6's from-source reorder is QSL-131 |
| TC-253 | The verified binding admits VerifiedPackage only under all three conditions, refusing each failure independently | Integration | P1 | FR-087-AC-3 | ✅ Passed locally; #340 (step 3 and steps 5-7 at the `library` level, see FR-087 Status) |
| TC-254 | library converts VerifiedPackage to ImportView without resolving any name | Unit | P1 | FR-087-AC-4 | ✅ Passed locally (steps 1-2: the view keyed by `WireNodeId`; steps 3-5: the `library` signature scan and `check::imports`, see FR-087 Status) |
| TC-255 | NodeKey is never minted from a WireNodeId; E4 and E9 resolve by lookup, never by construction | Integration | P1 | FR-087-AC-6 | 🚧 Planned; QSL-158 |
| TC-256 | check and package's dependency edge is one direction, and CheckedPackage wraps CheckedGraph | Integration | P1 | FR-087-AC-9 | ✅ Passed locally; step 3 by `tests/it/layer_crate_reexports.rs`, steps 1 and 6 by Cargo's `qsl-package` → `qsl-semantics` edge and TC-390; steps 2, 4 and 5 by inspection |
| TC-257 | Exactly one closed checked clause-kind enum exists, and syntax::ClauseKind gains no variant | Unit | P1 | FR-088-AC-1 | ✅ Passed locally; #300 |
| TC-258 | QualifiedName is used only as a declared preimage component, never as an identity | Manual | P1 | FR-088-AC-6 | 🚧 Planned; QSL-158 (not in #300's scope) |
| TC-259 | A package type's identity is its checked node id, scoped only by owner | Unit | P1 | FR-088-AC-7 | 🚧 Partial: step 4 (a declared type's node id is its declaration's key) passes locally, #300; steps 1 to 3 (same owner, different owners, builtin or anonymous type) and step 5 (recompilation) planned, QSL-156 |
| TC-260 | ValueTypeRef is exactly the two-member union Native/Package | Manual | P1 | FR-088-AC-8 | 🚧 Planned; QSL-158 (not in #300's scope) |
| TC-261 | M-2's items are relocated to check and absent from model | Unit | P1 | FR-074-AC-1, FR-074-AC-2 | ✅ Passed locally |
| TC-262 | The model -> check edge is fully closed after M-2 | Unit | P1 | FR-074-AC-3 | ✅ Passed locally |
| TC-281 | value::library and value::package_identity relocate into the new top-level library module, per the R-10/T-3 shapes | Integration | P1 | FR-087-AC-11 | 🚧 Planned; QSL-158 |
| TC-282 | Every resolve_libraries refusal classifies to an I2 rule, a §4 condition, E3 resolution, or a named exception | Unit | P1 | FR-087-AC-12 | ✅ Passed locally |
| TC-291 | PopulationId is deterministic over its admission preimage and distinguishes distinct admissions | Unit | P1 | FR-089-AC-1 | ✅ Passed locally; QSL-131 |
| TC-292 | Kernel Value::Population carries PopulationId only, with no model dependency | Manual | P1 | FR-089-AC-2 | ✅ Inspected locally; QSL-131 |
| TC-293 | The evaluator resolves a Value::Population identity through the recorded correspondence, not a carried payload | Unit | P1 | FR-089-AC-3 | ✅ Passed locally; QSL-131 |
| TC-294 | An unresolved PopulationId refuses with a typed cause, not a panic or Undefined | Unit | P1 | FR-089-AC-4 | ✅ Passed locally; QSL-131 |
| TC-295 | The QSL layer admits a Value::Population identity under ValueType::Population by its resolved binding's declared maximum | Unit | P1 | FR-089-AC-5 | ✅ Passed locally; QSL-131 |
| TC-296 | A standalone Direct admission and an invocation's Post binding over the same domain package and population_key mint distinct PopulationIds | Unit | P1 | FR-089-AC-1 | ✅ Passed locally; QSL-131 |
| TC-297 | Kernel admits, plan_pairs and compare_keys refuse a population pair | Unit | P1 | FR-089-AC-6 | ✅ Passed locally; QSL-131 |
| TC-376 | Function application checking accepts a well-typed call and refuses wrong arity, an unknown name and a type mismatch | Unit | P1 | FR-065 | ✅ Passed locally; verifies FR-065's behavior generally, not a specific AC (QSL-148) |
| TC-377 | Function declaration checking accepts a well-typed declaration, reports its calls, and refuses an ill-typed body | Unit | P1 | FR-065 | ✅ Passed locally; verifies FR-065's behavior generally, not a specific AC (QSL-148) |
| TC-378 | A real recursive-descent fixture shows the nesting-depth limit is the proximate cause of a function-declaration refusal | Unit | P1 | FR-062 | 🚧 Planned; QSL-148 |
| TC-379 | E3 resolves an imported name to its PackageNodeKey and refuses a missing or ambiguous one | Unit | P1 | FR-087-AC-13 | 🚧 Planned; QSL-158 (PR #299 review, finding 2) |
| TC-380 | The function-declaration contract check refuses an ill-typed declaration and admits a well-typed one | Unit | P1 | FR-065-AC-7 | ✅ Passed locally (QSL-148, PR #303) |
| TC-381 | The expression-node limit bounds the whole checked package, not each declaration | Unit | P1 | FR-062-AC-11 | ✅ Passed locally (PR #303 review round 3, finding F1) |
| TC-382 | S6a returns each kernel outcome unchanged in FamilyOutcome::Evaluated | Unit | P1 | FR-090-AC-1 | ✅ Passed locally |
| TC-384 | S6a invariant breaks are InternalFaults, not panics or refusals | Unit | P1 | FR-090-AC-3 | ✅ Passed locally |
| TC-385 | S6a's input type admits no Relation, and FamilyOutcome has exactly two arms | Unit | P1 | FR-090-AC-4 | ✅ Passed locally |
| TC-386 | F diagnostic maps every snapshot-cause and model-refusal catalog code to category refusal | Unit | P1 | FR-090-AC-5 | ✅ Passed locally |
| TC-387 | The ProtocolClause snapshot cause maps each WrongSnapshotCause to wrong_snapshot | Unit | P1 | FR-090-AC-6 | ✅ Passed locally |
| TC-388 | An evaluation-time wrong-anchor snapshot reaches the caller as a coded QSL refusal, not a kernel refusal | Integration | P1 | FR-090-AC-7 | ✅ Passed locally |
| TC-389 | A refused model query reaches the caller with the ModelRefusal's own catalog code, not a kernel refusal | Integration | P1 | FR-090-AC-8 | ✅ Passed locally |
| TC-390 | FamilyOutcome, FamilyResult and EvalOutcome live once in the check core, no lower layer names them, and the check core names no family cause | Unit | P1 | FR-090-AC-9 | ✅ Passed locally |
| TC-391 | An unresolved or mismatched population argument is refused at admission, and is an InternalFault inside S6a | Unit | P1 | FR-090-AC-10 | ✅ Passed locally |
| TC-392 | S2 returns one Value form per declaration, in source order, with its span and the unit edition | Unit | P1 | FR-091-AC-1 | 🚧 Planned; QSL-141 |
| TC-393 | The forms FunctionDeclaration carries its name, using alias, type forms, measure and body | Unit | P1 | FR-091-AC-2 | 🚧 Planned; QSL-141 |
| TC-394 | Each Value expression construct maps to its Expression variant, with grouping from the CST | Unit | P1 | FR-091-AC-3 | 🚧 Planned; QSL-141 |
| TC-395 | S2 refuses an inadmissible source and a unit holding a declaration with no dispatch entry | Unit | P1 | FR-091-AC-4, FR-091-AC-5, FR-091-AC-6 | 🚧 Planned; QSL-141 |
| TC-396 | S2 refuses unrepresented constructs, and the check stage refuses another family's construct with that family's cause | Integration | P1 | FR-091-AC-7, FR-091-AC-8 | 🚧 Planned; QSL-141 |
| TC-397 | S2 bounds expression depth by its explicit limit, independently of S1 | Unit | P1 | FR-091-AC-9 | 🚧 Planned; QSL-141 |
| TC-398 | The Value form builder depends only on layer 2, layer 1, F and K, and its forms hold no ValueType or NodeKey | Unit | P1 | FR-091-AC-11 | 🚧 Partial: step 1's crate edges (`tests/it/family_outcome_layering.rs`) and step 2 over the existing forms types (`qsl-forms/tests/it/identity_free_forms.rs`) pass locally; step 1's family-module edges and step 3 planned, QSL-141 |
| TC-399 | Source compiled through S1, S2 and the assembler checks and evaluates a called function | Integration | P1 | FR-091-AC-12, FR-091-AC-13 | 🚧 Planned; QSL-141 |
| TC-400 | The assembler refuses unresolved and ambiguous names, ill-formed bounds and alias cycles, reporting every error | Unit | P1 | FR-091-AC-14, FR-091-AC-15, FR-091-AC-16, FR-091-AC-17 | 🚧 Planned; QSL-141 |
| TC-401 | The assembler builds record and tuple declarations with check-minted keys and resolves names to them | Unit | P1 | FR-091-AC-18 | 🚧 Planned; QSL-141; source owner needs QSL-159 |
| TC-402 | The assembler lives in the check core and its non-test code has no edge to qsl-cst | Unit | P1 | FR-091-AC-20 | 🚧 Planned; QSL-141 |
| TC-403 | Every Value Expression node carries the span of its CST node | Unit | P1 | FR-091-AC-10 | 🚧 Planned; QSL-141 |
| TC-404 | format takes the qsl-cst ParsedSource, formats complete-V1 source and refuses inadmissible input | Unit | P1 | FR-003-AC-7, FR-003-AC-8 | ✅ Passed locally |
| TC-405 | The assembler refuses floating types and unresolved model references | Unit | P1 | FR-091-AC-19, FR-091-AC-23 | 🚧 Planned; QSL-141 |
| TC-406 | Each S2 and assembler cause maps to its catalog code with an exhaustive match | Unit | P1 | FR-091-AC-21 | 🚧 Planned; QSL-141; `stage_limit_exceeded` code needs QSL-160 |
| TC-407 | A false dispatched precondition reaches the caller as a family-owned undefined result, not a kernel Undefined | Integration | P1 | FR-090-AC-11 | ✅ Passed locally |
| TC-408 | An absent lookup key reaches the caller as a StateModel undefined result, and an absent-refused lookup as a refusal | Integration | P1 | FR-090-AC-12 | ✅ Passed locally |
| TC-409 | An enum value's VariantId is its FR-141 member node key, and its rank orders sets and bags | Unit | P1 | FR-088-AC-11 | ✅ Passed locally; QSL-131 V3; steps 2-6 via retagged tests, step 7 new |
| TC-410 | Each connected supertype component has its own object universe, and a reference key carries the authored object identity | Unit | P1 | FR-084-AC-7 | 🚧 Planned; QSL-131 |
| TC-411 | A quantity UnitId is a declared unit's node key or a compound unit's digest, and the two never compare equal | Unit | P1 | FR-088-AC-12 | ✅ Passed locally; step 3 runs under `make conformance` |
| TC-412 | The assembler resolves each using alias to a declared profile selection and refuses an undeclared one | Unit | P1 | FR-091-AC-22 | 🚧 Planned; QSL-141 |
| TC-413 | Type and declared record nodes key to the structural-node golden vectors, scoped only by owner | Unit | P1 | FR-092-AC-1, FR-092-AC-2, FR-092-AC-3, FR-092-AC-7, FR-092-AC-8, FR-092-AC-9, FR-092-AC-11, FR-092-AC-12 | 🚧 Implemented on the QSL-156 A4b branch, pending merge; AC-7's collision half, AC-11 and AC-12 unbacked there |
| TC-414 | Parameter, literal and function nodes key to the golden vectors, and a function key carries its owner | Unit | P1 | FR-092-AC-4, FR-092-AC-5, FR-092-AC-6, FR-092-AC-10 | 🚧 Implemented on the QSL-156 A4b branch, pending merge |
| TC-415 | Each checked Value expression lowers to its FR-322 node with its catalogued operation | Unit | P1 | FR-093-AC-1, FR-093-AC-2, FR-093-AC-3, FR-093-AC-4, FR-093-AC-5, FR-093-AC-6, FR-093-AC-8, FR-093-AC-10, FR-093-AC-11 | 🚧 Implemented on the QSL-156 A4b branch, pending merge, except steps 8 and 9 (QSL-212); step 6 needs the QSpec lock accessor |
| TC-416 | The v2 emission arm writes the nodes check lowered, and each emitted node recomputes to its node id | Integration | P1 | FR-093-AC-7, FR-093-AC-9 | 🚧 Planned; QSL-6 S1b after QSL-156 A4b |
| TC-417 | Model declaration, Reference and Population nodes key to the golden vectors under ModelOwner | Unit | P1 | FR-094-AC-1, FR-094-AC-2, FR-094-AC-3, FR-094-AC-4, FR-094-AC-7 | 🚧 Implemented on the QSL-156 A4b branch, pending merge |
| TC-418 | Clause function nodes key to the golden vectors with the operation member's ModelOwner | Unit | P1 | FR-094-AC-5 | 🚧 Implemented on the QSL-156 A4b branch, pending merge; step 2's C3 key unasserted there |
| TC-419 | A declared unit's quantity type is its unit node, and a compound unit's keys to the golden vectors | Unit | P1 | FR-094-AC-6, FR-094-AC-7 | 🚧 Implemented on the QSL-156 A4b branch, pending merge; QSpec unit vectors under `make conformance` |

## Stage typestate, clause and type (FR-087–088, ADR-013 S-3) coverage

[FR-087](../spec/functional/FR-087-typestate-and-cross-package-node-key.md)
(S-3a: T-1, T-3, O-15) and
[FR-088](../spec/functional/FR-088-clause-name-and-type-identity.md)
(S-3b: O-08, O-09 clause id, O-10, O-11, O-14, C-26) are specified under
QSL-158, splitting ADR-013 §7 slice S-3, which no FR owned before this
split. TC-243–260 and TC-281 above are the corresponding test cases, all
`🚧 Planned`, except TC-245 (`✅ Passed locally`, the `PackageNodeKey`
equality test), TC-282 (`✅ Passed locally`, `LibraryRefusal::class()`
and its per-variant tests), TC-253 (`✅ Passed locally`, the verified
binding) and TC-254 (`✅ Passed locally`, the name-free `ImportView`). FR-087 also resolves, by owner
ruling on QSL-158 (2026-09-21), the `CheckedPackage`-placement half of
QSL-167 (an
ADR-011 §4-versus-§6.1 defect): `CheckedPackage` is canonically layer-4
`package` per ADR-013 T-1, `check`'s S3 output becomes `CheckedGraph`
instead of the already-landed `check::CheckedPackage` (FR-068/M-5), and
FR-087-AC-9/TC-256 assert the resulting `package` → `check` (never the
reverse) edge direction, and that `CheckedPackage` is built from and wraps
a `CheckedGraph`. FR-068 is amended in place
(its AC-2, AC-6 and AC-10)
to record this supersession, since the criteria it originally tested for
`CheckedPackage`'s placement in `check` no longer hold there — see FR-068's
own Status section.

FR-087's `library` module is also the relocation target ADR-011's module
table (`:744`) already names for `src/value/library.rs` (FR-307, tested by
TC-227/`tests/library_resolution.rs`) and `src/value/package_identity.rs`
(Description, item 3); FR-087-AC-11/TC-281 assert the relocation, the
shape changes an owner ruling on QSL-158 (2026-09-21) requires
(`ExportIdentity` replaced by `PackageNodeKey`, `resolve_name` staying out
of `library`, `package_identity`'s wire node reference changed to
`WireNodeId`), and the resulting closure of `package_identity`'s current
`NodeKey::from_hex` call over wire-read JSON (`value::library` itself
makes no `NodeKey`-constructor call). That same owner ruling settles how the
relocated `resolve_libraries`/`LibraryLock` whole-graph resolution
composes with the new per-dependency `VerifiedPackage`/`ImportView`/§4
binding: `LibraryLock` is the library lock the §4 binding's third
condition reads, and `resolve_libraries`' whole-graph refusals classify,
variant by variant, to an ADR-011 I2 rule, the §4 binding's condition 2 or
condition 3, or E3 name resolution, with `DuplicatePackageId` named as
lying outside all four (FR-087-AC-12/TC-282; FR-087 Description, item 3).
`check` reads `ImportView` and `LibraryLock` (read-only) from `library`, a
permitted layer-3 edge under FR-068-AC-6's layer rule (FR-087-AC-9/TC-256).

## Kernel value identities (ADR-013 OQ-B to OQ-F) coverage

ADR-013 §8 OQ-B to OQ-F fix the kernel's reference, quantity and enum
identities. FR-088-AC-11 (TC-409) covers an enum value's FR-141
`VariantId` and its rank key. FR-088-AC-12 (TC-411) covers the two-domain
`UnitId`; it passes locally, and `make conformance` runs its QSpec-vector
step. FR-084-AC-7 (TC-410) covers one object universe per connected
supertype component and the authored object component of a reference key.
TC-409 passes locally as of QSL-131 V3; TC-410 is still `🚧 Planned` under
QSL-131. OQ-A's layer-2 edge is covered by TC-398's allow-list.

## Typed replay envelopes (FR-069–073) coverage

[FR-069](../spec/functional/FR-069-implement-typed-proof-result-envelope.md)
through [FR-073](../spec/functional/FR-073-implement-redacted-safe-diagnostic-rendering.md)
are implemented under issue #231, in `qsl-replay/src/{bounds,identity,proof_result,
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

- TC-177 (FR-069-AC-1): `qsl-replay/src/proof_result.rs::tests::tc_177_every_fr331_value_maps_to_its_exact_category`
- TC-178 (FR-069-AC-2, FR-069-AC-4): `qsl-replay/src/proof_result.rs::tests::tc_178_refuses_unknown_version_vocabulary_or_oversized_envelope`
- TC-179 (FR-069-AC-3): `qsl-replay/src/proof_result.rs::tests::tc_179_round_trip_preserves_backend_tool_pin_and_dispositions`
- TC-180 (FR-070-AC-1): `qsl-replay/src/witness.rs::witness_tests::tc_180_exactly_one_field_and_derived_facts_track_the_stored_transcript`
- TC-181 (FR-070-AC-2, FR-070-AC-6, FR-070-AC-7): `qsl-replay/src/witness.rs::witness_tests::tc_181_refuses_malformed_transcripts`, `::envelope_tests::tc_181_refuses_an_out_of_domain_digest`, `::envelope_tests::tc_181_refuses_an_oversized_encoding`
- TC-182 (FR-070-AC-3): `qsl-replay/src/witness.rs::envelope_tests::tc_182_round_trip_preserves_every_o25_member_and_the_transcript`
- TC-183 (FR-070-AC-4): `qsl-replay/src/witness.rs::envelope_tests::tc_183_refuses_reconstruction_when_any_o25_member_is_missing` — caveat: this is a `Property`-typed row, but the test asserts only four of the roughly thirteen O-25 members individually (`backend`, `trace_position`, `source_digests`, `obligation_identity`); the rest share the identical `.ok_or(WitnessRefusal::MissingMember(...))?` pattern but are not each individually exercised.
- TC-184 (FR-070-AC-5): `qsl-replay/src/witness.rs::envelope_tests::tc_184_family_payload_is_a_typed_extension_point` — caveat: the "typed extension point" half is asserted by attaching and round-tripping a new payload type; the "not an untyped map" half is a source-inspection fact (no `get_extra`/string-keyed accessor exists on `WitnessEnvelope`), not itself a runtime assertion.
- TC-185 (FR-071-AC-1): `qsl-replay/src/request.rs::tests::tc_185_carries_exactly_o26_members_and_round_trips`
- TC-186 (FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7): `qsl-replay/src/request.rs::tests::tc_186_byte_provision_is_digest_only_complete_and_bounded`
- TC-188 (FR-071-AC-4): `qsl-replay/src/request.rs::tests::tc_188_refuses_unknown_version_or_profile_before_recompilation`
- TC-189 (FR-072-AC-1): `qsl-replay/src/result.rs::tests::tc_189_witness_and_input_arms_stay_distinct`
- TC-190 (FR-072-AC-2): `qsl-replay/src/result.rs::tests::tc_190_disagreement_settles_inconclusive_and_is_never_repaired`
- TC-191 (FR-072-AC-3, FR-072-AC-5): `qsl-replay/src/result.rs::tests::tc_191_round_trips_the_fr351_record_and_compares_structurally`
- TC-209 (FR-073-AC-1): `qsl-replay/src/witness.rs::witness_tests::tc_209_debug_and_display_never_reproduce_the_full_transcript`
- TC-210 (FR-073-AC-2): `qsl-replay/src/request.rs::tests::tc_210_debug_never_reproduces_byte_provision_raw_bytes`
- TC-211 (FR-073-AC-3): `qsl-replay/src/lib.rs::redaction_tests::tc_211_refusal_causes_redact_while_typed_accessors_stay_readable`

TC-187 stays `🚧 Planned; #231` and TC-192 is now attributed
`🚧 Planned; #217`: both rows' core claim is an absence of something (no
bare-`&str`/`String` entry point exists anywhere on `ReplayRequest`'s public
API for TC-187; no fifth type is defined for TC-192's exemplar) that only a
source-level inspection can establish, not a runtime assertion inside the
test itself, so neither test carries a `#[trace]` tag naming the AC it
cannot fail on. The positive half of each (a multi-segment name round-trips
its segments; two structurally different functions reuse the four #231
types) is exercised by
`qsl-replay/src/request.rs::tests::tc_187_selection_is_always_a_typed_qualified_name`
and `qsl-replay/src/result.rs::tests::tc_192_function_exemplar_reuses_the_four_types_with_none_new`
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
- TC-220 (FR-082-AC-3): `qsl-semantics/tests/it/model_conformance.rs::an_ancestor_chain_at_the_configured_bound_is_admitted_and_one_longer_refuses` (the caller-supplied `ancestor_steps` ceiling, exactly at and one past it) and `::a_type_with_more_than_128_ancestors_passes_conformance_at_default_limits`
- TC-221 (FR-082-AC-4): `tests/model_conformance.rs::r08a_*_refuses` and `::r08b_*_discharges_the_obligation`
- TC-222 (FR-083-AC-1): `tests/model_dispatch.rs::d04_registration_order_does_not_change_the_linked_table`
- TC-223 (FR-083-AC-2): `tests/model_dispatch.rs::d03_no_candidate_*_no_applicable` and `::d02_an_undominated_multi_way_tie_*`
- TC-224 (FR-083-AC-3): `tests/model_dispatch.rs::d02_an_undominated_multi_way_tie_*` (the same `Ambiguous` outcome carries no table for the family's other, cleanly-resolving subtype)
- TC-225 (FR-083-AC-4): `qsl-semantics/tests/it/model_dispatch.rs::a_family_at_the_configured_bound_links_and_one_step_more_refuses` (the caller-supplied `family_steps` ceiling, exactly at and one past it, where the uncounted redefiner would make the family ambiguous), `::a_dispatch_family_with_more_than_128_redefinition_steps_links_at_default_limits` and `::a_dispatch_family_with_more_than_128_redefinition_steps_passes_the_checked_bridge`
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

The remaining rows (TC-214, TC-216, TC-217, TC-227, TC-230 through TC-234,
TC-236, TC-240 and TC-241) name real behavior with no existing test
asserting their exact content — `title`/`displayName` independence,
cross-process purity, dangling relationship ends, the
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

## Kernel population identity (FR-089) coverage

[FR-089](../spec/functional/FR-089-carry-population-identity-across-the-kernel-boundary.md)
carries ADR-013 O-13's Population row (QC-21), authored under QSL-172 to
close an AD-016 kernel-row gap. QSL-131 Slice B (kernel side) landed the
opaque `PopulationId` type and `Value::Population(PopulationId)` in
`quire-exact`. Kernel `ValueType::admits` returns `false` for every
`(ValueType::Population(_), Value::Population(_))` pair, `plan_pairs` refuses
a population pair with `Refusal::CheckedInvariant`, and `compare_keys` yields
no key for one (FR-089-AC-6). QSL has no equality or key of its own: it
calls `quire_exact::member_equal` and `quire_exact::compare_keys`. TC-297 is `✅ Passed locally`, backed by
`value::tests::admits_refuses_a_population_pair`,
`equality::tests::plan_pairs_refuses_a_population_pair` and
`key::tests::compare_keys_yields_no_key_for_a_population_pair`. The
declared-maximum comparison is a QSL-layer check (FR-089-AC-5, TC-295): QSL
`model`/the evaluator resolves the binding, then compares its declared
maximum. TC-292 is `✅ Inspected locally`:
the payload type is `PopulationId`, and the crate-DAG direction (ADR-011
§6.1) makes a `PopulationBinding` import structurally impossible, not merely
absent from a scan.

QSL-131 Slice B's other half (`model` minting and the evaluator's
resolution step) landed: `model::population::mint_population_id` mints a
`PopulationId` at admission time (`admit_binding`/`admit_invocation`,
`qsl-semantics/src/model/population.rs`), and `ObjectEnvironment` records the
`PopulationId` -> `PopulationBinding` correspondence
(`with_population`/`resolve_population`, `qsl-semantics/src/model/object_environment.rs`) the
evaluator resolves through (`Machine::resolve_population`,
`qsl-eval/src/value/expression/evaluate.rs`). TC-291, TC-293, TC-294, TC-295 and
TC-296 are `✅ Passed locally`. TC-296 backs FR-089-AC-1's admission-role
discriminator half specifically: a `Direct` (standalone `admit_binding`)
admission and an `admit_invocation`-attached `Post` binding over the same
domain package and `population_key` mint distinct `PopulationId`s under the
closed three-state discriminator (`Direct`, `Pre`, `Post`), rather than
colliding under a `pre`/`post`-only reading -- this discriminator is
computed entirely by QSL `model`, not the kernel.

FR-089-AC-4 (TC-294) and FR-089-AC-5 (TC-295) are backed at the argument-
admission boundary (`CheckedPackage::call`/`evaluate`'s own `validate`,
`qsl-eval/src/value/expression/mod.rs`), not only inside the evaluator's own
`AllInstances`/`Lookup` consumption sites: an unresolved identity or a
mismatched declared maximum refuses whether or not the checked body actually
consumes the parameter (PR #326 review finding F1), for both `allInstances`
and `lookup` consumers. `Machine::resolve_population`'s own defence-in-depth
check remains, unreachable through either public entry point for a checked
program (see that method's own doc), but since FR-090-AC-10 (QSL-174) it is
no longer a kernel refusal: meeting either condition there is an S6a
invariant break, `Err(InternalFault)`, not `Refusal::UnresolvedPopulation`/
`Refusal::PopulationMaximumMismatch`, which are deleted.

FR-089's own admission preimage (domain package selection, `population_key`,
admission role) does not distinguish two bindings that differ only in
document content or declared maximum, admitted under the same
package/key/role within one evaluation -- an open spec question (Linear
QSL-131) this Slice does not resolve.
`ObjectEnvironment::with_population` is the interim guard: it refuses to
record a second, unequal binding under an id already bound, rather than
silently letting the later admission overwrite the earlier one.

## Family evaluation outcome (FR-090) coverage

[FR-090](../spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md)
carries ADR-013 O-16, O-17, T-4 and T-6 for the S6a result: the layer-5
`Evaluation { outcome, location, losses }` whose `outcome` is the layer-3
`check`-core `FamilyOutcome { Evaluated(quire_exact::Outcome<T>),
FamilyEvaluated(FamilyResult) }`, beside T-4's `InternalFault`; an S6a input
type that admits no `Relation`; F `diagnostic`'s map to category `refusal`;
the evaluation-time `wrong_snapshot` and model-query refusals and the `precondition-false` and `absent-key` undefined results
carried in `FamilyOutcome::FamilyEvaluated` instead of the kernel types, and
unresolved population arguments refused at admission (`CallFailure::Input`)
and faulted inside S6a. TC-382, TC-384 to TC-391, TC-407 and TC-408
(FR-090-AC-1, AC-3 to AC-12) are `✅ Passed locally`. TC-390 uses the resolved-import and definition-scan approach of
TC-256, TC-170 and TC-176. TC-388, TC-389, TC-407 and TC-408 match the
`FamilyOutcome::FamilyEvaluated` arm the ADR-013 O-16 ruling on FR-090-OQ-1
fixes, inside the `Evaluation` the ruling on FR-090-OQ-3 fixes; TC-385 backs
the ruling on FR-090-OQ-2.

## Value forms and assembler (FR-091) and format input (FR-003-AC-7, AC-8) coverage

[FR-091](../spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md)
carries ADR-011 §2.1 to §2.3 (E2, E3, E9), §3 and §6.1, ADR-012 §1, §3 and
§4.3, and ADR-013 O-11, O-17, R-07, T-4 and T-5 for the `Value` family's S2
production and the forms-to-`PackageDeclarations` assembler in the layer-3
`check` core. TC-392 to TC-403, TC-405, TC-406 and TC-412 are `🚧 Planned`
under QSL-141, except TC-398's crate edges and its step 2 over the existing
forms types, which pass locally (`🚧 Partial`). TC-398 and TC-402 use the resolved-import and definition-scan
approach of TC-256, TC-170 and TC-390. TC-399 is the end-to-end case from
source to `CheckedPackage::call`. TC-404 backs FR-003-AC-7
and AC-8, the `format` input retargeted to the `qsl-cst` CST (ADR-011 §7.3
M-6a), under QSL-8.

## Value node keys and expression lowering (FR-092, FR-093) coverage

[FR-092](functional/FR-092-key-type-parameter-and-declared-nodes.md) carries
ADR-013 O-04 and OQ-G: QSL's `quire.structural-node/v1` preimage for type,
value, parameter and declared function nodes, with golden vectors T1 to T12,
D1 to D5, P1 to P4, L1 to L3 and F1 to F3, and it keys recursion groups by a
content order (QSL-211), with vectors L5, L6, E11 to E13 and G1 to G15.
TC-413 and TC-414 back its twelve ACs.
[FR-093](functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
carries the lowering of checked expressions to FR-322 application nodes, with
vectors E1 to E3 and the ownership split: QSL-156 A4b lowers and keys in
`check`, and QSL-6 S1b serializes. Its text-leaf walk ends a recursive
composite in a recursion leaf (QSL-212), with vectors T13, T14, G16 to G21,
S4, S5, P10 to P16 and E14 to E17. TC-415 backs AC-1 to AC-6, AC-8, AC-10
and AC-11, and TC-416 AC-7 and AC-9. TC-413 to TC-415 are implemented on the
QSL-156 A4b branch, pending merge, except TC-415 steps 8 and 9; TC-416 is
planned for QSL-6 S1b. No FR-093 AC backs the `Pre` row; the
`ProtocolClause` postcondition lowering backs it. Remaining work: #218.
[FR-094](functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
keys the nodes those two leave to the model layer: model declaration nodes
and clause functions under QSpec's `ModelOwner`, the `Reference<T>` and
`Population<T>[N]` type nodes, and quantity type nodes for declared and
compound units, with vectors M1 to M5, R1 to R5, S1 to S3, PO1 to PO3, P5
to P8, L4, E4 to E10, C1 to C6 and U1 to U4. TC-417 backs AC-1 to AC-4 and AC-7, TC-418
AC-5 and TC-419 AC-6 and AC-7. All three are implemented on the QSL-156 A4b
branch, pending merge.
