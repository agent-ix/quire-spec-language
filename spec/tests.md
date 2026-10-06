---
id: TM-001
title: "Root test matrix"
type: TestMatrix
---

## Overview

The root matrix indexes the test cases that no area matrix owns. Each
coverage section below names the requirements it covers.

## Requirements Traceability

A `#[trace]` tag with a bare id (`TC-196`, `FR-075-AC-4`) names a QSL
artifact. A tag that names a quire-specification requirement or test case
carries the `QSpec-` prefix (`QSpec-TC-196`, `QSpec-FR-151-AC-2`), because
the two repositories number their artifacts independently and the same id
names different artifacts in each.

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-059 | FR-059-AC-1 | TC-156 | ✅ Passed locally |
| FR-059 | FR-059-AC-2 | TC-156 | ✅ Passed locally |
| FR-059 | FR-059-AC-3 | TC-156 | ✅ Passed locally |
| FR-059 | FR-059-AC-4 | TC-156 | ✅ Passed locally |
| FR-059 | FR-059-AC-5 | TC-156 | ✅ Passed locally |
| FR-059 | FR-059-AC-6 | TC-156 | ✅ Passed locally |
| FR-059 | FR-059-AC-8 | TC-156 | ✅ Passed locally |
| FR-059 | FR-059-AC-9 | TC-156 | ✅ Passed locally |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-156 | Report FB-05 and FB-11 violations over the four-repository backend dependency graph | Integration | P1 | FR-059-AC-1..FR-059-AC-6, FR-059-AC-8, FR-059-AC-9 | ✅ Passed locally |
| TC-157 | Report pending, passing and failing T-12 API-surface rules | Integration | P1 | FR-060-AC-1..FR-060-AC-4 | ✅ Passed locally for FR-060-AC-1..AC-3; 🚧 FR-060-AC-4's T12-B and T12-C clauses amended to allow-lists plus debt lists (2026-09-22), gate rewrite planned |
| TC-160 | Every family implements the six-part checked contract with no bypass | Unit | P1 | FR-062-AC-1..FR-062-AC-7, FR-062-AC-9, FR-062-AC-13, FR-057-AC-10 | ✅ Passed locally for AC-2, AC-4 (step 5), AC-5 (all three clauses; the third, `check` and `emit_checked` never return `Incomplete`), AC-7 backed by TC-378, AC-9 (step 8), AC-12, and AC-13 (steps 10 and 11); AC-3 (all three clauses, for `src/check` and `src/family`) and AC-6 (first sentence, an API-surface scan and a rename test); AC-1 (step 1: `compile_fail` doctests on `FamilyContract`, omitting `requirements` and omitting the checked-input parameter, plus a compiling control; `evaluate` is crate-private to `qsl-eval`, so its omission is not a case); depth steps re-planned for ADR-030 |
| TC-161 | The seam probe demonstrates exhaustiveness at every S1-S4 seam | Integration | P1 | FR-063-AC-1..FR-063-AC-7, FR-062-AC-8, FR-067-AC-4 | ✅ Passed locally for FR-062-AC-8 (`CheckCause::code`), backed by the `make seam-probe` gate (part of `make ci`); FR-063-AC-6 backed (S1/S2/S3/S4 all land, including the qsl-forms leading-token-kind table via its own probe build); FR-063's other criteria per its own Status section |
| TC-162 | The string-edge scan reports every unmarked string dispatch | Integration | P1 | FR-064-AC-1..FR-064-AC-6 | ✅ Passed locally |
| TC-163 | Function identity and provenance survive checking and package conversion | Integration | P1 | FR-065-AC-1..FR-065-AC-3, FR-065-AC-8 | ✅ Passed locally for FR-065-AC-1, AC-2 (`a_function_identity_survives_emission_and_the_i2_read`, `qsl-package/src/emit/tests.rs`, a real `emit_checked`/I2 round trip) and AC-3 (`qsl-package/src/emit/tests.rs`, a real `emit_checked`/v2 round trip over the call's own occurrence); AC-8 backed (#384, the FR-092 and FR-093 keys) |
| TC-164 | A call receives the same verdict from a declaration body and from a clause expression | Integration | P1 | FR-065-AC-5 | ✅ Passed locally |
| TC-165 | The migration recipe names every required test, conversion, removal condition and remaining family | Manual | P1 | FR-066-AC-1..FR-066-AC-4 | ✅ Verified by inspection |
| TC-166 | The replay executor selects a function by typed QualifiedName, never by string | Unit | P1 | FR-062-AC-10, FR-065-AC-6 | ✅ Passed locally (`qsl-replay`: `compile_fail` doctests on `replay` and `call_site` with compiling controls; `tc_166_case_variant_functions_each_replay_their_own_body`, `tc_166_call_site_locates_each_case_variant_function`, `tc_444_a_selection_naming_no_function_refuses`, `call_site_refuses_an_unknown_function_name`) |
| TC-167 | The S2 forms stage refuses on a recovering CST and mints no identity | Unit | P1 | FR-067-AC-1, FR-067-AC-2, FR-067-AC-3, FR-067-AC-7, FR-067-AC-8 | ✅ Passed locally |
| TC-168 | SEAM-5 (LoweredSourceGraph) is deleted in the same change as the S2 forms core | Integration | P1 | FR-067-AC-5, FR-067-AC-6 | ✅ Passed locally |
| TC-170 | check-stage modules move to check with no duplication | Unit | P1 | FR-068-AC-1, FR-068-CON-1, FR-068-CON-3 | ✅ Passed locally |
| TC-171 | CheckedPackage/CheckedExpression/CheckedFunction constructors are private to check | Manual | P1 | FR-068-AC-2 | ✅ Inspected locally |
| TC-172 | check's real import graph has no edge into value::expression or into checking, and value::expression re-exports no check item | Integration | P1 | FR-068-AC-3, FR-068-AC-10 (retired) | ✅ Passed locally; steps 1-5 retired, enforced by Cargo's `qsl-semantics` → root-crate refusal and TC-390 (`no_crate_below_layer_three_depends_on_the_check_core`); step 6 by `tests/it/layer_crate_reexports.rs` |
| TC-173 | Refusal split: check causes in check, InputRefusal in value::expression | Unit | P1 | FR-068-AC-4, FR-068-AC-8 | ✅ Passed locally |
| TC-174 | Checking and evaluation produce identical results before and after the split | Integration | P1 | FR-068-AC-5 | ✅ Passed locally |
| TC-175 | The move stays inside M-5: no early M-2 work, no edge widening | Integration | P1 | FR-068-AC-6, FR-068-AC-7 | 🚧 FR-068-AC-6 steps amended to the module-level layer rule (2026-09-22), gate rewrite planned; FR-068-AC-7 steps retired by FR-074, superseded by TC-261 |
| TC-176 | The interim `model` -> `check` edge stays bounded to two files and thirteen names, imported directly | Unit | P1 | FR-068-AC-9, FR-068-CON-5 | ❌ Retired by FR-074, superseded by TC-262 |
| TC-193 | Candidate set matches registered backends advertising the requested kind | Unit | P1 | FR-075-AC-1, FR-075-AC-5 | ✅ Passed locally; AC-5's no-second-enum half by inspection |
| TC-194 | Registry candidate sets are invariant under registration-order permutation | Property | P1 | FR-075-AC-2, FR-075-AC-4, FR-075-AC-7, FR-080-AC-1 | 🚧 Planned (implementation: one `duplicate-backend` refusal per identity, identity-only candidates) |
| TC-195 | An unregistered named backend yields a distinct unknown-backend marker | Unit | P1 | FR-075-AC-3 | ✅ Passed locally |
| TC-196 | A conflicting backend identity registration refuses both and withdraws the held registration | Unit | P1 | FR-075-AC-4 | ✅ Passed locally; inverted (quire-specification FR-290-AC-10) |
| TC-433 | A backend member is its identity, kept verbatim | Unit | P1 | FR-075-AC-6 | ✅ Passed locally |
| TC-447 | Duplicate backend identity registration matches quire-specification TC-282 under every order | Unit | P1 | FR-075-AC-4, FR-075-AC-7 | 🚧 Planned (implementation: one `duplicate-backend` refusal per identity, identity-only candidates) |
| TC-448 | An identical repeat registration is idempotent | Unit | P1 | FR-075-AC-7 | ✅ Passed locally (quire-specification FR-290-AC-9) |
| TC-449 | The request builder writes one item per requirement record | Unit | P1 | FR-075-AC-8 | ✅ Passed locally |
| TC-197 | Empty candidate set carries the data an unsupported warning needs | Unit | P1 | FR-076-AC-1, FR-076-AC-2 | ✅ Passed locally; `qsl-route/tests/it/route_registry.rs` |
| TC-198 | Backend absence never settles as a refusal or a hold at the registry | Unit | P1 | FR-076-AC-3 | ✅ Passed locally; `qsl-route/tests/it/route_registry.rs` |
| TC-199 | requests::report takes no Backend parameter and has no capability/family disposition | Unit | P1 | FR-077-AC-1, FR-077-AC-2 | ✅ Passed locally; `compile_fail` doctests traced on `FR077Doctests` (`src/linking/composed/requests.rs`) |
| TC-200 | Requests are still recorded as data after negotiation removal | Unit | P1 | FR-077-AC-3 | ✅ Passed locally; `tests/it/composed_admission_stages.rs` |
| TC-201 | value::ieee and value::division carry no negotiate_* function | Unit | P1 | FR-078-AC-1, FR-078-AC-2 | ✅ Passed locally; `cargo test --doc -p quire-spec-language` |
| TC-202 | value::ieee and value::division evaluation is unchanged by negotiate_* removal | Unit | P1 | FR-078-AC-3 | Retired with FR-078-AC-3; no test carries the tag |
| TC-205 | cargo-deny denies inventory, linkme and ctor | Integration | P1 | FR-080-AC-2 | ✅ Passed locally with `cargo-deny` installed; `qsl-route/tests/it/route_registry.rs` (the test skips when `cargo-deny` is absent; `make cargo-deny-bans` does not) |
| TC-206 | The registry module lint gate finds no static, OnceLock or thread_local | Integration | P1 | FR-080-AC-3 | ✅ Passed locally; `xtask/src/route_lint.rs` |
| TC-207 | One unit test exists and passes per ADR-012 §5.2 row | Unit | P1 | FR-080-AC-4 | ✅ Passed locally; `qsl-route/tests/it/route_registry.rs` |
| TC-208 | The S7 seam probe fails to compile the registry arm on an unhandled capability-kind variant | Integration | P1 | FR-080-AC-5 | 🚧 No tagged test; `make seam-probe` (part of `make ci`) confirms the S7 location `qsl-route/src/lib.rs::same_kind` |
| TC-177 | The proof-result envelope maps every FR-331 outcome to its exact O-16 category | Property | P1 | FR-069-AC-1 | ✅ Passed locally |
| TC-178 | The proof-result reader refuses an oversized envelope | Unit | P1 | FR-069-AC-4 | ✅ Passed locally |
| TC-179 | A positive proof-result envelope round-trips its backend identity and dispositions exactly | Unit | P1 | FR-069-AC-3 | ✅ Passed locally |
| TC-180 | The witness envelope stores the transcript once and derives every other fact from it | Unit | P1 | FR-070-AC-1 | ✅ Passed locally |
| TC-181 | The witness envelope refuses a malformed transcript, an out-of-domain digest, or an oversized encoding | Property | P1 | FR-070-AC-2, FR-070-AC-6, FR-070-AC-7 | ✅ Passed locally |
| TC-182 | A positive witness envelope round-trips its transcript and every O-25 member exactly | Unit | P1 | FR-070-AC-3 | ✅ Passed locally |
| TC-183 | The witness envelope refuses reconstruction when any one O-25 member is missing | Property | P1 | FR-070-AC-4 | ✅ Passed locally |
| TC-184 | A family adds a typed witness payload through a typed extension point, not an untyped map | Unit | P1 | FR-070-AC-5 | ✅ Passed locally |
| TC-185 | The replay request carries exactly the O-26 members and round-trips them exactly | Unit | P1 | FR-071-AC-1 | ✅ Passed locally |
| TC-186 | The replay request's byte provision is reachable only by digest, never by path, is complete, and stays within the size bound | Property | P1 | FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7, FR-071-AC-9, FR-071-AC-10, FR-071-AC-11 | ✅ Passed locally; step 8 (FR-071-AC-10, an unknown semantic profile) passes locally; step 7 (FR-071-AC-9, the package reference's `dependencies`) passes locally; step 9 (FR-071-AC-11, a `sha256-jcs` entry's refusal keeps its cause) passes locally |
| TC-187 | The replay request's function selection accepts only a typed QualifiedName, never a bare string | Unit | P1 | FR-071-AC-3 | 🚧 Planned; #231 |
| TC-189 | The replay result keeps the Witness arm and Input arm distinct, each with its own settlement | Unit | P1 | FR-072-AC-1 | ✅ Passed locally |
| TC-190 | A replay disagreement settles inconclusive with a typed cause and is never repairable | Unit | P1 | FR-072-AC-2 | ✅ Passed locally |
| TC-191 | A replay result's nested witness record round-trips exactly, compares without display-text interpretation, and refuses an oversized encoding | Unit | P1 | FR-072-AC-3, FR-072-AC-5 | ✅ Passed locally |
| TC-192 | #217's function exemplar builds on the existing result/request/witness types with no new type | Integration | P1 | FR-072-AC-4 | 🚧 Planned; #217 |
| TC-209 | The witness envelope's Debug and Display rendering never reproduces the full transcript | Unit | P1 | FR-073-AC-1 | ✅ Passed locally |
| TC-210 | The replay request's Debug and Display rendering never reproduces a byte-provision entry's raw bytes | Unit | P1 | FR-073-AC-2 | ✅ Passed locally |
| TC-211 | A refusal cause from any of the four envelopes renders with no unredacted transcript, byte or value content, while the typed accessor stays fully readable | Unit | P1 | FR-073-AC-3 | ✅ Passed locally |
| TC-213 | Original declaration keys survive normalization unchanged | Unit | P1 | FR-081-AC-1 | ❌ Not passed: the effective view omits operation members (see the FR-081–086 coverage note) |
| TC-214 | Structurally identical declarations from distinct originals never collapse to one effective identity | Unit | P1 | FR-081-AC-2 | 🚧 Planned; #120 |
| TC-215 | A dominated redefinition is retained for provenance, not deleted | Unit | P1 | FR-081-AC-3 | ✅ Passed locally |
| TC-216 | Changing only a display name leaves every key, identity and ordering unchanged | Property | P1 | FR-081-AC-4 | 🚧 Planned; #120 |
| TC-217 | The model binder is a pure function with no cross-run ambient state | Unit | P1 | FR-081-AC-5 | 🚧 Planned; #120 |
| TC-218 | Redefinition variance checking reports every failing axis, not only the first | Unit | P1 | FR-082-AC-1 | ✅ Passed locally |
| TC-219 | A missing redefinition target and a supertype cycle each refuse with a named cause | Unit | P1 | FR-082-AC-2, FR-082-AC-6 | ✅ Passed locally |
| TC-220 | A conformance ancestor walk that exhausts its edge limit stops incomplete instead of truncating | Unit | P1 | FR-082-AC-3, FR-082-AC-6, FR-082-AC-7 | ✅ Passed locally (ADR-030: `ancestor_steps` is a charged edge-count limit, not a depth ceiling) |
| TC-221 | A narrowing field redefinition requires an established postcondition | Unit | P1 | FR-082-AC-4 | ✅ Passed locally |
| TC-222 | Dispatch selects the descendant candidate independent of declaration order | Property | P1 | FR-083-AC-1 | ✅ Passed locally |
| TC-223 | No-applicable-candidate and multiple-undominated-candidates are named separately | Unit | P1 | FR-083-AC-2 | ✅ Passed locally |
| TC-224 | An ambiguous family produces no dispatch table, not even for subtypes that resolved cleanly | Unit | P1 | FR-083-AC-3 | ✅ Passed locally |
| TC-225 | A dispatch family walk that exhausts its edge limit stops incomplete instead of reporting a false unique winner | Unit | P1 | FR-083-AC-4 | ✅ Passed locally (ADR-030: `family_steps` is an edge-count work limit, not a depth ceiling) |
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
| TC-240 | allInstances and lookup return the FR-153 typed result shape and its bound/foreign/ineligible refusals | Unit | P1 | FR-084-AC-5 | 🚧 Partly passed: step 3 (the unbounded population) passes locally; the other steps are planned under #120 |
| TC-241 | Kind mapping runs interfaces first, and a port whose interface type is not an Interface refuses wrong-export | Unit | P1 | FR-086-AC-5 | 🚧 Planned; #120 |
| TC-242 | A selected object's reference key names the same most-specific type through every conforming query | Unit | P1 | FR-084-AC-6 | ✅ Passed locally |
| TC-243 | Typestate constructors are private to their stage module | Manual | P1 | FR-087-AC-1 | 🚧 Partial: steps 1-3 by `xtask::typestate_scan` (one layer-crate definition per type, in its owning file, every field private, no state generic); the constructor half by TC-244's allow-list. Step 4 (accessor-only reads) is not backed: a private field stays readable by its module's child modules |
| TC-244 | compile_fail matrix over every forbidden typestate construction (R-10, O-15) | Unit | P1 | FR-087-AC-2 | ✅ Passed locally: step 0 by `xtask::typestate_scan` over all five stage types and the lane-private namesakes, against named constructor sites (a value handed out through an out-parameter is not seen); rows 1-5 by `compile_fail` doctests, each paired with a block over the same names that must compile, since stable rustdoc does not check the error code; row 6 is row 2's |
| TC-245 | PackageNodeKey has exactly one shape and declared equality | Unit | P1 | FR-087-AC-5 | ✅ Passed locally |
| TC-246 | ResolvedSourcePackage is retired, with no dangling caller | Integration | P1 | FR-087-AC-7, FR-087-CON-4 | ✅ Passed locally: steps 1-3 by a workspace search and build (no automated scan), step 4 by TC-246's mapping table |
| TC-247 | The canonical EmittedPackage/CheckedPackage stay distinct from their pre-existing namesakes | Integration | P1 | FR-087-AC-8, FR-087-AC-10 | 🚧 Partial: steps 1-3, 6 and 7 by `xtask::typestate_scan`; step 4 by field names only (no method or trait comparison); step 5 was checked once, by this PR's diff |
| TC-248 | Frame identity's subject sets resolve to DeclarationKey through the model correspondence | Unit | P1 | FR-088-AC-2 | ✅ Passed locally; #300 |
| TC-249 | Clause identity is the checked node id; the occurrence key disambiguates structurally identical clauses | Unit | P1 | FR-088-AC-3 | ✅ Passed locally; #300 |
| TC-250 | Clause-kind wire-string totality in both directions, with mutation coverage | Property | P1 | FR-088-AC-4 | ✅ Passed locally; #300 |
| TC-251 | Qualified-name resolution is confined to the check stage (R-06) | Integration | P1 | FR-088-AC-5 | ✅ Passed locally; #300 (the `QualifiedName` half only -- AC-5's "or a bare string" half is investigated, not enforced; see `tests/it/name_resolution_confinement.rs`'s own module doc) |
| TC-252 | Checked type node to kernel ValueType is total, tested per type-node form including a sum | Unit | P1 | FR-088-AC-9, FR-088-AC-10 | ✅ Passed locally; #300 |
| TC-253 | The verified binding admits VerifiedPackage only under all three conditions, refusing each failure independently | Integration | P1 | FR-087-AC-3, FR-087-AC-14 | ✅ Passed locally; #340 (step 3 and steps 5-7 at the `library` level, see FR-087 Status); step 9's QSpec-fixture read under `make conformance` |
| TC-254 | library converts VerifiedPackage to ImportView without resolving any name | Unit | P1 | FR-087-AC-4 | ✅ Passed locally (steps 1-2: the view keyed by `WireNodeId`; steps 3-5: the `library` signature scan and `check::imports`, see FR-087 Status) |
| TC-255 | NodeKey is never minted from a WireNodeId; E4 and E9 resolve by lookup, never by construction | Integration | P1 | FR-087-AC-6 | 🚧 Partial: steps 1, 2 and 5 by `xtask::typestate_scan`, which sees aliases, qualified self types, trait default methods, statics, consts and macro tokens; step 3 for a wire value named in the minting item itself only; step 4 is replaced by this test, since `arch-lint api-surface` is not part of `make ci`; step 6's transitive trace is not backed |
| TC-256 | check and package's dependency edge is one direction, and CheckedPackage wraps CheckedGraph | Integration | P1 | FR-087-AC-9 | ✅ Passed locally; step 3 by `tests/it/layer_crate_reexports.rs`, steps 1 and 6 by Cargo's `qsl-package` → `qsl-semantics` edge and TC-390; steps 2, 4 and 5 by inspection |
| TC-257 | Exactly one closed checked clause-kind enum exists, and syntax::ClauseKind gains no variant | Unit | P1 | FR-088-AC-1 | ✅ Passed locally; #300 |
| TC-258 | QualifiedName is used only as a declared preimage component, never as an identity | Manual | P1 | FR-088-AC-6 | 🚧 Partial: steps 1 and 3 by `xtask::typestate_scan` over the layer crates' struct and variant fields and function return types, aliases seen through; step 4 by `value::expression::family`. Not backed: step 2 (a hand-written `PartialEq`/`Hash`, or a name beside a filler field), local maps, and the root crate's lane-private `QualifiedName` types (for example `src/runtime/validation.rs`'s `StateIndexes`) |
| TC-259 | A package type's identity is its checked node id, scoped only by owner | Unit | P1 | FR-088-AC-7 | 🚧 Partial: step 4 (a declared type's node id is its declaration's key) passes locally, #300; steps 1 to 3 (same owner, different owners, builtin or anonymous type) and step 5 (recompilation) planned |
| TC-260 | ValueTypeRef is exactly the two-member union Native/Package | Manual | P1 | FR-088-AC-8 | 🚧 Partial: steps 1-2 by `xtask::typestate_scan`; step 3 for `model` fields whose name contains `type`, and the three value-type records; step 4 is not backed |
| TC-261 | M-2's items are relocated to check and absent from model | Unit | P1 | FR-074-AC-1, FR-074-AC-2 | ✅ Passed locally |
| TC-262 | The model -> check edge is fully closed after M-2 | Unit | P1 | FR-074-AC-3 | ✅ Passed locally |
| TC-281 | value::library and value::package_identity relocate into the new top-level library module, per the R-10/T-3 shapes | Integration | P1 | FR-087-AC-11 | 🚧 Partial: steps 1, 4 and 5 by `xtask::typestate_scan`; step 3 by TC-227; step 6 by `arch-lint`'s T12-B allow-list, which names only `check`; step 2's byte-identity against the pre-relocation tree is not backed |
| TC-282 | Every resolve_libraries refusal classifies to an I2 rule, a §4 condition, E3 resolution, or a named exception | Unit | P1 | FR-087-AC-12 | ✅ Passed locally |
| TC-291 | PopulationId is deterministic over its admission preimage and distinguishes distinct admissions | Unit | P1 | FR-089-AC-1, FR-089-AC-7 | ✅ Passed locally for the three-fact preimage; 🚧 Planned for ADR-016 ID-5's seven-member preimage (AC-1 as amended, AC-7), ADR-016 G-3 |
| TC-293 | The evaluator resolves a Value::Population identity through the recorded correspondence, not a carried payload | Unit | P1 | FR-089-AC-3 | ✅ Passed locally |
| TC-294 | An unresolved PopulationId refuses with a typed cause, not a panic or Undefined | Unit | P1 | FR-089-AC-4 | ✅ Passed locally |
| TC-295 | The QSL layer admits a Value::Population identity under ValueType::Population by its resolved binding's declared maximum | Unit | P1 | FR-089-AC-5 | ✅ Passed locally |
| TC-296 | A standalone Direct admission and an invocation's Post binding over the same domain package and population_key mint distinct PopulationIds | Unit | P1 | FR-089-AC-1 | ✅ Passed locally |
| TC-376 | Function application checking accepts a well-typed call and refuses wrong arity, an unknown name and a type mismatch | Unit | P1 | FR-065-AC-4 | ✅ Passed locally |
| TC-377 | Function declaration checking accepts a well-typed declaration, reports its calls, and refuses an ill-typed body | Unit | P1 | FR-065 | ✅ Passed locally; verifies FR-065's behavior generally, not a specific AC |
| TC-378 | On a nested fixture, the node limit is the proximate cause of a function-declaration stop | Unit | P1 | FR-062-AC-7, FR-096-AC-11 | 🚧 Planned (ADR-030: node limit replaces the deleted depth limit) |
| TC-379 | E3 resolves an imported name to its PackageNodeKey and refuses a missing or ambiguous one | Unit | P1 | FR-087-AC-13 | 🚧 Steps 1 and 3 pass locally; steps 2 and 4 planned |
| TC-380 | The function-declaration contract check refuses an ill-typed declaration and admits a well-typed one | Unit | P1 | FR-065-AC-7 | ✅ Passed locally |
| TC-381 | The expression-node limit bounds the whole checked package, not each declaration | Unit | P1 | FR-062-AC-11 | ✅ Passed locally: the whole-package count refuses as `ResourceExhausted` with the node-count stage-limit cause, code `stage_limit_exceeded` |
| TC-382 | S6a returns each kernel outcome unchanged in FamilyOutcome::Evaluated | Unit | P1 | FR-090-AC-1 | ✅ Passed locally |
| TC-384 | S6a invariant breaks are InternalFaults, not panics or refusals | Unit | P1 | FR-090-AC-3 | ✅ Passed locally |
| TC-385 | S6a's input type admits no Relation, and FamilyOutcome has exactly two arms | Unit | P1 | FR-090-AC-4 | ✅ Passed locally |
| TC-386 | F diagnostic maps every snapshot-cause and model-refusal catalog code to category refusal | Unit | P1 | FR-090-AC-5 | ✅ Passed locally |
| TC-387 | The ProtocolClause snapshot cause maps each WrongSnapshotCause to wrong_snapshot | Unit | P1 | FR-090-AC-6 | ✅ Passed locally |
| TC-388 | An evaluation-time wrong-anchor snapshot reaches the caller as a coded QSL refusal, not a kernel refusal | Integration | P1 | FR-090-AC-7 | ✅ Passed locally |
| TC-389 | A refused model query reaches the caller with the ModelRefusal's own catalog code, not a kernel refusal | Integration | P1 | FR-090-AC-8 | ✅ Passed locally |
| TC-390 | FamilyOutcome, FamilyResult and EvalOutcome live once in the check core, no lower layer names them, and the check core names no family cause | Unit | P1 | FR-090-AC-9 | ✅ Passed locally |
| TC-391 | An unresolved or mismatched population argument is refused at admission, and is an InternalFault inside S6a | Unit | P1 | FR-090-AC-10 | ✅ Passed locally |
| TC-392 | S2 returns one Value form per declaration, in source order, with its span and the unit edition | Unit | P1 | FR-091-AC-1 | ✅ Passed locally (`qsl-forms/tests/it/value_forms.rs`) |
| TC-393 | The forms FunctionDeclaration carries its name, using alias, type forms, measure and body | Unit | P1 | FR-091-AC-2 | ✅ Passed locally (`qsl-forms/tests/it/value_forms.rs`) |
| TC-394 | Each Value expression construct maps to its Expression variant, with grouping from the CST | Unit | P1 | FR-091-AC-3 | ✅ Passed locally (`qsl-forms/tests/it/value_forms.rs`) |
| TC-395 | S2 refuses an inadmissible source and a unit holding a declaration with no dispatch entry | Unit | P1 | FR-091-AC-4, FR-091-AC-5, FR-091-AC-6 | ✅ Passed locally (`qsl-forms/tests/it/value_forms.rs`) |
| TC-396 | S2 refuses unrepresented constructs, and the check stage refuses another family's construct with that family's cause | Integration | P1 | FR-091-AC-7, FR-091-AC-8 | ✅ Passed locally (`qsl-forms/tests/it/value_forms.rs`, `qsl-semantics/src/check/assemble/tests.rs`) |
| TC-397 | S2 builds every body S1 admits, whatever its depth | Unit | P1 | FR-091-AC-9 | 🚧 Planned (ADR-030: S2 depth limit deleted) |
| TC-398 | The Value form builder depends only on layer 2, layer 1, F and K, and its forms hold no ValueType or NodeKey | Unit | P1 | FR-091-AC-11 | ✅ Passed locally: step 1's crate edges (`tests/it/family_outcome_layering.rs`) and step 2 over the existing forms types (`qsl-forms/tests/it/identity_free_forms.rs`) pass locally; step 3's `_`-arm scan passes locally (`identity_free_forms.rs`); step 1's family-module edges pass locally (`identity_free_forms.rs`) |
| TC-399 | Source compiled through S1, S2 and the assembler checks and evaluates a called function | Integration | P1 | FR-091-AC-12, FR-091-AC-13 | ✅ Passed locally (`qsl-eval/tests/it/source_call.rs`, `qsl-semantics/src/check/assemble/tests.rs`) |
| TC-400 | The assembler refuses unresolved and ambiguous names, ill-formed bounds and alias cycles, reporting every error | Unit | P1 | FR-091-AC-14, FR-091-AC-15, FR-091-AC-16, FR-091-AC-17 | ✅ Passed locally (`qsl-semantics/src/check/assemble/tests.rs`) |
| TC-401 | The assembler builds record and tuple declarations with check-minted keys and resolves names to them | Unit | P1 | FR-091-AC-18 | 🚧 Partial: passes locally (`qsl-semantics/src/check/assemble/tests.rs`); source owner from FR-001's `RawSourceRef` |
| TC-402 | The assembler lives in the check core and its non-test code has no edge to qsl-cst | Unit | P1 | FR-091-AC-20 | ✅ Passed locally (`qsl-semantics/src/check/assemble/tests.rs`) |
| TC-403 | Every Value Expression node carries the span of its CST node | Unit | P1 | FR-091-AC-10 | ✅ Passed locally (`qsl-forms/tests/it/value_forms.rs`) |
| TC-404 | format takes the qsl-cst ParsedSource, formats complete-V1 source and refuses inadmissible input | Unit | P1 | FR-003-AC-7, FR-003-AC-8 | ✅ Passed locally |
| TC-405 | The assembler admits floating types and refuses unresolved model references | Unit | P1 | FR-091-AC-19, FR-091-AC-23, FR-091-AC-24 | ✅ Passed locally |
| TC-406 | Each S2 and assembler cause maps to its catalog code with an exhaustive match | Unit | P1 | FR-091-AC-21 | 🚧 Partial: S2 and assembler causes pass locally (`qsl-forms/tests/it/value_forms.rs`, `qsl-semantics/src/check/assemble/tests.rs`); `stage_limit_exceeded` code needs S-5b; the enum, dimension and unit causes pass locally; the topology cause STD-112; depth steps re-planned for ADR-030 |
| TC-407 | A false dispatched precondition reaches the caller as a family-owned undefined result, not a kernel Undefined | Integration | P1 | FR-090-AC-11 | ✅ Passed locally |
| TC-408 | An absent lookup key reaches the caller as a StateModel undefined result, and an absent-refused lookup as a refusal | Integration | P1 | FR-090-AC-12 | ✅ Passed locally |
| TC-409 | An enum value's VariantId is its FR-141 member node key, and its rank orders sets and bags | Unit | P1 | FR-088-AC-11 | ✅ Passed locally; steps 2-6 via retagged tests, step 7 new |
| TC-410 | Each connected supertype component has its own object universe, and a reference key carries the authored object identity | Unit | P1 | FR-084-AC-7 | 🚧 Planned |
| TC-411 | A quantity UnitId is a declared unit's node key or a compound unit's digest, and the two never compare equal | Unit | P1 | FR-088-AC-12 | ✅ Passed locally; step 3 runs under `make conformance` |
| TC-412 | The assembler resolves each using alias to a declared profile selection and refuses an undeclared one | Unit | P1 | FR-091-AC-22 | ✅ Passed locally |
| TC-413 | Type and declared record nodes key to the structural-node golden vectors, scoped only by owner | Unit | P1 | FR-092-AC-1, FR-092-AC-2, FR-092-AC-3, FR-092-AC-7, FR-092-AC-8, FR-092-AC-9, FR-092-AC-11, FR-092-AC-12 | ✅ Passed locally (#384); depth steps re-planned for ADR-030 |
| TC-414 | Parameter, literal and function nodes key to the golden vectors, and a function key carries its owner | Unit | P1 | FR-092-AC-4, FR-092-AC-5, FR-092-AC-6, FR-092-AC-10 | ✅ Passed locally (#384) |
| TC-415 | Each checked Value expression lowers to its FR-322 node with its catalogued operation | Unit | P1 | FR-093-AC-1, FR-093-AC-2, FR-093-AC-3, FR-093-AC-4, FR-093-AC-5, FR-093-AC-6, FR-093-AC-8, FR-093-AC-10, FR-093-AC-11, FR-093-AC-14, FR-093-AC-15 | 🚧 Passed locally (#384) except step 6's first half, which needs the QSpec lock accessor; depth steps re-planned for ADR-030 |
| TC-416 | The v2 emission arm writes the nodes check lowered, and each emitted node recomputes to its node id | Integration | P1 | FR-093-AC-7, FR-093-AC-9, FR-093-AC-12, FR-093-AC-13, FR-093-AC-16, FR-093-AC-17, FR-093-AC-18, FR-093-AC-19, FR-093-AC-20 | 🚧 Partial; FR-093-AC-13 passes locally under `make conformance`, `float64.add` `mode` included; FR-093-AC-16 passes locally; FR-093-AC-17 passes locally; FR-093-AC-18 passes locally; FR-093-AC-19 passes locally; FR-093-AC-20 passes locally |
| TC-417 | Model declaration, Reference and Population nodes key to the golden vectors under ModelOwner | Unit | P1 | FR-094-AC-1, FR-094-AC-2, FR-094-AC-3, FR-094-AC-4, FR-094-AC-7 | ✅ Passed locally (#384) |
| TC-418 | Clause function nodes key to the golden vectors with the operation member's ModelOwner | Unit | P1 | FR-094-AC-5 | ✅ Passed locally (#384) |
| TC-419 | A declared unit's quantity type is its unit node, and a compound unit's keys to the golden vectors | Unit | P1 | FR-094-AC-6, FR-094-AC-7 | ✅ Passed locally (#384); QSpec unit vectors under `make conformance` |
| TC-420 | Occurrence keys and source regions are lexical values, and every checked node has an occurrence | Unit | P1 | FR-095-AC-1, FR-095-AC-2 | ✅ Passed locally |
| TC-421 | The package source map carries the wire's source map, and a location resolves or refuses by cause | Integration | P1 | FR-095-AC-3, FR-095-AC-4 | ✅ Passed locally; step 4 (QSpec positive fixtures) under `make conformance` |
| TC-422 | Each Locus variant resolves to regions by its own rule, and the artifact pointer is RFC 6901 | Unit | P1 | FR-095-AC-5, FR-095-AC-6 | ✅ Passed locally |
| TC-423 | The default checking ceilings bind wide and long leaf lists, admit large enum packages, and are recorded with the result | Unit | P1 | NFR-011-M-1 (node refusal: step 1), NFR-011-M-2 (step 3), NFR-011-M-3 (byte refusal before work: step 4), NFR-011-M-4 (work refusal: step 5; enum package admitted: step 4) | ✅ Passed locally: ceilings bind and are recorded, and each refusal asserts `stage_limit_exceeded` with its limit's cause; depth steps re-planned for ADR-030 |
| TC-424 | An admitted source carries the source reference its caller named, and its node keys ignore the content digest | Unit | P1 | FR-001-AC-5, FR-001-AC-6, FR-001-AC-7, FR-001-AC-8, FR-001-AC-9, FR-001-AC-10, FR-001-AC-11, FR-001-AC-12 | 🚧 Partial: the four-label form passed locally (S-4b); the two-label form waits on the reader change (FR-001 Status) |
| TC-425 | parse and format take the two source labels and report the source reference | Integration | P1 | FR-010-AC-11 | 🚧 Partial: the four-label form passed locally (S-4b); the two-label form waits on the CLI change (FR-010 Status) |
| TC-426 | A check location resolves to the region of the unit it was read from, or to none | Unit | P1 | FR-096-AC-1 | ✅ Passed locally (`qsl-semantics/src/check/region.rs`: steps 2 to 4) |
| TC-427 | A stage limit names its kind, bound, actual counter and locus | Unit | P1 | FR-096-AC-2, FR-096-AC-3, FR-096-AC-4, FR-096-AC-5, FR-096-AC-16, FR-096-AC-17 | ✅ Passed locally (S-5b); AC-16 (lowering's own work charge located at a node) and AC-17 (Typer's package-wide node count reached from a second declaration) added; depth steps re-planned for ADR-030 |
| TC-428 | A refusal record carries its code, category, locus and the catalog's fields | Unit | P1 | FR-096-AC-6, FR-096-AC-7, FR-096-AC-8, FR-096-AC-13, FR-096-AC-15 | ✅ Passing locally: steps 1 to 6; all twelve kernel refusals with a record carry their target domain or width and `Refusal::code()`/`cause()` return the key table's code and cause (`ForeignReference` included); a kernel `CheckedInvariant` builds no record and `Machine::run` returns it as `Err(InternalFault)` |
| TC-429 | The I2 reader locates its version refusal and its limits in the artifact | Integration | P1 | FR-096-AC-9, FR-096-AC-10, FR-096-AC-18 | ✅ Passed locally (S-5b), on IR-281 |
| TC-430 | Native run and compile requests and their outputs carry the two source labels | Integration | P1 | FR-026-AC-6, FR-027-AC-4, FR-031-AC-5 | 🚧 Partial: steps 1 to 4 pass locally in the four-label form (S-4b), and their two-label form waits on the request and output identity change (QSL-381); step 5 planned until the body record code moves to `{authority, identity, document, digest}` (QSL-381) |
| TC-431 | Runtime input artifacts carry the two labels, and bytes missing one refuse | Unit | P1 | FR-018-AC-8, FR-024-AC-6 | 🚧 Partial: the four-label form passed locally (S-4b); the two-label form waits on the artifact identity change |
| TC-432 | A family check's stage limit names its kind, bound and actual counter, and the counter is where the limit stops | Unit | P1 | FR-062-AC-12 | ✅ Passed locally; depth steps re-planned for ADR-030 |
| TC-434 | Model limits have finite defaults, and normalization work stops at the limit | Unit | P1 | NFR-012 (defaults and recording: steps 1 and 2; bounded work: steps 3 and 4; meter memory: step 5; shared paths: step 6; ancestor-steps order: step 7) | ✅ Passed locally |
| TC-436 | Proof-bound and interval-key constructors refuse empty ranges, and domain keys order by node then path | Unit | P1 | FR-097-AC-1 | ✅ Passed locally |
| TC-437 | The extent rule names each unbounded type position once, by node and path, under a node-count ceiling | Unit | P1 | FR-097-AC-2 | ✅ Passed locally |
| TC-438 | The request writer computes the available finite bound, writes a bounded request as its own item, and refuses bad bounds before writing | Unit | P1 | FR-097-AC-3, FR-097-AC-4 | ✅ Passed locally |
| TC-439 | An exploration outcome maps to its O-16 category and keeps its frontier | Unit | P1 | FR-097-AC-5 | ✅ Passed locally |
| TC-440 | QSL's extent agrees with IR's requires-bound at the pinned IR revision | Integration | P1 | FR-097-AC-6 | ✅ Passed locally: every fixture agrees, the quantity fixture `Measure` included (a recursive type's first unbounded node in the recursion group QSL names), and step 4 passes |
| TC-441 | An unbounded collection never refuses for cardinality and stops only on the caller's meter | Unit | P1 | FR-097-AC-7, FR-097-AC-8 | ✅ Passed locally; step 5 checks the interim `UnrepresentableBound` lowering refusal until QSL-42 |
| TC-453 | Exploration orders successors canonically and keys states by their JCS bytes | Unit | P1 | FR-101-AC-1, FR-101-AC-2, FR-101-AC-9 | ✅ Passed locally |
| TC-454 | The sampler reproduces its vectors, and sampled traces replay | Unit | P1 | FR-101-AC-3, FR-101-AC-4, FR-101-AC-5, FR-101-AC-10 | ✅ Passed locally |
| TC-455 | Stopped explorations stay incomplete, and unbounded requests require a bound | Unit | P1 | FR-101-AC-6, FR-101-AC-7, FR-101-AC-8 | ✅ Passed locally |
| TC-442 | Spine compile admits a domain package and locks its model selection | Integration | P1 | FR-027-AC-9, FR-056-AC-9 | ✅ Passed locally |
| TC-443 | A model field's multiplicity and presence give its assembled value type | Unit | P1 | FR-056-AC-10 | ✅ Passed locally |
| TC-444 | The replay executor recompiles, selects, calls, and refuses each O-26 case | Unit | P1 | FR-098-AC-1, FR-098-AC-2, FR-098-AC-3, FR-098-AC-4, FR-098-AC-5, FR-098-AC-6, FR-098-AC-7 | ✅ Passed locally for steps 1 to 6, step 6 being a predicate whose body calls another declared function (the QSL-22 Layer 3 exemplar's shape); step 7 (FR-098-AC-6, FR-098-AC-7, replay against dependencies) passes locally; step 5's whitespace-only authority passes with `invalid_source_identity`, cause `blank-label` and `label` `authority` |
| TC-446 | Spine compile resolves imports against supplied libraries | Integration | P1 | FR-099-AC-1, FR-099-AC-2, FR-099-AC-3, FR-099-AC-4, FR-099-AC-5, FR-099-AC-6, FR-099-AC-7, FR-027-AC-10 | ✅ Steps 1 to 8 pass locally (step 5's `g::f(3)` result amended) |
| TC-490 | E3 resolves header profile selections against the DefinitionLock catalog | Integration | P1 | FR-110-AC-1, FR-110-AC-2, FR-110-AC-3, FR-110-AC-5, FR-110-AC-6, FR-110-AC-7, FR-110-AC-8, FR-110-AC-9 | 🚧 Partial; FR-110-AC-9 passes locally under `make conformance`; step 4 waits on the code change; step 1's layer-closure rows and steps 6-7 (layer selection) planned |
| TC-491 | library::bundle links a complete-V1 bundle and refuses each closure, facet and limit defect | Integration | P1 | FR-111-AC-1, FR-111-AC-2, FR-111-AC-3, FR-111-AC-4, FR-111-AC-5, FR-111-AC-6, FR-111-AC-7 | ✅ Passed locally: `library::bundle_tests`, keeping the `QSpec-FR-131`/`QSpec-FR-339` tags |
| TC-450 | CLI run routes a program by its declared edition and calls a 1-draft function | Integration | P1 | FR-100-AC-1, FR-100-AC-2, FR-100-AC-3 | 🚧 Partial: passed locally with the four-label `source`; the two-label `source` waits on FR-001's reader change |
| TC-451 | Spine run binds arguments by name and maps each outcome and refusal to its exit code | Integration | P1 | FR-100-AC-4, FR-100-AC-5, FR-100-AC-6 | ✅ Passed locally |
| TC-452 | The spine run entry is qsl_replay::spine::run, agrees with the CLI, and maps every outcome | Unit | P1 | FR-100-AC-7, FR-100-AC-8, FR-100-AC-9 | ✅ Passed locally |
| TC-456 | S2 builds state clause forms and the self, result and reaches expressions | Unit | P1 | FR-102-AC-1, FR-102-AC-2, FR-102-AC-3 | 🚧 Planned |
| TC-457 | S2 state clause dispatch is thin, bounded and seam-probed | Unit | P1 | FR-102-AC-4, FR-102-AC-5, FR-102-AC-6 | 🚧 Planned |
| TC-458 | Spine intake and assembly admit operations and frames | Integration | P1 | FR-103-AC-1, FR-103-AC-2, FR-103-AC-3, FR-103-AC-4, FR-103-AC-5 | 🚧 Planned |
| TC-459 | S3 checks the ConfigVersion state clauses and types self, result and pre | Unit | P1 | FR-104-AC-1, FR-104-AC-2, FR-104-AC-7, FR-104-AC-8 | ✅ Passed locally; AC-1's `Int[0, 1000]` half is verified over the shared `ConfigVersion` fixture itself |
| TC-460 | S3 refuses ill-formed state clauses with their catalog codes | Unit | P1 | FR-104-AC-3, FR-104-AC-4 | ✅ Passed locally |
| TC-461 | S3 records one operation-contract requirement per state clause and frame | Unit | P1 | FR-104-AC-5, FR-104-AC-6 | ✅ Passed locally |
| TC-462 | S4 emits state clause, operation anchor and frame nodes with their bodies | Integration | P1 | FR-105-AC-1, FR-105-AC-2, FR-105-AC-5 | ✅ Covered |
| TC-463 | The state package reads back through I2, keeps its identity rules and emits all or nothing | Integration | P1 | FR-105-AC-3, FR-105-AC-4, FR-105-AC-6 | ✅ Passed locally; step 1 covered; steps 2 to 3 covered; step 4 covered |
| TC-464 | Snapshot and invocation documents read and admit into an observation set | Unit | P1 | FR-106-AC-1, FR-106-AC-2, FR-106-AC-6, FR-106-AC-8, FR-106-AC-9 | 🚧 Planned |
| TC-465 | Admission refuses or reports incomplete for each input defect, in check order | Unit | P1 | FR-106-AC-3, FR-106-AC-4, FR-106-AC-5, FR-106-AC-7, FR-106-AC-11, FR-106-AC-12, FR-106-AC-13 | 🚧 Planned |
| TC-466 | S6a evaluates state clauses over their observations, pre reads and reaches | Integration | P1 | FR-107-AC-1, FR-107-AC-2, FR-107-AC-3 | ✅ Passed locally |
| TC-467 | S6a clause entry refuses bad selections, reports exhaustion and is deterministic | Integration | P1 | FR-107-AC-4, FR-107-AC-5, FR-107-AC-6 | 🚧 Planned |
| TC-468 | The spine clause run entry reports typed dispositions and exit codes | Integration | P1 | FR-109-AC-1, FR-109-AC-2, FR-109-AC-3, FR-109-AC-4, FR-109-AC-5, FR-109-AC-6, FR-109-AC-7 | 🚧 Planned |
| TC-469 | The ConfigVersion spine corpus gives native-equal typed dispositions | Integration | P1 | FR-108-AC-1, FR-108-AC-2, FR-108-AC-3, FR-108-AC-4, FR-108-AC-5, FR-108-AC-6 | ✅ Passed locally |
| TC-470 | runtime_invariant exits 30 and outranks other diagnostics | Unit | P1 | FR-096-AC-12 | ✅ Implemented |
| TC-471 | Model successors follow operations, arguments, frames and contracts | Integration | P1 | FR-120-AC-1, FR-120-AC-2, FR-120-AC-3, FR-120-AC-4 | 🚧 Planned |
| TC-472 | Invariant-violating successors are recorded, and undecided expansions stop the run | Integration | P1 | FR-120-AC-5, FR-120-AC-6, FR-120-AC-7, FR-120-AC-8, FR-120-AC-9, FR-120-AC-13 | 🚧 Planned; ADR-016 G-4 |
| TC-473 | Model effects and results are trace data, and ambient-state reads refuse at S3 | Integration | P1 | FR-120-AC-10, FR-120-AC-11, FR-120-AC-12 | 🚧 Planned |
| TC-474 | The engine records findings, stops on an expansion stop, and replays a stopped trace | Integration | P1 | FR-101-AC-12, FR-101-AC-13, FR-101-AC-14, FR-097-AC-5 | ✅ Passed locally |
| TC-480 | S2 builds enum and predicate forms | Unit | P1 | FR-091-AC-25, FR-091-AC-26 | ✅ Passed locally (`qsl-forms/tests/it/value_forms.rs`) |
| TC-481 | The assembler admits source enums and predicates, which check and lowering then use | Integration | P1 | FR-091-AC-27, FR-091-AC-28, FR-091-AC-29, FR-091-AC-30, FR-092-AC-13 | ✅ Passed locally (`qsl-semantics` `check::assemble` tests, `qsl-eval/tests/it/source_call.rs`) |
| TC-482 | S2 builds dimension and unit forms | Unit | P1 | FR-091-AC-31 | ✅ Passed locally (`qsl-forms/tests/it/value_forms.rs`) |
| TC-483 | The assembler admits source dimensions and units into a UnitGraph and refuses each source error | Unit | P1 | FR-091-AC-32, FR-091-AC-33, FR-091-AC-34, FR-091-AC-35 | 🚧 Partial: AC-32 to AC-34 and AC-35's errors pass locally (`qsl-semantics` `check::assemble` tests); AC-35's catalog code awaits STD-112 |
| TC-500 | A sum seed or running total outside its domain is a located undefined outcome | Unit | P1 | FR-096-AC-14 | ✅ Passing locally: `Undefined::SumOutOfDomain` at the `sum` node for a running total and at the summand node for a seed, no charge after the failed decision |
| TC-510 | S2 builds protocol scoped anchor forms with their scope and segments | Unit | P1 | FR-112-AC-1, FR-112-AC-2, FR-112-AC-3 | 🚧 Planned |
| TC-511 | S3 resolves scoped anchors through nested scopes and refuses a missing anchor or member | Unit | P1 | FR-113-AC-1, FR-113-AC-2, FR-113-AC-3 | 🚧 Planned |
| TC-512 | S3 refuses ambiguous, shadowing, wrong-kind and wrong-channel names, in builder order | Unit | P1 | FR-113-AC-4, FR-113-AC-5, FR-113-AC-6, FR-113-AC-7 | 🚧 Planned |
| TC-513 | S3 binds a protocol attempt to its operation's one anchor and frame | Integration | P1 | FR-114-AC-1, FR-114-AC-2, FR-114-AC-3, FR-114-AC-4 | 🚧 Planned; emitted-node assertions pending STD-111 |
| TC-514 | The spine run entry checks an invocation against its operation frame | Integration | P1 | FR-115-AC-1, FR-115-AC-2, FR-115-AC-3, FR-115-AC-4, FR-115-AC-5, FR-115-AC-6 | ✅ Passed locally |
| TC-515 | The replay facade replays a frame counterexample and keeps its identities | Integration | P1 | FR-116-AC-1, FR-116-AC-2, FR-116-AC-3, FR-116-AC-4, FR-116-AC-5, FR-116-AC-6 | ✅ Passed locally |
| TC-516 | call_site names a function's parameters and an operation's and a state clause's identities, matching what replay accepts | Unit | P1 | FR-121-AC-1, FR-121-AC-2, FR-121-AC-3, FR-121-AC-4, FR-121-AC-5, FR-121-AC-6, FR-121-AC-7, FR-121-AC-8, FR-121-AC-9, FR-121-AC-10, FR-121-AC-11, FR-121-AC-12, FR-121-AC-13, FR-121-AC-14, FR-121-AC-15, FR-121-AC-16, FR-121-AC-17, FR-121-AC-18, FR-121-AC-19, FR-121-AC-20, FR-121-AC-21, FR-121-AC-22 | ✅ Passed locally |
| TC-517 | The replay facade replays a state-clause counterexample and keeps its identities | Integration | P1 | FR-122-AC-1, FR-122-AC-2, FR-122-AC-3, FR-122-AC-4, FR-122-AC-5, FR-122-AC-6, FR-122-AC-7 | 🚧 Planned |
| TC-740 | S6a stop reports and the decision path derive a state clause's basis and witness | Unit | P1 | FR-265-AC-1, FR-265-AC-2, FR-265-AC-3, FR-265-AC-4, FR-265-AC-5, FR-265-AC-6 | 🚧 Planned |
| TC-741 | Clause run reports carry a settlement basis on every disposition and a witness only when decisive | Integration | P1 | FR-266-AC-1, FR-266-AC-2, FR-266-AC-3 | 🚧 Planned |
| TC-745 | The checked-input gate rejects pre-check signatures and reconstruction and passes the workspace | Unit | P1 | FR-270-AC-1, FR-270-AC-2, FR-270-AC-3, FR-270-AC-4 | ✅ Passed locally (`xtask/src/checked_input.rs`); AC-4's `ci:` wiring by inspection |
| TC-746 | Canonical doc tags define the canonical set, and misplaced or duplicate tags fail | Unit | P1 | FR-271-AC-1, FR-271-AC-2 | ✅ Passed locally (`xtask/src/canonical_types.rs`) |
| TC-747 | The canonical-types gate finds namesakes, re-exports and same-shaped copies | Unit | P1 | FR-272-AC-1, FR-272-AC-2, FR-272-AC-3, FR-272-AC-4 | ✅ Passed locally (`xtask/src/canonical_types.rs`) |
| TC-748 | Once every namesake is deleted or renamed, the canonical-types gate and make ci pass | Integration | P1 | FR-273-AC-1 | 🚧 Planned |
| TC-790 | Each model form types through StateModel and refuses with a StateModel cause | Integration | P1 | FR-300-AC-1, FR-300-AC-2 | 🚧 Planned |
| TC-791 | The seam probe reports the StateModel arms at S1, S2 and S3 | Integration | P1 | FR-300-AC-3 | 🚧 Planned |
| TC-792 | StateModel causes raised under Value and ProtocolClause evaluation render with the state-model prefix | Integration | P1 | FR-301-AC-1, FR-301-AC-2 | 🚧 Planned |
| TC-793 | Unit compile carries one linked dispatch table per called operation, named by each call | Integration | P1 | FR-302-AC-1 | 🚧 Planned |
| TC-794 | Unit compile refuses ambiguous and inapplicable dispatch with no checked package | Integration | P1 | FR-302-AC-2, FR-302-AC-3, FR-302-AC-4 | 🚧 Planned |
| TC-795 | Unit compile refuses a dispatch family past the caller's family_steps limit | Integration | P1 | FR-302-AC-5 | 🚧 Planned |
| TC-796 | The model correspondence faults on a second, different entry in either direction | Integration | P1 | FR-303-AC-1, FR-303-AC-2, FR-303-AC-3, FR-303-AC-4 | ✅ Passed locally; AC-4 at the S3 lowering, no spine-level seam |
| TC-797 | An abstraction relation checks into one binding per key, total or partial | Unit | P1 | FR-304-AC-1, FR-304-AC-2 | 🚧 Planned |
| TC-798 | An abstraction binding whose model key resolves to nothing refuses missing-name | Unit | P1 | FR-304-AC-3 | 🚧 Planned |
| TC-799 | Duplicate and conflicting abstraction bindings refuse conflicting-binding | Unit | P1 | FR-304-AC-4 | 🚧 Planned |
| TC-800 | Abstraction bindings with malformed parameter or field maps refuse malformed-declaration | Unit | P1 | FR-304-AC-5 | 🚧 Planned |
| TC-801 | RustPath, RustField and RustReceiver segments are checked against Rust syntax | Unit | P1 | FR-304-AC-6 | 🚧 Planned |
| TC-802 | A frame binding checks without a naming clause and relates to the frame and anchor when one exists | Integration | P1 | FR-305-AC-1, FR-305-AC-2 | 🚧 Planned |
| TC-803 | An inherited operation's frame binds only at its declaring type | Unit | P1 | FR-305-AC-3 | 🚧 Planned |
| TC-804 | The abstraction relation is emitted as a v2 node and enters the package_id | Integration | P1 | FR-306-AC-1, FR-306-AC-2 | 🚧 Planned |
| TC-805 | The abstraction relation changes no requirement record, route result or clause run | Integration | P1 | FR-306-AC-3, FR-306-AC-4 | 🚧 Planned |
| TC-806 | The export refuses each item with an unbound element and returns the others | Integration | P1 | FR-307-AC-1, FR-307-AC-2, FR-307-AC-3 | 🚧 Planned |
| TC-807 | The export references the receiver's static type and the operation key | Integration | P1 | FR-307-AC-4, FR-307-AC-5 | 🚧 Planned |
| TC-809 | Evaluation selects the dispatched body from the linked table by the receiver's most-specific type | Integration | P1 | FR-302-AC-6 | 🚧 Planned |
| TC-810 | Abstraction binding keys are unique across all of a unit's declarations | Integration | P1 | FR-304-AC-7 | 🚧 Planned |
| TC-811 | Abstraction declarations parse from source and refuse unsupported or malformed forms | Integration | P1 | FR-304-AC-8 | 🚧 Planned |
| TC-518 | S3 checks fairness constraints and interval operators of infinite-trace clauses | Unit | P1 | FR-123-AC-1, FR-123-AC-2, FR-123-AC-3, FR-123-AC-4 | 🚧 Planned |
| TC-519 | Terminal declarations check, and the request carries one deadlock-freedom item per subject | Unit | P1 | FR-124-AC-1, FR-124-AC-2, FR-124-AC-3 | 🚧 Planned |
| TC-520 | A model subject's behaviours read as temporal traces, with terminal stutter and interval wrap | Integration | P1 | FR-125-AC-1, FR-125-AC-2, FR-125-AC-3, FR-125-AC-4, FR-125-AC-5 | 🚧 Planned |
| TC-521 | The explicit-state model checker proves, refutes and stops over model subjects | Integration | P1 | FR-126-AC-1, FR-126-AC-2, FR-126-AC-3, FR-126-AC-4, FR-126-AC-5, FR-126-AC-6, FR-126-AC-7 | 🚧 Planned |
| TC-522 | Model-check outcomes settle as QSpec FR-331 terminal records with their strength | Unit | P1 | FR-127-AC-1, FR-127-AC-2, FR-127-AC-3, FR-127-AC-4, FR-127-AC-5, FR-127-AC-7, FR-127-AC-8, FR-127-AC-9, FR-127-AC-10 | 🚧 Planned |
| TC-523 | The replay facade replays a model counterexample through ModelSystem | Integration | P1 | FR-128-AC-1, FR-128-AC-2, FR-128-AC-3, FR-128-AC-4 | 🚧 Planned |
| TC-524 | A checked temporal clause emits as a v2 temporal clause node and reads back | Integration | P1 | FR-337-AC-1, FR-337-AC-2, FR-337-AC-3, FR-337-AC-4 | 🚧 Planned |
| TC-525 | An EN-1 closure certificate is accepted or rejected by the core checker | Unit | P1 | FR-338-AC-1, FR-338-AC-2, FR-338-AC-3 | 🚧 Planned |
| TC-526 | An EN-1 component certificate is accepted or rejected by the core checker | Unit | P1 | FR-339-AC-1, FR-339-AC-2 | 🚧 Planned |
| TC-893 | An SMT proof certificate is accepted or rejected by the core checker | Unit | P1 | FR-314-AC-1, FR-314-AC-2 | 🚧 Planned |
| TC-900 | The SMT-LIB transition-relation encoding is canonical and refuses unencodable constructs | Unit | P1 | FR-315-AC-1, FR-315-AC-2, FR-315-AC-3, FR-315-AC-4, FR-315-AC-5 | 🚧 Planned |
| TC-901 | The quire fences of spec artifacts are checked against the objects they declare | Unit | P1 | FR-355-AC-1, FR-355-AC-2, FR-355-AC-3, FR-355-AC-4, FR-355-AC-5, FR-355-AC-6 | 🚧 Planned |
| TC-530 | S3 checks strong fairness constraints and the unmarked fairness kind | Unit | P1 | FR-129-AC-1, FR-129-AC-2 | 🚧 Planned |
| TC-531 | The explicit-state model checker decides strong fairness by SCC refinement | Integration | P1 | FR-130-AC-1, FR-130-AC-2, FR-130-AC-3, FR-130-AC-4, FR-130-AC-5 | 🚧 Planned |
| TC-532 | Replay checks strong fairness on a model counterexample | Integration | P1 | FR-131-AC-1, FR-131-AC-2, FR-131-AC-3 | 🚧 Planned |
| TC-533 | A fairness constraint over a supplied trace settles as a missing premise | Unit | P1 | FR-132-AC-1, FR-132-AC-2 | 🚧 Planned |
| TC-534 | A refuted liveness record names the strong constraint that would exclude its lasso | Integration | P1 | FR-133-AC-1, FR-133-AC-2, FR-133-AC-3 | 🚧 Planned |
| TC-535 | Candidates carry and are filtered by the fairness kinds their backends advertise | Unit | P1 | FR-134-AC-1, FR-134-AC-2 | 🚧 Planned |
| TC-536 | Exploration and model-check limits publish their defaults | Unit | P1 | FR-101-AC-15, FR-126-AC-8 | 🚧 FR-101-AC-15 passes locally; FR-126-AC-8 not yet implemented |
| TC-537 | An undefined claim evaluation refutes in the explicit-state model checker | Integration | P1 | FR-125-AC-6, FR-126-AC-9 | 🚧 Planned |
| TC-538 | An undefined-evaluation counterexample settles refuted with its cause | Unit | P1 | FR-127-AC-6 | 🚧 Planned |
| TC-539 | Replay reproduces an undefined claim evaluation at its position | Integration | P1 | FR-128-AC-5, FR-128-AC-6 | 🚧 Planned |
| TC-835 | S2 parses temporal operators with an optional interval, independent of profile | Unit | P1 | FR-325-AC-1, FR-325-AC-2, FR-325-AC-3, FR-325-AC-4 | 🚧 Planned |
| TC-836 | S3 admits temporal operators by the unit's temporal profile and records the requirement | Unit | P1 | FR-326-AC-1, FR-326-AC-2, FR-326-AC-3, FR-326-AC-4, FR-326-AC-5 | 🚧 Planned |
| TC-837 | S6a evaluates a temporal clause over a finite trace with typed positions and metered work | Unit | P1 | FR-327-AC-1, FR-327-AC-2, FR-327-AC-3, FR-327-AC-4 | 🚧 Planned |
| TC-838 | S6a evaluates an infinite-trace clause three-valued over a finite prefix | Unit | P1 | FR-328-AC-1, FR-328-AC-2, FR-328-AC-3, FR-328-AC-4 | 🚧 Planned |
| TC-839 | S6a evaluates an infinite-trace clause exactly over a fair lasso | Unit | P1 | FR-329-AC-1, FR-329-AC-2, FR-329-AC-3, FR-329-AC-4, FR-329-AC-5 | 🚧 Planned |
| TC-840 | run_clause runs a selected temporal clause over a supplied trace | Integration | P1 | FR-330-AC-1, FR-330-AC-2, FR-330-AC-3, FR-330-AC-4 | 🚧 Planned |
| TC-841 | The replay facade replays a temporal counterexample over an observed trace | Integration | P1 | FR-331-AC-1, FR-331-AC-2, FR-331-AC-3, FR-331-AC-5 | 🚧 Planned |
| TC-842 | An infinite-trace item settles only through negotiation | Integration | P1 | FR-332-AC-1, FR-332-AC-2, FR-332-AC-3, FR-332-AC-4 | 🚧 Planned |
| TC-843 | Collection and population types resolve with an optional bound | Unit | P1 | FR-333-AC-1, FR-333-AC-2, FR-333-AC-3 | 🚧 Planned |
| TC-844 | Collection and population nodes are keyed by the root definitions' identity preimage | Integration | P1 | FR-334-AC-1, FR-334-AC-2, FR-334-AC-3, FR-334-AC-4 | 🚧 Planned |
| TC-845 | A claim over an unbounded declaration settles end to end | Integration | P1 | FR-335-AC-1, FR-335-AC-2, FR-335-AC-3, FR-335-AC-4, FR-335-AC-5 | 🚧 Planned |
| TC-846 | A model subject is finite by its universes, apart from proof bounds | Integration | P1 | FR-336-AC-1, FR-336-AC-2, FR-336-AC-3, FR-336-AC-4 | 🚧 Planned |
| TC-847 | An undefined letter fails a clause over a supplied trace or lasso, and replays | Integration | P1 | FR-327-AC-5, FR-328-AC-5, FR-329-AC-6, FR-330-AC-5, FR-331-AC-4 | 🚧 Planned |
| TC-848 | A fairness premise over a supplied trace is missing, and the clause settles unsupported | Integration | P1 | FR-328-AC-6, FR-329-AC-7, FR-330-AC-6 | 🚧 Planned |
| TC-860 | The spec-versioning gate reads each pair, refuses a malformed one, and runs every case against both revisions | Integration | P1 | FR-340-AC-1, FR-340-AC-2, FR-340-AC-3, FR-340-AC-4, FR-340-AC-5 | 🚧 Planned |
| TC-861 | Spine compile results classify into exactly one refinement class, reading every cause | Unit | P1 | FR-341-AC-1, FR-341-AC-2, FR-341-AC-3, FR-341-AC-4, FR-341-AC-5, FR-341-AC-6 | 🚧 Planned |
| TC-862 | Clause-run dispositions classify into one refinement class with their stage | Unit | P1 | FR-342-AC-1, FR-342-AC-2, FR-342-AC-3, FR-342-AC-4, FR-342-AC-5 | 🚧 Planned |
| TC-863 | The versioning comparison returns the table's result for every class pair | Unit | P1 | FR-343-AC-1 | 🚧 Planned |
| TC-864 | A seeded spec-versioning regression fails the gate naming exactly that case | Integration | P1 | FR-340-AC-6, FR-343-AC-2, FR-343-AC-3, FR-343-AC-4, FR-344-AC-5 | 🚧 Planned |
| TC-865 | An unsupported or incomplete superseding run is unresolved, never holds, and names its limit | Integration | P1 | FR-343-AC-5, FR-344-AC-4 | 🚧 Planned |
| TC-866 | The refinement report orders every result, lists every regression, and gives FR-301's verdict and exit | Unit | P1 | FR-344-AC-1, FR-344-AC-2, FR-344-AC-3, FR-344-AC-6, FR-344-AC-7 | 🚧 Planned |
| TC-867 | The layering comparison returns the table's result for every parent and child class | Unit | P1 | FR-345-AC-1, FR-345-AC-2, FR-345-AC-3 | 🚧 Planned |
| TC-868 | A seeded profile-layering regression fails the gate naming the case and its edge | Integration | P1 | FR-345-AC-4, FR-345-AC-5, FR-345-AC-6, FR-345-AC-7, FR-345-AC-8, FR-345-AC-9 | 🚧 Planned |
| TC-875 | The conformance runner executes vectors through public entry points and compares typed results | Integration | P1 | FR-350-AC-1, FR-350-AC-2 | 🚧 Planned |
| TC-876 | The conformance runner reports unsupported, incomplete and tool-failure vectors on their own | Integration | P1 | FR-350-AC-3, FR-350-AC-4, FR-350-AC-5 | 🚧 Planned |
| TC-877 | Each capability settles from its vectors in the stated precedence, and an empty capability is uncovered | Integration | P1 | FR-351-AC-1, FR-351-AC-2 | 🚧 Planned |
| TC-878 | Capability outcomes come only from vectors executed in the run, over the scope the caller selects | Integration | P1 | FR-351-AC-3, FR-351-AC-4 | 🚧 Planned |
| TC-879 | The conformance report gives outcome counts, coverage and each failing vector, in a stable order | Integration | P1 | FR-352-AC-1, FR-352-AC-2, FR-352-AC-3 | 🚧 Planned |
| TC-880 | The verdict is complete-V1 qualified only when every in-scope capability passes | Integration | P1 | FR-353-AC-1, FR-353-AC-2 | 🚧 Planned |
| TC-881 | A scoped run's verdict names its scope, and a refused run has no verdict | Integration | P1 | FR-353-AC-3, FR-353-AC-4 | 🚧 Planned |
| TC-882 | An extension's declaration form parses, checks against its typed-node schema and packages as a typed node | Unit | P1 | FR-354-AC-1, FR-354-AC-2 | 🚧 Planned |
| TC-883 | Malformed, colliding, cyclic and unselected extensions refuse with the definitions involved | Unit | P1 | FR-354-AC-3, FR-354-AC-4 | 🚧 Planned |
| TC-884 | An extension changes no other unit's identity, backends change no admission, and a reader without it refuses | Unit | P1 | FR-354-AC-5 | 🚧 Planned |
| TC-885 | Formatting keeps the checked package identity of every complete-V1 fixture unit | Integration | P1 | FR-003-AC-9 | 🚧 Planned |
| TC-650 | The protocol subject builds, refuses bad bindings and keys states canonically | Unit | P1 | FR-205-AC-1, FR-205-AC-2, FR-205-AC-3, FR-205-AC-4 | 🚧 Planned |
| TC-651 | The protocol system takes each step kind with structural moves folded | Integration | P1 | FR-206-AC-1, FR-206-AC-2, FR-206-AC-3, FR-206-AC-4, FR-206-AC-5, FR-206-AC-6 | 🚧 Planned |
| TC-652 | Compensations register, activate, attempt, close and end with their recovery status | Integration | P1 | FR-207-AC-1, FR-207-AC-2, FR-207-AC-3, FR-207-AC-4 | 🚧 Planned |
| TC-653 | Replicated role instances spawn under max, act, retire by lifetime and key by object | Integration | P1 | FR-208-AC-1, FR-208-AC-2, FR-208-AC-3 | 🚧 Planned |
| TC-654 | Activation on each starts instances under max_live_instances and states the budget used | Integration | P1 | FR-209-AC-1, FR-209-AC-2, FR-209-AC-3, FR-209-AC-4 | 🚧 Planned |
| TC-655 | The protocol system admits exactly the causal interleavings | Integration | P1 | FR-210-AC-1, FR-210-AC-2, FR-210-AC-3, FR-210-AC-4 | 🚧 Planned |
| TC-656 | Protocol terminal declarations check and protocol deadlocks are reported with blocked threads | Integration | P1 | FR-211-AC-1, FR-211-AC-2, FR-211-AC-3, FR-211-AC-4 | 🚧 Planned |
| TC-657 | Scheduler fairness is derived per thread and turned off by scheduling adversarial | Integration | P1 | FR-212-AC-1, FR-212-AC-2, FR-212-AC-3, FR-212-AC-4 | 🚧 Planned |
| TC-658 | Interval operators over a protocol measure distance in counted steps | Integration | P1 | FR-213-AC-1, FR-213-AC-2, FR-213-AC-3 | 🚧 Planned |
| TC-659 | Every protocol step class needs an explicit refinement row | Unit | P1 | FR-214-AC-1, FR-214-AC-2, FR-214-AC-3, FR-214-AC-4 | 🚧 Planned |
| TC-660 | ProtocolSystem implements TransitionSystem with canonical identities and the shared application rule | Integration | P1 | FR-215-AC-1, FR-215-AC-2, FR-215-AC-3, FR-215-AC-4 | 🚧 Planned |
| TC-661 | Protocol steps carry static footprints, enabling footprints and visibility | Unit | P1 | FR-216-AC-1, FR-216-AC-2, FR-216-AC-3, FR-216-AC-4, FR-216-AC-5 | 🚧 Planned |
| TC-662 | The replay facade replays a protocol counterexample through ProtocolSystem | Integration | P1 | FR-217-AC-1, FR-217-AC-2, FR-217-AC-3, FR-217-AC-4 | 🚧 Planned |
| TC-663 | S3 checks every protocol control construct and records scopes | Unit | P1 | FR-218-AC-1, FR-218-AC-2, FR-218-AC-3, FR-218-AC-4, FR-218-AC-5, FR-218-AC-6, FR-218-AC-7 | 🚧 Planned |
| TC-887 | S2 builds the complete protocol forms with their members and spans | Unit | P1 | FR-308-AC-1, FR-308-AC-2, FR-308-AC-3, FR-308-AC-4, FR-308-AC-5 | 🚧 Planned |
| TC-755 | Every lifecycle operation takes a typed request, limits and Cancel, and rejects wrong-stage input at compile time | Unit | P1 | FR-275-AC-1, FR-275-AC-2, FR-275-AC-3, FR-275-AC-5 | 🚧 Partial: steps 1 to 3, 5 and 7 pass for `parse`, `select`, `check` and `package`, with `compile_fail` doctests for `parse`, `select`, `check` and `package`; `execute` is crate-internal, so its doctest lands with FR-279; `format` (FR-003), `inspect` (FR-297), `render` (FR-298), `analyze` (FR-281), `monitor` (FR-283), `replay` (FR-098) and `check_fences` (FR-355) are not operations yet |
| TC-664 | The memory clause checks and the request selects the resolved model | Unit | P1 | FR-219-AC-1, FR-219-AC-2, FR-219-AC-3, FR-219-AC-4, FR-219-AC-5 | 🚧 Planned |
| TC-665 | S3 checks orderings and fences and classifies every access | Unit | P1 | FR-220-AC-1, FR-220-AC-2, FR-220-AC-3, FR-220-AC-4, FR-220-AC-5 | 🚧 Planned |
| TC-666 | The tso memory model explores store buffers, flushes and locked accesses | Integration | P1 | FR-221-AC-1, FR-221-AC-2, FR-221-AC-3, FR-221-AC-4 | 🚧 Planned |
| TC-667 | The ra memory model explores messages, views, fences and garbage collection | Integration | P1 | FR-222-AC-1, FR-222-AC-2, FR-222-AC-3, FR-222-AC-4 | 🚧 Planned |
| TC-668 | The SC event graph orders seq_cst events and prunes cyclic choices | Integration | P1 | FR-223-AC-1, FR-223-AC-2, FR-223-AC-3, FR-223-AC-4 | 🚧 Planned |
| TC-669 | Non-atomic locations explore and the race-freedom item reports races once | Integration | P1 | FR-224-AC-1, FR-224-AC-2, FR-224-AC-3, FR-224-AC-4 | 🚧 Planned |
| TC-670 | Memory bounds limit stores, default to 4 and are stated in every result | Integration | P1 | FR-225-AC-1, FR-225-AC-2, FR-225-AC-3, FR-225-AC-4 | 🚧 Planned |
| TC-671 | S3 warns of load-buffering shapes and negotiation routes by resolved model | Unit | P1 | FR-226-AC-1, FR-226-AC-2, FR-226-AC-3 | 🚧 Planned |
| TC-672 | Weak-memory counterexamples carry memory components and replay | Integration | P1 | FR-227-AC-1, FR-227-AC-2, FR-227-AC-3, FR-227-AC-4 | 🚧 Planned |
| TC-673 | Flush and visibility fairness join every infinite-trace fairness set | Integration | P1 | FR-228-AC-1, FR-228-AC-2, FR-228-AC-3, FR-228-AC-4 | 🚧 Planned |
| TC-674 | The memory component, observations, atoms and footprints flow through the seam | Integration | P1 | FR-229-AC-1, FR-229-AC-2, FR-229-AC-3, FR-229-AC-4, FR-229-AC-5 | 🚧 Planned |
| TC-888 | S2 builds the memory clause, access ordering and fence forms | Unit | P1 | FR-309-AC-1, FR-309-AC-2, FR-309-AC-3 | 🚧 Planned |
| TC-590 | S3 checks state-graph claims, refuses non-state predicates and fairness, and the request carries deadlock-freedom items | Unit | P1 | FR-165-AC-1, FR-165-AC-2, FR-165-AC-3 | 🚧 Planned |
| TC-591 | Phase 0 samples seeded witnesses for possible claims, on by default | Integration | P1 | FR-166-AC-1, FR-166-AC-2, FR-166-AC-3, FR-166-AC-4 | 🚧 Planned |
| TC-592 | EN-1 explores a state-graph subject once, labels it and tracks open nodes | Integration | P1 | FR-167-AC-1, FR-167-AC-2, FR-167-AC-3, FR-167-AC-4, FR-167-AC-5 | 🚧 Planned |
| TC-593 | Backward reachability and path counting decide state-graph claims with canonical evidence | Integration | P1 | FR-168-AC-1, FR-168-AC-2, FR-168-AC-3, FR-168-AC-4, FR-168-AC-6 | 🚧 Planned |
| TC-594 | State-graph outcomes settle as terminal records that state their settlement method | Unit | P1 | FR-169-AC-1, FR-169-AC-2, FR-169-AC-3, FR-169-AC-4, FR-169-AC-5 | 🚧 Planned |
| TC-595 | The replay facade replays witnesses, traps and path pairs through ModelSystem | Integration | P1 | FR-170-AC-1, FR-170-AC-2, FR-170-AC-3, FR-170-AC-4, FR-170-AC-6 | 🚧 Planned |
| TC-612 | An undefined predicate refutes a state-graph claim and replays | Integration | P1 | FR-168-AC-5, FR-169-AC-6, FR-170-AC-5 | 🚧 Planned |
| TC-613 | A witness proves possible only after exploration rules out an undefined evaluation | Integration | P1 | FR-166-AC-5, FR-168-AC-7, FR-169-AC-7 | 🚧 Planned |
| TC-614 | A witness on a stopped run settles inconclusive with well-definedness unchecked | Integration | P1 | FR-166-AC-6, FR-168-AC-8, FR-169-AC-8 | 🚧 Planned |
| TC-618 | Phase 0 walks stop at their step budget, and the horizon is not a limit | Integration | P1 | FR-166-AC-7 | 🚧 Planned |
| TC-619 | An explored witness ends at a target node on a run with open nodes | Integration | P1 | FR-168-AC-9 | 🚧 Planned |
| TC-643 | State-graph certificates of true claims are accepted and certify the proof | Integration | P1 | FR-169-AC-9 | 🚧 Planned |
| TC-644 | State-graph certificates that do not hold are rejected and never prove | Integration | P1 | FR-169-AC-10 | 🚧 Planned |
| TC-596 | S2 builds forms for hyper clauses over behaviours and relations over model executions | Unit | P1 | FR-171-AC-1, FR-171-AC-2, FR-171-AC-3 | 🚧 Planned |
| TC-597 | S3 binds trace variables to models, types indexed atoms and step labels, splits the match and checks align skip | Unit | P1 | FR-172-AC-1, FR-172-AC-2, FR-172-AC-3, FR-172-AC-4 | 🚧 Planned |
| TC-598 | Hyper and relation clauses classify into HP-1 to HP-6 and HP-4 settles unsupported | Unit | P1 | FR-173-AC-1, FR-173-AC-2, FR-173-AC-3, FR-173-AC-4 | 🚧 Planned |
| TC-599 | Hyper products admit reductions by their rows and halve by compiler-checked copy-swap | Integration | P1 | FR-174-AC-1, FR-174-AC-2, FR-174-AC-3, FR-174-AC-4 | 🚧 Planned |
| TC-600 | The evaluator reads a hyper body over a tuple of lassos in lockstep | Unit | P1 | FR-175-AC-1, FR-175-AC-2, FR-175-AC-3 | 🚧 Planned |
| TC-601 | EN-1 checks a universal hyperproperty over the self-composition product | Integration | P1 | FR-176-AC-1, FR-176-AC-2, FR-176-AC-3, FR-176-AC-4 | 🚧 Planned |
| TC-602 | EN-1 checks a forall-exists safety hyperproperty by witness sets | Integration | P1 | FR-177-AC-1, FR-177-AC-2, FR-177-AC-3, FR-177-AC-4, FR-177-AC-5, FR-177-AC-6, FR-183-AC-6 | 🚧 Planned |
| TC-603 | EN-1 checks a projection-aligned hyperproperty over the projected product | Integration | P1 | FR-178-AC-1, FR-178-AC-2, FR-178-AC-3, FR-178-AC-4, FR-178-AC-5, FR-178-AC-6 | 🚧 Planned |
| TC-604 | EN-1 checks a step relation over every tuple of reachable transitions | Integration | P1 | FR-179-AC-1, FR-179-AC-2, FR-179-AC-3 | 🚧 Planned |
| TC-605 | A bound step relation hands over its code claim, and its Kani counterexample replays | Integration | P1 | FR-180-AC-1, FR-180-AC-2, FR-180-AC-3 | 🚧 Planned |
| TC-606 | A single-existential claim is proved by a lasso witness and refuted by a trap | Integration | P1 | FR-181-AC-1, FR-181-AC-2, FR-181-AC-3, FR-181-AC-4 | 🚧 Planned |
| TC-615 | A single-existential witness proves only after exploration rules out an undefined evaluation | Integration | P1 | FR-181-AC-5 | 🚧 Planned |
| TC-616 | A single-existential witness on a stopped run settles well-definedness unchecked | Integration | P1 | FR-181-AC-6 | 🚧 Planned |
| TC-645 | The product-closure certificate checker accepts the certificates of true hyper proofs | Integration | P1 | FR-163-AC-1 | 🚧 Planned |
| TC-646 | The product-closure certificate checker rejects tampered and false certificates and stops at a limit | Integration | P1 | FR-163-AC-2, FR-163-AC-3, FR-163-AC-4 | 🚧 Planned |
| TC-607 | Hyper and step-relation outcomes settle as terminal records with their causes | Unit | P1 | FR-182-AC-1, FR-182-AC-2, FR-182-AC-3, FR-182-AC-4, FR-182-AC-7, FR-182-AC-8 | 🚧 Planned |
| TC-608 | The replay facade replays hyper counterexamples of every kind through ModelSystem | Integration | P1 | FR-183-AC-1, FR-183-AC-2, FR-183-AC-3, FR-183-AC-4 | 🚧 Planned |
| TC-609 | max_witness_set and max_relation_tuples are caller budgets with published defaults | Unit | P1 | FR-184-AC-1, FR-184-AC-2, FR-184-AC-3 | 🚧 Planned |
| TC-610 | An HP-1 relation whose first refuting tuple is undefined settles refuted with UndefinedEvaluation | Integration | P1 | FR-179-AC-4, FR-182-AC-5, FR-183-AC-5 | 🚧 Planned |
| TC-611 | An HP-1 relation whose first refuting tuple is false settles refuted ahead of later undefined tuples | Integration | P1 | FR-179-AC-5, FR-182-AC-6 | 🚧 Planned |
| TC-540 | S3 checks a refinement declaration's subjects, population map and object map | Unit | P1 | FR-135-AC-1, FR-135-AC-2, FR-135-AC-3, FR-135-AC-4 | 🚧 Planned |
| TC-541 | S3 checks a refinement's step rows, over a model subject and a protocol subject | Unit | P1 | FR-136-AC-1, FR-136-AC-2, FR-136-AC-3, FR-136-AC-4 | 🚧 Planned |
| TC-542 | S3 checks a refinement's assume and ensure rows into F_C and F_A | Unit | P1 | FR-137-AC-1, FR-137-AC-2, FR-137-AC-3, FR-137-AC-4 | 🚧 Planned |
| TC-543 | History fields check at S3 and compute along a behaviour without changing the concrete model | Unit | P1 | FR-138-AC-1, FR-138-AC-2, FR-138-AC-3, FR-138-AC-4 | 🚧 Planned |
| TC-544 | Population-valued expressions type and evaluate inside a refinement only | Unit | P1 | FR-139-AC-1, FR-139-AC-2, FR-139-AC-3, FR-139-AC-4, FR-139-AC-5 | 🚧 Planned |
| TC-545 | The refinement mapping builds the abstract state from a concrete state and its history | Unit | P1 | FR-140-AC-1, FR-140-AC-2, FR-140-AC-3, FR-140-AC-4 | 🚧 Planned |
| TC-546 | check_step decides initial states and steps by the stutter, abstract-operation and any rules | Unit | P1 | FR-141-AC-1, FR-141-AC-2, FR-141-AC-3, FR-141-AC-4, FR-141-AC-5 | 🚧 Planned |
| TC-547 | The refinement product proves, refutes, leaves undetermined and stops the safety half | Integration | P1 | FR-142-AC-1, FR-142-AC-2, FR-142-AC-3, FR-142-AC-4, FR-142-AC-5, FR-142-AC-6, FR-142-AC-7 | 🚧 Planned |
| TC-548 | The liveness half reads abstract fairness through the mapping under concrete fairness | Integration | P1 | FR-143-AC-1, FR-143-AC-2, FR-143-AC-3, FR-143-AC-4, FR-143-AC-5 | 🚧 Planned |
| TC-549 | Refinement items request one temporal-satisfaction record and settle as one terminal record | Unit | P1 | FR-144-AC-1, FR-144-AC-2, FR-144-AC-3, FR-144-AC-4, FR-144-AC-5 | 🚧 Planned |
| TC-550 | The replay facade replays a refinement counterexample and recomputes what the abstract model saw | Integration | P1 | FR-145-AC-1, FR-145-AC-2, FR-145-AC-3, FR-145-AC-4 | 🚧 Planned |
| TC-551 | Per-step simulation records are written for functional refinements and never settle refuted | Unit | P1 | FR-146-AC-1, FR-146-AC-2, FR-146-AC-3, FR-146-AC-4 | 🚧 Planned |
| TC-552 | S3 checks a refinement whose abstract side is a protocol and writes a refinement record | Unit | P1 | FR-147-AC-1, FR-147-AC-2, FR-147-AC-3, FR-147-AC-4 | 🚧 Planned |
| TC-553 | Concrete steps are decided against an abstract protocol with internal closure and observation labels | Integration | P1 | FR-148-AC-1, FR-148-AC-2, FR-148-AC-3, FR-148-AC-4, FR-148-AC-5 | 🚧 Planned |
| TC-554 | An undefined mapping row or history update refutes a refinement and replays | Integration | P1 | FR-138-AC-5, FR-141-AC-6, FR-142-AC-8, FR-145-AC-5 | 🚧 Planned |
| TC-555 | A refused or incomplete mapping row or argument leaves the refinement undetermined, and an undefined argument refutes | Unit | P1 | FR-140-AC-5, FR-141-AC-7, FR-144-AC-6 | 🚧 Planned |
| TC-556 | The simulation certificate checker accepts a true relation and rejects a false one | Integration | P1 | FR-149-AC-1, FR-149-AC-2, FR-149-AC-3, FR-149-AC-4, FR-144-AC-7 | 🚧 Planned |
| TC-889 | S2 builds the refinement form with every row in source order | Unit | P1 | FR-310-AC-1, FR-310-AC-2, FR-310-AC-3 | 🚧 Planned |
| TC-620 | S3 admits random parameters, workloads and rewards and refuses malformed ones | Unit | P1 | FR-185-AC-1, FR-185-AC-2, FR-185-AC-3, FR-185-AC-4 | 🚧 Planned |
| TC-890 | S2 builds random, reward and workload declaration forms | Unit | P1 | FR-311-AC-1, FR-311-AC-2, FR-311-AC-3 | 🚧 Planned |
| TC-621 | S3 checks probabilistic claim forms, thresholds, units and confidence parameters | Unit | P1 | FR-186-AC-1, FR-186-AC-2, FR-186-AC-3, FR-186-AC-4 | 🚧 Planned |
| TC-622 | ModelSystem gives actions, step probabilities and rewards, and reports NotMarkov | Integration | P1 | FR-187-AC-1, FR-187-AC-2, FR-187-AC-3, FR-187-AC-4 | 🚧 Planned |
| TC-623 | The QSpec sampler draws weighted choices exactly and reproducibly | Integration | P1 | FR-188-AC-1, FR-188-AC-2, FR-188-AC-3 | 🚧 Planned |
| TC-624 | EN-4 decides probabilistic claims with Okamoto and SPRT, with Bonferroni, symmetry and activation | Integration | P1 | FR-189-AC-1, FR-189-AC-2, FR-189-AC-3, FR-189-AC-4, FR-189-AC-5 | 🚧 Planned |
| TC-625 | EN-4 measures a long-run fraction by regeneration with asymptotic coverage | Integration | P1 | FR-190-AC-1, FR-190-AC-2, FR-190-AC-3 | 🚧 Planned |
| TC-626 | Statistical runs stop on caller-set budgets and reproduce from their provenance | Integration | P1 | FR-191-AC-1, FR-191-AC-2, FR-191-AC-3, FR-191-AC-4 | 🚧 Planned |
| TC-627 | Statistical results settle on the measured axis, fail the pipeline when rejected, and never count as proof | Integration | P1 | FR-192-AC-1, FR-192-AC-2, FR-192-AC-3, FR-192-AC-4 | 🚧 Planned |
| TC-628 | Sampled witnesses are kept in trace order and replay through the model | Integration | P1 | FR-193-AC-1, FR-193-AC-2, FR-193-AC-3 | 🚧 Planned |
| TC-629 | Window aggregates evaluate exactly over past windows and keep the temporal layer Boolean | Integration | P1 | FR-194-AC-1, FR-194-AC-2, FR-194-AC-3, FR-194-AC-4 | 🚧 Planned |
| TC-630 | S3 checks exact-only forms and fairness sets, and EN-5 advertises exact evidence | Unit | P1 | FR-195-AC-1, FR-195-AC-2, FR-195-AC-3, FR-195-AC-4 | 🚧 Planned |
| TC-631 | EN-5 builds the DTMC or MDP product with monitors, accumulators and intermediate states | Integration | P1 | FR-196-AC-1, FR-196-AC-2, FR-196-AC-3, FR-196-AC-4 | 🚧 Planned |
| TC-632 | Backward induction decides finite-horizon forms exactly, or over dyadic intervals with precision doubling | Integration | P1 | FR-197-AC-1, FR-197-AC-2, FR-197-AC-3, FR-197-AC-4 | 🚧 Planned |
| TC-633 | Unbounded reachability and expected rewards are decided by graph precomputation, interval iteration and exact policy iteration | Integration | P1 | FR-198-AC-1, FR-198-AC-2, FR-198-AC-3, FR-198-AC-4, FR-198-AC-5 | 🚧 Planned |
| TC-634 | Long-run fractions are decided exactly by bottom components and maximal end components | Integration | P1 | FR-199-AC-1, FR-199-AC-2, FR-199-AC-3 | 🚧 Planned |
| TC-635 | Every-scheduler claims with a fairness set are decided over fair schedulers through fair end components | Integration | P1 | FR-200-AC-1, FR-200-AC-2, FR-200-AC-3, FR-200-AC-4 | 🚧 Planned |
| TC-636 | check_probability_certificate accepts sound certificates and rejects flawed ones in exact rationals | Integration | P1 | FR-201-AC-1, FR-201-AC-2, FR-201-AC-3, FR-201-AC-4, FR-201-AC-5 | 🚧 Planned |
| TC-637 | replay_probabilistic_witness reproduces path-set and subsystem refutations and refuses altered witnesses | Integration | P1 | FR-202-AC-1, FR-202-AC-2, FR-202-AC-3, FR-202-AC-4, FR-202-AC-6 | 🚧 Planned |
| TC-638 | Exact runs stop on caller-set budgets and settle with ExactValue or ValueBounds | Integration | P1 | FR-203-AC-1, FR-203-AC-2, FR-203-AC-3, FR-203-AC-4 | 🚧 Planned |
| TC-639 | Closed probabilistic timed automata are decided exactly through digital clocks, with the workload resolving delays | Integration | P1 | FR-204-AC-1, FR-204-AC-2, FR-204-AC-3, FR-204-AC-4 | 🚧 Planned |
| TC-640 | A sampled undefined evaluation is a rejection and replays | Integration | P1 | FR-189-AC-6, FR-192-AC-5, FR-193-AC-4 | 🚧 Planned |
| TC-641 | An undefined state reached with positive probability refutes an exact claim | Integration | P1 | FR-196-AC-5, FR-202-AC-5, FR-203-AC-5 | 🚧 Planned |
| TC-642 | A delay no distribution gives is decided by its minimum or maximum under a workload | Integration | P1 | FR-204-AC-5 | 🚧 Planned |
| TC-685 | S3 checks time declarations, clocks, clock constraints, time invariants and urgency | Unit | P1 | FR-230-AC-1, FR-230-AC-2, FR-230-AC-3, FR-230-AC-4 | 🚧 Planned |
| TC-686 | A timed subject's behaviours read as timed traces with delays, urgency, idle tails and digital time | Integration | P1 | FR-231-AC-1, FR-231-AC-2, FR-231-AC-3, FR-231-AC-4 | 🚧 Planned |
| TC-687 | The request carries one time-lock-freedom item per timed subject, and deadlocks, fairness and vacuity read over time | Unit | P1 | FR-232-AC-1, FR-232-AC-2, FR-232-AC-3, FR-232-AC-4 | 🚧 Planned |
| TC-688 | Model claims bind model-time or model-steps, with defaults, identity keys and interval units | Unit | P1 | FR-233-AC-1, FR-233-AC-2, FR-233-AC-3 | 🚧 Planned |
| TC-689 | S3 checks timed intervals with open or closed ends and classifies TT-1 to TT-4, with the punctual-interval boundary | Unit | P1 | FR-234-AC-1, FR-234-AC-2, FR-234-AC-3, FR-234-AC-4 | 🚧 Planned |
| TC-690 | Timed outcomes settle as FR-331 terminal records with zone-certified, exhaustive and new-cause rows | Unit | P1 | FR-235-AC-1, FR-235-AC-2, FR-235-AC-3, FR-235-AC-4 | 🚧 Planned |
| TC-691 | Timed counterexamples carry exact rational delays, a final delay for time-locks, and a time-divergent lasso | Unit | P1 | FR-236-AC-1, FR-236-AC-2, FR-236-AC-3, FR-236-AC-4 | 🚧 Planned |
| TC-692 | The replay facade replays timed counterexamples and time-locks in exact arithmetic | Integration | P1 | FR-237-AC-1, FR-237-AC-2, FR-237-AC-3, FR-237-AC-4, FR-237-AC-6 | 🚧 Planned |
| TC-693 | Zones as difference-bound matrices in exact integer arithmetic | Unit | P1 | FR-238-AC-1, FR-238-AC-2, FR-238-AC-3 | ✅ Passed locally (`qsl-analyze/src/zone_check/dbm/tests.rs`, `qsl-analyze/src/zone_check/scale.rs`) |
| TC-694 | The zone engine decides timed claims by symbolic search, with finite abstraction, budgets and determinism | Integration | P1 | FR-239-AC-1, FR-239-AC-2, FR-239-AC-3, FR-239-AC-4 | 🚧 Planned |
| TC-695 | Timed liveness under time divergence and fairness, and time-lock search, on the symbolic graph | Integration | P1 | FR-240-AC-1, FR-240-AC-2, FR-240-AC-3, FR-240-AC-4, FR-241-AC-5 | 🚧 Planned |
| TC-696 | Symbolic counterexamples concretize canonically to exact rational delays | Unit | P1 | FR-241-AC-1, FR-241-AC-2, FR-241-AC-3, FR-241-AC-4 | 🚧 Planned |
| TC-697 | Timed formulas translate to claim automata that agree with the evaluator | Unit | P1 | FR-242-AC-1, FR-242-AC-2, FR-242-AC-3 | 🚧 Planned |
| TC-698 | Timed items route to the zone engine, and only exact probabilistic claims to the digital-clock route | Integration | P1 | FR-243-AC-1, FR-243-AC-2, FR-243-AC-3 | 🚧 Planned |
| TC-699 | Every zone-engine proof carries a canonical zone certificate | Unit | P1 | FR-244-AC-1, FR-244-AC-2, FR-244-AC-3, FR-244-AC-4 | 🚧 Planned |
| TC-700 | The in-core checker accepts valid zone certificates and rejects every tampering | Integration | P1 | FR-245-AC-1, FR-245-AC-2, FR-245-AC-3, FR-245-AC-4, FR-245-AC-5, FR-245-AC-6 | 🚧 Partial: step 4 (FR-245-AC-4, the checker DBM's Kani harnesses in `qsl-replay/src/certificate/zone/proofs.rs`) and step 6 (FR-245-AC-6, `tests/it/family_outcome_layering.rs`) pass locally; steps 1 to 3 and 5 (FR-245-AC-1, AC-2, AC-3, AC-5) planned |
| TC-701 | S3 checks task sets and routes schedulability claims to EN-7 | Unit | P1 | FR-246-AC-1, FR-246-AC-2 | 🚧 Planned |
| TC-702 | EN-7 computes fixed-priority, EDF and AMC verdicts in exact arithmetic | Unit | P1 | FR-247-AC-1, FR-247-AC-2, FR-247-AC-3, FR-247-AC-4 | 🚧 Planned |
| TC-703 | Closed-form verdicts settle only after check_closed_form recomputes their evidence | Unit | P1 | FR-248-AC-1, FR-248-AC-2, FR-248-AC-3, FR-248-AC-4 | 🚧 Planned |
| TC-704 | Task automata lower to timed subjects for deadline checking, and the stopwatch class is unsupported | Integration | P1 | FR-249-AC-1, FR-249-AC-2 | 🚧 Planned |
| TC-705 | Hybrid solver results map to proved or inconclusive, never refuted | Unit | P1 | FR-250-AC-1, FR-250-AC-2 | 🚧 Planned |
| TC-706 | Tick-based monitor plans round soundly, size buffers from the event rate and fault on counter errors | Unit | P1 | FR-251-AC-1, FR-251-AC-2, FR-251-AC-3 | 🚧 Planned |
| TC-707 | QSL writes monitor-agreement, tick-arithmetic and timestamp-contract obligations, and disposes elapsed-time obligations unsupported | Unit | P1 | FR-252-AC-1, FR-252-AC-2 | 🚧 Planned |
| TC-708 | Delay distributions check, windows are exact and the race resolves ties by the workload | Unit | P1 | FR-253-AC-1, FR-253-AC-2, FR-253-AC-3, FR-253-AC-4 | 🚧 Planned |
| TC-709 | Timed runs sample with exact rational delays, measure timed events and replay | Integration | P1 | FR-254-AC-1, FR-254-AC-2, FR-254-AC-3, FR-254-AC-4 | 🚧 Planned |
| TC-710 | An undefined timed claim evaluation refutes with a timed prefix and replays | Integration | P1 | FR-239-AC-5, FR-237-AC-5, FR-235-AC-5 | 🚧 Planned |
| TC-565 | S3 carries the symmetric annotation and refuses a fold over its references | Unit | P1 | FR-150-AC-1, FR-150-AC-2 | 🚧 Planned |
| TC-566 | Every identity-observing form is refused in every clause kind, and transparent forms check | Unit | P1 | FR-150-AC-3, FR-150-AC-4 | 🚧 Planned |
| TC-567 | Symmetry declarations admit on an annotated population and refuse malformed forms | Unit | P1 | FR-151-AC-1, FR-151-AC-2 | 🚧 Planned |
| TC-568 | Initial states not closed under the generators settle SymmetryBroken; a finer class admits | Unit | P1 | FR-151-AC-3, FR-151-AC-4 | 🚧 Planned |
| TC-569 | The sort canonicaliser maps each state into its orbit and coalesces orbits | Unit | P1 | FR-152-AC-1, FR-152-AC-2, FR-152-AC-3, FR-152-AC-4 | 🚧 Planned |
| TC-570 | Weak each fairness is decided on the annotated quotient | Integration | P1 | FR-153-AC-1, FR-153-AC-2 | 🚧 Planned |
| TC-571 | Strong each fairness refines on the quotient, and every verdict equals the unreduced verdict | Integration | P1 | FR-153-AC-3, FR-153-AC-4 | 🚧 Planned |
| TC-572 | A receiver-scoped modifies entry limits candidates and check_frame to the receiver | Integration | P1 | FR-154-AC-1, FR-154-AC-2, FR-154-AC-5 | 🚧 Planned |
| TC-573 | The receiver scope holds in the Frame run and admission, and in the emitted frame node | Integration | P1 | FR-154-AC-3, FR-154-AC-4 | 🚧 Planned |
| TC-574 | Read, write and enabling footprints derive from clauses and frames, and decide independence | Unit | P1 | FR-155-AC-1, FR-155-AC-2 | 🚧 Planned |
| TC-575 | Footprints are enforced: a read outside the footprint is an internal fault, a write outside the frame a violation | Integration | P1 | FR-155-AC-3, FR-155-AC-4 | 🚧 Planned |
| TC-576 | Ample sets reduce ADR-021 §7.2 to one interleaving and keep the deadlock | Integration | P1 | FR-156-AC-1, FR-156-AC-2 | 🚧 Planned |
| TC-577 | The breadth-first proviso prevents ignoring, the closure reaches through disabled members, and choice is deterministic | Integration | P1 | FR-156-AC-3, FR-156-AC-4 | 🚧 Planned |
| TC-578 | Fairness visibility keeps a fair violation that C0 to C3 alone would discard | Integration | P1 | FR-157-AC-1, FR-157-AC-2 | 🚧 Planned |
| TC-579 | Fairness visibility applies only under a non-empty fairness set, at either granularity, and over protocols | Integration | P1 | FR-157-AC-3, FR-157-AC-4 | 🚧 Planned |
| TC-580 | A state constraint stores boundary states, reports real violations and never proves | Integration | P1 | FR-158-AC-1, FR-158-AC-2 | 🚧 Planned |
| TC-581 | Undefined, absent and initially-false constraints, bounds, identity and determinism | Integration | P1 | FR-158-AC-3, FR-158-AC-4 | 🚧 Planned |
| TC-582 | The preservation table admits or settles each selected reduction before expansion | Unit | P1 | FR-159-AC-1, FR-159-AC-2 | 🚧 Planned |
| TC-583 | Reductions route only to advertising candidates, need footprints, and enter the obligation identity | Integration | P1 | FR-159-AC-3, FR-159-AC-4 | 🚧 Planned |
| TC-584 | Reduced proofs, reduction causes, depth and refutations settle through the one map | Unit | P1 | FR-160-AC-1, FR-160-AC-2, FR-160-AC-3, FR-160-AC-4, FR-160-AC-7 | 🚧 Planned |
| TC-585 | Symmetry counterexamples concretise to subject traces, closing a loop in one or more passes | Integration | P1 | FR-161-AC-1, FR-161-AC-2 | 🚧 Planned |
| TC-586 | Each-fairness loops take every identity, and every reduced counterexample is one ordinary trace kind | Integration | P1 | FR-161-AC-3, FR-161-AC-4 | 🚧 Planned |
| TC-587 | TransitionSystem hooks offer reductions, the product delegates them, and simulation stays unreduced | Integration | P1 | FR-162-AC-1, FR-162-AC-2, FR-162-AC-3, FR-162-AC-4 | 🚧 Planned |
| TC-588 | An undefined claim evaluation found on a reduced run refutes with a concrete prefix | Integration | P1 | FR-160-AC-5 | 🚧 Planned |
| TC-589 | Complete enabling footprints and membership locations keep safety violations under partial-order reduction | Integration | P1 | FR-155-AC-5, FR-155-AC-6, FR-156-AC-5, FR-156-AC-6 | 🚧 Planned |
| TC-617 | Complete enabling footprints and membership keep fair violations under partial-order reduction | Integration | P1 | FR-157-AC-5, FR-157-AC-6 | 🚧 Planned |
| TC-886 | Reduced and unreduced runs agree over the model-check corpus | Integration | P1 | FR-160-AC-6 | 🚧 Planned |
| TC-815 | S1 and S2 build union declaration and case forms | Unit | P1 | FR-313-AC-1, FR-313-AC-2 | 🚧 Planned |
| TC-816 | S1 parses a case scrutinee without a top-level record and keeps protocol case apart | Unit | P1 | FR-313-AC-3 | 🚧 Planned |
| TC-817 | S3 admits union declarations, including an escaping recursive union | Unit | P1 | FR-316-AC-1, FR-316-AC-2 | 🚧 Planned |
| TC-818 | S3 refuses ill-formed union declarations and bounds them only by checking ceilings | Unit | P1 | FR-316-AC-2, FR-316-AC-3, FR-316-AC-4 | 🚧 Planned |
| TC-819 | S3 checks union construction and refuses FR-143's four construction errors | Unit | P1 | FR-317-AC-1, FR-317-AC-2, FR-317-AC-4 | 🚧 Planned |
| TC-820 | S3 resolves qualified member names by candidate and refuses ambiguity | Unit | P1 | FR-317-AC-3 | 🚧 Planned |
| TC-821 | S3 checks a case, types its binders per arm and decides its result type by QSpec FR-146 | Unit | P1 | FR-318-AC-1, FR-318-AC-2 | 🚧 Planned |
| TC-822 | S3 refuses each exhaustiveness obligation with its payload and locus | Unit | P1 | FR-318-AC-3 | 🚧 Planned |
| TC-823 | S3 reports exactly one refusal per case, in builder order | Unit | P1 | FR-318-AC-4 | 🚧 Planned |
| TC-824 | A case checks under each clause kind and adds no requirement record | Unit | P1 | FR-318-AC-5 | 🚧 Planned |
| TC-825 | Deeply nested case checks under default ceilings and stops only at a named ceiling | Unit | P1 | FR-318-AC-6 | 🚧 Planned |
| TC-826 | Union, member and case identities are stable and member identity is a retyped VariantId | Unit | P1 | FR-319-AC-1, FR-319-AC-2, FR-319-AC-3 | 🚧 Planned |
| TC-827 | Union nodes lower and emit in QSpec's spelling and survive the I2 read and recompile | Integration | P1 | FR-320-AC-1, FR-320-AC-2 | 🚧 Planned |
| TC-828 | A value-validity item containing case settles unsupported downstream | Integration | P1 | FR-320-AC-3 | 🚧 Planned |
| TC-829 | Argument admission refuses ill-formed supplied union values before any charge | Unit | P1 | FR-321-AC-1, FR-321-AC-2 | 🚧 Planned |
| TC-830 | Union values round-trip through v2 union_value nodes at any depth | Unit | P1 | FR-321-AC-3, FR-321-AC-4 | 🚧 Planned |
| TC-831 | S6a evaluates case and construction, propagates stopped operands and faults on a broken invariant | Unit | P1 | FR-322-AC-1, FR-322-AC-2, FR-322-AC-4 | 🚧 Planned |
| TC-832 | S6a charges union construction at QSpec's accounting point and nothing for case selection | Unit | P1 | FR-322-AC-3 | 🚧 Planned |
| TC-833 | Union values as collection elements, in queries and in predicates | Unit | P1 | FR-323-AC-1, FR-323-AC-2, FR-323-AC-3, FR-046-AC-9 | 🚧 Planned |
| TC-834 | Unions compile under the value profile with the same lock selections as records | Integration | P1 | FR-324-AC-1 | 🚧 Planned |
| TC-720 | A reached limit names its kind, bound, count and setting | Unit | P1 | FR-255-AC-1, FR-255-AC-2, FR-255-AC-3 | 🚧 Planned |
| TC-721 | Every setting raises its limit through the library, the replay request and the settings operation the driver CLI calls | Integration | P1 | FR-255-AC-4, FR-255-AC-5, FR-255-AC-6 | 🚧 Planned |
| TC-722 | Deep sources parse on a small stack under raised S1 limits | Unit | P1 | FR-256-AC-1 | 🚧 Planned |
| TC-723 | S1 parses to its default limits and names the setting it reached | Unit | P1 | FR-256-AC-2, FR-256-AC-3 | 🚧 Planned |
| TC-724 | Deep forms and control anchors build on a small stack | Unit | P1 | FR-257-AC-1, FR-257-AC-2, FR-257-AC-3 | 🚧 Planned |
| TC-725 | Deep expressions check, lower and emit on a small stack | Unit | P1 | FR-258-AC-1, FR-258-AC-5 | 🚧 Planned |
| TC-726 | Deep types key and lower their text leaves on a small stack | Unit | P1 | FR-258-AC-2 | 🚧 Planned |
| TC-727 | Checker defaults admit any depth that fits, and its limit outcomes name the setting | Unit | P1 | FR-258-AC-3, FR-258-AC-4 | 🚧 Planned |
| TC-728 | A deep package's identities do not depend on the stack, and byte limits report as limits | Unit | P1 | FR-259-AC-1, FR-259-AC-2, FR-259-AC-4 | 🚧 Planned (step 5 implemented) |
| TC-729 | QSL identities, digests and allocation failures over quire-canonical at any depth | Unit | P1 | FR-259-AC-3, FR-259-AC-5 | 🚧 Planned (step 3 implemented) |
| TC-730 | Intake judges a deep package document on its content | Integration | P1 | FR-260-AC-1, FR-260-AC-2, FR-260-AC-5 | 🚧 Planned (step 4 implemented) |
| TC-731 | Intake reports composite cycles of any length | Integration | P1 | FR-260-AC-3 | 🚧 Planned |
| TC-732 | The intake byte limit names its setting and clears when raised | Integration | P1 | FR-260-AC-4 | 🚧 Planned |
| TC-733 | Library preimage and observation reads judge deep documents on their content | Integration | P1 | FR-261-AC-1, FR-261-AC-2, FR-261-AC-3 | 🚧 Planned |
| TC-734 | A 100,000-deep recursive call completes on work fuel | Unit | P1 | FR-262-AC-1 | 🚧 Planned |
| TC-735 | Deep values and value types evaluate, key, compare, clone and drop | Unit | P1 | FR-262-AC-2 | 🚧 Planned |
| TC-736 | A deep source and value replay to the proving run's verdict | Integration | P1 | FR-263-AC-1, FR-070-AC-10 | ✅ Passed locally |
| TC-737 | Replay passes every stage limit through and names the setting it reached | Integration | P1 | FR-263-AC-2, FR-263-AC-3 | 🚧 Planned |
| TC-738 | A deep package emits, reads back and verifies, in the stratified grammar | Integration | P1 | FR-264-AC-1, FR-264-AC-2 | 🚧 Planned |
| TC-739 | The v2 read refuses an inline nested term and names its limits' settings | Unit | P1 | FR-264-AC-3, FR-264-AC-4, FR-264-AC-5 | 🚧 Planned |
| TC-902 | Core entry points and maybe_grow sites run 100,000 deep on a small stack | Unit | P1 | FR-356-AC-5, FR-356-AC-6 | 🚧 Planned |
| TC-903 | A deep-input fuzz target drives the parser and the checker | Property | P1 | FR-356-AC-7 | 🚧 Planned |
| TC-904 | Scalar-parity claims replay to diverged, agrees, refused and faulted outcomes, and never to Refuted | Unit | P1 | FR-357-AC-1, FR-357-AC-2, FR-357-AC-3, FR-357-AC-4, FR-357-AC-5, FR-357-AC-6, FR-357-AC-7, FR-357-AC-8, FR-357-AC-9, FR-357-AC-10, FR-357-AC-11, FR-357-AC-13, FR-357-AC-14, FR-357-AC-15, FR-357-AC-16, FR-357-AC-17, FR-357-AC-18, FR-357-AC-19 | ✅ Passed locally |
| TC-905 | A witness value text decodes exactly for every composite and leaf family, and each malformed form refuses | Unit | P1 | FR-070-AC-8, FR-070-AC-9, FR-070-AC-11, FR-070-AC-12 | ✅ Passed locally |
| TC-906 | Composite and leaf-family arguments replay, and refuse by kind, by domain and at the request's limits | Unit | P1 | FR-098-AC-8, FR-098-AC-9, FR-098-AC-10 | 🚧 Passed locally except the union cases (QSL-503) and a quantity-typed parameter in source (STD-113) |
| TC-907 | A composite equality-parity claim settles Diverged, Agrees or refused when falsified, and by rows V-1 to V-5 when verified | Unit | P1 | FR-358-AC-1, FR-358-AC-2, FR-358-AC-3, FR-358-AC-4, FR-358-AC-5, FR-358-AC-6, FR-358-AC-7, FR-358-AC-8 | 🚧 Passed locally except the `Text` leaf (QSL-646) and the source-level `k` cases (tested on the derivation seam) |
| TC-908 | The replay facade compiles QSL source to the checked-package bytes the spine emits | Integration | P2 | FR-060-AC-5, FR-060-AC-6, FR-060-AC-7 | ✅ Passed locally |
| TC-909 | Integer bounds and literals up to i128 check, and one beyond refuses at check | Unit | P1 | FR-091-AC-36, FR-091-AC-37, FR-091-AC-38 | ✅ Passed locally |
| TC-910 | Wide integer ranges key to their vectors, and counters keep one exact spelling | Unit | P1 | FR-092-AC-14, FR-092-AC-15 | ✅ Passed locally; step 3 reads QSpec's `integer_encoding_vectors` under `QSPEC_DIR` (`make conformance`) |
| TC-911 | Field refinement decides wide integer domains and literals exactly | Unit | P1 | FR-056-AC-16, FR-082-AC-9 | ✅ Passed locally |
| TC-912 | The IR wire round-trips integer bounds and literals up to i128 | Integration | P1 | FR-033-AC-6 | 🚧 Planned (waits on IR-662) |
| TC-913 | A wide-range source recompiles to its package_id and replays | Integration | P1 | FR-098-AC-11 | ✅ Passed locally |
| TC-894 | An imported call discharges its preconditions as a local call does | Integration | P1 | FR-099-AC-8 | 🚧 Planned |
| TC-895 | The native checker reads a field's optionality from its presence | Unit | P1 | FR-016-AC-10 | 🚧 Planned |
| TC-896 | A field redefinition narrows presence and multiplicity on separate axes | Unit | P1 | FR-082-AC-8 | 🚧 Planned |
| TC-897 | A Text scalar type is admitted with its QSpec profile | Unit | P1 | FR-056-AC-11 | 🚧 Planned |

## Provenance (FR-095, ADR-013 S-4) coverage

[FR-095](functional/FR-095-occurrence-keyed-source-map-and-locus.md)
carries ADR-013 O-07, O-12, T-5 and C-14. TC-420 to TC-422
pass locally; TC-421's QSpec-fixture step runs under `make conformance`.
O-12's `LocatedSpan` replacement and C-21 are ADR-013 §7 slice S-4b:
[FR-001](functional/FR-001-read-exact-source.md) defines the
caller-named `RawSourceRef` (TC-424) and
[FR-010](functional/FR-010-report-native-outcomes.md) the `parse` and
`format` grammar that supplies it (TC-425). Check-stage regions, stage
limits, refusal records and the I2 reader's loci are slice S-5b
([FR-096](functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md),
TC-426 to TC-429). TC-426 (a check location's region, including the C-21
embedded-document mapping), TC-427, TC-429, TC-378, TC-428
(refusal record fields at S6a, including the ten kernel value refusals'
catalog codes) and TC-500 (a `sum` running total
outside its domain is undefined) pass locally.

## Stage typestate, clause and type (FR-087–088, ADR-013 S-3) coverage

[FR-087](../spec/functional/FR-087-typestate-and-cross-package-node-key.md)
(S-3a: T-1, T-3, O-15) and
[FR-088](../spec/functional/FR-088-clause-name-and-type-identity.md)
(S-3b: O-08, O-09 clause id, O-10, O-11, O-14, C-26) are specified by
splitting ADR-013 §7 slice S-3, which no FR owned before this
split. TC-243–260, TC-281, TC-282 and TC-379 above are the corresponding
test cases. TC-379 is `🚧 Planned`, as FR-087's Status
records: it passes steps 1 and 3. TC-246 passed.
TC-243, TC-247, TC-255, TC-258, TC-260 and TC-281 are
partial, each row naming the steps its test backs; TC-259 is partial; and TC-251 covers FR-088-AC-5's `QualifiedName` half only. The
rest pass locally. FR-087 also resolves, by owner
ruling on QSL-158 (2026-09-21), the `CheckedPackage`-placement half of
an ADR-011 §4-versus-§6.1 defect: `CheckedPackage` is canonically layer-4
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
variant by variant, to an ADR-011 I2 rule, the §4 binding's condition 2, or E3
name resolution, with `DuplicatePackageId` named as
lying outside all three (FR-087-AC-12/TC-282; FR-087 Description, item 3).
`check` reads `ImportView` and `LibraryLock` (read-only) from `library`, a
permitted layer-3 edge under FR-068-AC-6's layer rule (FR-087-AC-9/TC-256).

## Kernel value identities (ADR-013 OQ-B to OQ-F) coverage

ADR-013 §8 OQ-B to OQ-F fix the kernel's reference, quantity and enum
identities. FR-088-AC-11 (TC-409) covers an enum value's FR-141
`VariantId` and its rank key. FR-088-AC-12 (TC-411) covers the two-domain
`UnitId`; it passes locally, and `make conformance` runs its QSpec-vector
step. FR-084-AC-7 (TC-410) covers one object universe per connected
supertype component and the authored object component of a reference key.
TC-409 passes locally; TC-410 is still `🚧 Planned`. OQ-A's layer-2 edge is covered by TC-398's allow-list.

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
- TC-178 (FR-069-AC-4): `qsl-replay/src/proof_result.rs::tests::tc_178_refuses_an_oversized_envelope`
- TC-179 (FR-069-AC-3): `qsl-replay/src/proof_result.rs::tests::tc_179_round_trip_preserves_backend_and_dispositions`
- TC-180 (FR-070-AC-1): `qsl-replay/src/witness.rs::witness_tests::tc_180_exactly_one_field_and_derived_facts_track_the_stored_transcript`
- TC-181 (FR-070-AC-2, FR-070-AC-6, FR-070-AC-7): `qsl-replay/src/witness.rs::witness_tests::tc_181_refuses_malformed_transcripts`, `::envelope_tests::tc_181_refuses_an_out_of_domain_digest`, `::envelope_tests::tc_181_refuses_an_oversized_encoding`
- TC-182 (FR-070-AC-3): `qsl-replay/src/witness.rs::envelope_tests::tc_182_round_trip_preserves_every_o25_member_and_the_transcript`
- TC-183 (FR-070-AC-4): `qsl-replay/src/witness.rs::envelope_tests::tc_183_refuses_reconstruction_when_any_o25_member_is_missing` — caveat: this is a `Property`-typed row, but the test asserts only four of the roughly thirteen O-25 members individually (`backend`, `trace_position`, `source_digests`, `obligation_identity`); the rest share the identical `.ok_or(WitnessRefusal::MissingMember(...))?` pattern but are not each individually exercised.
- TC-184 (FR-070-AC-5): `qsl-replay/src/witness.rs::envelope_tests::tc_184_family_payload_is_a_typed_extension_point` — caveat: the "typed extension point" half is asserted by attaching and round-tripping a new payload type; the "not an untyped map" half is a source-inspection fact (no `get_extra`/string-keyed accessor exists on `WitnessEnvelope`), not itself a runtime assertion.
- TC-185 (FR-071-AC-1): `qsl-replay/src/request.rs::tests::tc_185_carries_exactly_o26_members_and_round_trips`
- TC-186 (FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7): `qsl-replay/src/request.rs::tests::tc_186_byte_provision_is_digest_only_complete_and_bounded`
- TC-186 step 8 (FR-071-AC-10): `qsl-replay/src/request.rs::tests::refuses_an_unknown_semantic_profile_before_the_byte_provision`
- TC-186 step 9 (FR-071-AC-11): `qsl-replay/src/request.rs::tests::a_package_document_refusal_keeps_its_cause`
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
FR-150–153 binding work it carries). Fifteen of the thirty TC-213–242 rows
above cite real, currently passing tests rather than planned work, naming
the exact backing test:

- TC-215 (FR-081-AC-3): `tests/model_normalization.rs::n06_a_strictly_more_derived_redefiner_resolves_the_conflict_and_hides_every_contender`
- TC-218 (FR-082-AC-1): `tests/model_conformance.rs::r03_an_incompatible_operation_redefinition_reports_every_failing_axis`
- TC-219 (FR-082-AC-2): `tests/model_conformance.rs::r07_zero_inherited_targets_refuses_redefinition_target` and `tests/model_normalization.rs::r01_a_closing_generalization_cycle_names_the_full_rotated_chain`; FR-082-AC-6's unknown-supertype and cycle rows: `qsl-semantics/tests/it/type_environment_model.rs::divergence_*_supertype_*` and `::divergence_generalization_cycle_*`
- TC-220 (FR-082-AC-3, AC-6, AC-7): `qsl-semantics/tests/it/type_environment_model.rs::divergence_chain_within_the_ceiling_agrees_at_check_and_at_evaluation`, `::divergence_chain_past_the_ceiling_refuses_at_check_and_at_evaluation` and `::check_and_evaluation_share_one_default_and_one_edge_count` (AC-3, AC-6); `::a_deep_chain_refuses_the_admission_work_budget` and `::a_linear_chain_costs_admission_work_linear_in_its_flattened_slots` (AC-7); `tests/model_conformance.rs`'s edge-count walks (AC-3).
- TC-221 (FR-082-AC-4): `tests/model_conformance.rs::r08a_*_refuses` and `::r08b_*_discharges_the_obligation`
- TC-222 (FR-083-AC-1): `tests/model_dispatch.rs::d04_registration_order_does_not_change_the_linked_table`
- TC-223 (FR-083-AC-2): `tests/model_dispatch.rs::d03_no_candidate_*_no_applicable` and `::d02_an_undominated_multi_way_tie_*`
- TC-224 (FR-083-AC-3): `tests/model_dispatch.rs::d02_an_undominated_multi_way_tie_*` (the same `Ambiguous` outcome carries no table for the family's other, cleanly-resolving subtype)
- TC-225 (FR-083-AC-4): `qsl-semantics/tests/it/model_dispatch.rs::a_family_at_the_configured_bound_links_and_one_step_more_refuses`, `::a_ten_thousand_long_redefines_chain_links_on_a_small_stack` and `::the_checked_bridge_counts_every_walk_against_one_family_steps`; `src/check/checked_dispatch.rs::family_steps_counts_the_edges_of_every_walk_together`.
- TC-226 (FR-084-AC-1): `tests/model_population.rs::l02_unknown_closure_is_incomplete_not_refused` (both halves: `open` extent and an unclosed generalization graph) and `::l05_foreign_type_refuses`
- TC-228 (FR-084-AC-3): `tests/model_population.rs::l03_lookup_undefined_mode`, `::l03_lookup_empty_mode`, `::l03_lookup_refused_mode`
- TC-229 (FR-084-AC-4): `tests/model_population.rs::l05_conflicting_identity_refuses_after_fourth_member_charge` and `::l05_duplicate_collapses_and_recovers_l01`
- TC-235 (FR-086-AC-3): `tests/model_systems.rs::y02_wrong_export_substitutions_name_the_required_and_actual_kind` (the `Port` target refused) and `::y01_every_kind_resolves_to_its_exact_producer_key` (the `Part` target admitted)
- TC-237 (FR-081-AC-6): `tests/model_normalization.rs::n06_two_undominated_redefiners_of_the_same_target_refuse_as_a_conflict`
- TC-238 (FR-081-AC-7): `tests/model_normalization.rs::n07_record_order_does_not_affect_identity_or_view` (asserts the whole view's identity digest is order-independent; does not separately assert a per-entry iteration sequence — see the TC's own caveat)
- TC-239 (FR-082-AC-5, Part A — the arity carve-out itself): `tests/model_conformance.rs::r04_an_arity_mismatch_refuses_without_checking_parameter_axes`. Part B (a combined arity-and-result-multiplicity failure) is not backed by any existing test; see the TC's own gap note.
- TC-242 (FR-084-AC-6): `tests/model_population.rs::l01_all_instances_selects_subtype_population_once`, which already carries the quire-specification `QSpec-FR-153-AC-6` trace tag and asserts this exact guarantee (`b1_via_a == b1_via_b`)

TC-213 (FR-081-AC-1) is not passed. Normalizing a domain package admitted
through `admit` and `read_records` with an operation member, `attemptUpdate`,
yields a view whose original keys omit that operation: `normalize` builds
view entries for object types and fields only (its module doc records this as
a scope decision). QSpec `model-complete.md` phase 2 qualifies operations,
relationships and systems elements as effective members too, so the
correspondence does not name every admitted model declaration.

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

## Kernel population identity (FR-089) coverage

[FR-089](../spec/functional/FR-089-carry-population-identity-across-the-kernel-boundary.md)
carries ADR-013 O-13's Population row (QC-21), authored to
close an AD-016 kernel-row gap. The kernel side landed the
opaque `PopulationId` type and `Value::Population(PopulationId)` in
`quire-exact`. Kernel `ValueType::admits` returns `false` for every
`(ValueType::Population(_), Value::Population(_))` pair, `plan_pairs` refuses
a population pair with `Refusal::CheckedInvariant`, and `compare_keys` yields
no key for one (FR-089-AC-6). QSL has no equality or key of its own: it
calls `quire_exact::member_equal` and `quire_exact::compare_keys`. TC-297, with its three tests, is in `agent-ix/quire-exact`. The
declared-maximum comparison is a QSL-layer check (FR-089-AC-5, TC-295): QSL
`model`/the evaluator resolves the binding, then compares its declared
maximum. TC-292, the inspection that the kernel payload is a `PopulationId`
with no model dependency, is in `agent-ix/quire-exact`.

The other half (`model` minting and the evaluator's
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
program (see that method's own doc), but since FR-090-AC-10 it is
no longer a kernel refusal: meeting either condition there is an S6a
invariant break, `Err(InternalFault)`, not `Refusal::UnresolvedPopulation`/
`Refusal::PopulationMaximumMismatch`, which are deleted.

FR-089's own admission preimage (domain package selection, `population_key`,
admission role) does not distinguish two bindings that differ only in
document content or declared maximum, admitted under the same
package/key/role within one evaluation -- an open spec question this Slice
does not resolve.
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
`check` core. TC-392 to TC-400, TC-402 and TC-403 pass locally
(with TC-405, TC-412 and TC-398's three steps). TC-401 and TC-406 are `🚧 Partial`: their remaining clauses
are noted on their rows. TC-398 and TC-402 use the resolved-import and definition-scan
approach of TC-256, TC-170 and TC-390. TC-399 is the end-to-end case from
source to `CheckedPackage::call`. TC-480 and TC-481 back the `enum`,
`ordered enum` and `predicate` declarations (FR-091-AC-25 to AC-30,
FR-092-AC-13), and TC-482 and TC-483 the `dimension` and `unit`
declarations and their `UnitGraph` (FR-091-AC-31 to AC-35). TC-480 to
TC-482 pass locally; TC-483 passes except AC-35's catalog code, which awaits STD-112. TC-404 backs FR-003-AC-7
and AC-8, the `format` input retargeted to the `qsl-cst` CST (ADR-011 §7.3
M-6a).

## Value node keys and expression lowering (FR-092, FR-093) coverage

[FR-092](functional/FR-092-key-type-parameter-and-declared-nodes.md) carries
ADR-013 O-04 and OQ-G: QSL's `quire.structural-node/v1` preimage for type,
value, parameter and declared function nodes, with golden vectors T1 to T12,
D1 to D5, P1 to P4, L1 to L3 and F1 to F3, and it keys recursion groups by a
content order, with vectors L5, L6, E11 to E13 and G1 to G15.
TC-413 and TC-414 back its twelve ACs.
[FR-093](functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
carries the lowering of checked expressions to FR-322 application nodes, with
vectors E1 to E3 and the ownership split: `check` lowers and keys, and the
S4 emitter serializes. Its text-leaf walk ends a recursive
composite in a recursion leaf, with vectors T13, T14, G16 to G21,
S4, S5, P10 to P16 and E14 to E17. TC-415 backs AC-1 to AC-6, AC-8, AC-10,
AC-11, AC-14 and AC-15, and TC-416 AC-7, AC-9, AC-12 (each node's `dependencies`)
and AC-13 (the comparison with QSpec's v2 positive fixtures). TC-413 to TC-415 are implemented (#384) and pass locally;
TC-416 is partial (see its row). No FR-093 AC backs the `Pre` row; the
`ProtocolClause` postcondition lowering backs it. Remaining work: #218.
[FR-094](functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
keys the nodes those two leave to the model layer: model declaration nodes
and clause functions under QSpec's `ModelOwner`, the `Reference<T>` and
`Population<T>[N]` type nodes, and quantity type nodes for declared and
compound units, with vectors M1, M3 to M5, R1, R3 to R5, S1 to S3, PO1 to
PO3, P5 to P8, L4, E4 to E10, C1, C2, C4 to C6 and U1 to U4. TC-417 backs AC-1 to AC-4 and AC-7, TC-418
AC-5 and TC-419 AC-6 and AC-7. All three are implemented (#384) and
pass locally.

## Finite simulation (FR-101) coverage

[FR-101](functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
is QSL's implementation of QSpec FR-181: canonical successor order, the
typed state key and its `quire.simulation.state-key/v1` digest, the
`quire.simulation.sampler/v1` generator, replay, the `cancelled`/`caller-cancelled`
cause, limit exhaustion and the pre-exploration `requires-bound`. TC-453
backs AC-1, AC-2 and AC-9, TC-454 AC-3 to AC-5 and AC-10, and TC-455 AC-6 to AC-8,
all implemented. The simulation tests in
`qsl-eval/tests/it/finite_simulation.rs` trace to these ids, not to QSpec's
TC-210 and FR-181-AC-* ids; this repository's TC-210 is a different case.
TC-439 keeps FR-097-AC-5's `Outcome::category()` map.

## State-space reduction (FR-150 to FR-162) coverage

FR-150 to FR-162 carry ADR-021: the `symmetric` annotation and its S3
refusals (FR-150), symmetry admission and instance orbits (FR-151), the sort
canonicaliser (FR-152), `each` fairness on the annotated quotient (FR-153),
receiver-scoped frames (FR-154), footprints and their enforcement (FR-155),
ample sets and the breadth-first proviso (FR-156), fairness visibility
(FR-157), state constraints (FR-158), the preservation-table pre-check
(FR-159), reduced verdicts and causes (FR-160), counterexample
concretisation (FR-161) and the `TransitionSystem` hooks (FR-162). TC-565 to
TC-589, TC-617 and TC-886 back every AC, all `🚧 Planned`. TC-886 is the
reduced-versus-unreduced differential test over the corpus. TC-578 is the vector where fairness
visibility changes the ample set and keeps a fair violation that C0 to C3
alone would discard. TC-579's protocol step needs ADR-027's
`ProtocolSystem`.

## Model simulation (FR-120) coverage

[FR-120](functional/FR-120-simulate-a-checked-package-s-state-family.md)
supplies `ModelSystem`, the model-level `TransitionSystem` for FR-101's
engine over a checked package's state family: QSpec FR-181's successor
relation over QSpec FR-013 frames (through FR-114's frame binding and
FR-115's frame check), invariant-violating successors recorded rather than
pruned (FR-181-AC-4), and effects and results as typed trace data with
ambient-state reads refused at S3 (FR-181-AC-6). TC-471 backs AC-1 to AC-4,
TC-472 AC-5 to AC-9 and TC-473 AC-10 to AC-12, all `🚧 Planned`. Their bound scalar fixtures use FR-056's scalar reader,
on main.
TC-474 backs FR-101-AC-12 to AC-14 and FR-097-AC-5's `Stopped` arm, the
engine's findings and stopped expansions. The tests build the S4 package in
memory.

## State clauses on the spine (FR-102 to FR-109) coverage

FR-102 to FR-109 carry ADR-012 §15, the state share of the #220
mapping: S2 state clause forms (FR-102),
operations and frames at I1 and E3 (FR-103), the S3 `ProtocolClause` check
and its requirement records (FR-104), S4 `state` node emission (FR-105), the
snapshot and invocation input (FR-106), S6a clause evaluation (FR-107), the
ConfigVersion spine corpus (FR-108) and the layer-6 run entry (FR-109).
TC-456 to TC-469 back every AC; each row above carries its own status. TC-469's native parity step retires when M-6c retires the
`0-draft` native path (ADR-012 §15.8); its check against the independent
expected dispositions stays.

## Protocol frames and scoped anchors (FR-112 to FR-116) coverage

FR-112 to FR-116 carry ADR-012 §12.2: S2 scoped
anchor forms (FR-112), S3 anchor resolution through nested scopes with the
missing, ambiguous and shadowing refusals (FR-113), a protocol attempt bound
to its operation's one FR-105 frame node (FR-114), the `Frame` run selection
and its `frame_violation` (FR-115), and frame counterexample replay (FR-116).
TC-510 to TC-515 back every AC. TC-514 and TC-515 have passed
locally; the rest are `🚧 Planned`. TC-513's emitted-node
assertions wait on STD-111, as TC-462's do. FR-116's decode of a Kani frame
witness waits on agent-ix/quire-contract-ir#109 and
agent-ix/quire-contract-codegen#49; TC-515 drives a hand-built envelope.

## StateModel family and abstraction relation (FR-300 to FR-307) coverage
[FR-300](functional/FR-300-check-model-forms-through-the-state-model-family.md)
to [FR-303](functional/FR-303-keep-the-model-correspondence-one-to-one.md)
carry ADR-016 G-2a to G-2c, G-7 and G-8: the `StateModel` check hook and its
seams (TC-790, TC-791), `state-model` causes under the enclosing family's
evaluation (TC-792), dispatch linked at S3 during unit compile (TC-793 to
TC-795, TC-809) and the one-to-one model correspondence (TC-796).
[FR-304](functional/FR-304-check-an-authored-abstraction-relation.md) to
[FR-307](functional/FR-307-export-the-bindings-each-item-references.md)
carry ADR-017 AR-1 to AR-6, QSL's share of QSpec FR-353 (QSpec TC-268): the
S3 check (TC-797 to TC-801, TC-810), the S2 source form (TC-811), the frame binding key (TC-802, TC-803), the v2
relation node (TC-804, TC-805) and the per-item export (TC-806, TC-807).
TC-797 to TC-803 and TC-806 to TC-807 build the relation input in the test
from checked packages; TC-804 and TC-811 use the spelling and node
QSpec FR-450 and FR-451 give. Every row is `🚧 Planned`.
## Infinite-trace profile and unbounded declarations (FR-325 to FR-336) coverage
FR-325 to FR-332 carry ADR-014 §3 and §5 as amended by ADR-018: S2
temporal forms with an optional interval (FR-325), S3 admission by the
unit's temporal profile (FR-326), the layer-5 evaluator over a finite trace
(FR-327), over an infinite-trace prefix (FR-328) and over a lasso by
ADR-018 SM-8 (FR-329), the spine run entry (FR-330), observed-trace
counterexample replay (FR-331) and negotiated settlement (FR-332). FR-333
to FR-336 carry ADR-014 §2, §4 and §9 and ADR-018 §1: the optional
collection bound and population maximum (FR-333), the root definitions'
identity preimage (FR-334), end-to-end settlement over an unbounded
declaration (FR-335) and a model subject's universes (FR-336). TC-835 to
TC-846 back every AC, with TC-847 (undefined letters) and TC-848 (the
missing fairness premise), all `🚧 Planned`. TC-842 and TC-845 run downstream
of CG `negotiate_*` with test descriptors.
## Refinement gates (FR-340 to FR-345) coverage
FR-340 to FR-345 carry ADR-017 §2: the spec-versioning gate's corpus and
case runs (FR-340), the shared compile classification (FR-341), the run
classification (FR-342), the prior/superseding comparison (FR-343), the
report, verdict and exit (FR-344), and the profile-layering comparison
(FR-345). TC-860 to TC-868 back every AC; all are `🚧 Planned`. Each
expected class, result and verdict is a literal in its test, never computed
with the gate's classification or comparison code. The seeds and controls
live in a test-only corpus the gate's real run over
`tests/fixtures/refinement/versioning/` does not read. TC-868's corpus
covers all five QSpec AD-003 `requires` edges (QSpec FR-453).
## Sum types and `case` (FR-313 and FR-316 to FR-324) coverage
TC-815 to TC-834 back every AC of FR-313 and FR-316 to FR-324 and FR-046-AC-9. All are
`🚧 Planned`. QSpec TC-262 to TC-265 run inside TC-817 to TC-819, TC-822,
TC-826 and TC-831 under `QSpec-` tags. TC-827's wire assertions,
TC-830's v2 `union_value` spelling, TC-832's charge points and TC-833's
union-key order read QSpec's union spelling (FR-440, FR-441), accounting
(FR-143, FR-146, `value-accounting.md`) and key rules (FR-144).

| TC-742 | native-run-result/2 carries basis and witness, and its strict reader refuses malformed and /1 documents | Integration | P1 | FR-267-AC-1, FR-267-AC-2, FR-267-AC-3 | 🚧 Planned |
| TC-743 | Replay re-derives a state-clause witness and checks that the element separates the clause | Integration | P1 | FR-268-AC-1, FR-268-AC-2, FR-268-AC-3, FR-268-AC-4 | 🚧 Planned |
| TC-744 | A witness disagreement settles inconclusive with a typed Witness cause that round-trips | Unit | P1 | FR-269-AC-1, FR-269-AC-2, FR-269-AC-3 | 🚧 Planned |
| TC-891 | A clause-run request file runs through run_clause and writes a /2 document | Integration | P1 | FR-312-AC-1, FR-312-AC-2, FR-312-AC-3, FR-312-AC-4 | 🚧 Planned |
| TC-779 | Each non-interpreter backend matches the interpreter over the conformance corpus and generated programs | Integration | P1 | FR-295-AC-1 | 🚧 Planned |
| TC-780 | Backends match the interpreter on budgets, limits and internal errors, and the outcome names no backend | Integration | P1 | FR-295-AC-2, FR-295-AC-3, FR-295-AC-4 | 🚧 Planned |
| TC-781 | The request selects the execution backend and no other backend runs in its place | Integration | P1 | FR-296-AC-1, FR-296-AC-2, FR-296-AC-3 | 🚧 Planned |
| TC-778 | The interpreter implements the execution seam, and the seam admits checked input only | Unit | P1 | FR-294-AC-1, FR-294-AC-2, FR-294-AC-3, FR-294-AC-4 | 🚧 Planned |
| TC-773 | Installing, removing or reordering providers changes nothing outside the touched candidate sets | Unit | P1 | FR-289-AC-1, FR-289-AC-2, FR-289-AC-3 | 🚧 Planned |
| TC-774 | Plugin results settle as typed records, and plugin proofs carry the trusted label | Unit | P1 | FR-290-AC-1, FR-290-AC-2, FR-290-AC-3, FR-290-AC-4 | 🚧 Planned |
| TC-775 | Plugin-run failures settle as typed records of the plugin's own items | Unit | P1 | FR-291-AC-1, FR-291-AC-2, FR-291-AC-3 | 🚧 Planned |
| TC-772 | One manifest conversion serves compile-time providers and plugins | Unit | P1 | FR-288-AC-1, FR-288-AC-2, FR-288-AC-3, FR-288-AC-4, FR-288-AC-5, FR-288-AC-6 | 🚧 Partial: AC-5 and AC-6 passed locally in qsl-route; AC-1 to AC-4 planned in the driver (IR-609) |
| TC-764 | Certificate checkers accept genuine certificates and reject altered or misbound ones in a core-only process | Integration | P1 | FR-282-AC-1, FR-282-AC-2, FR-282-AC-3, FR-282-AC-4, FR-282-AC-5, FR-282-AC-6, FR-282-AC-7, FR-282-AC-8 | 🚧 Planned |
| TC-765 | monitor reports violations, pending and tested verdicts over finite traces and lassos | Unit | P1 | FR-283-AC-1, FR-283-AC-2, FR-283-AC-5 | 🚧 Planned |
| TC-766 | monitor refuses inadmissible traces and agrees with replay's trace evaluation | Unit | P1 | FR-283-AC-3, FR-283-AC-4, FR-283-AC-6 | 🚧 Planned |
| TC-767 | The direction check keeps frontends, analyze, the cache and the plugin host out of the qualified core | Integration | P1 | FR-284-AC-1, FR-280-AC-3 | ✅ Passing (`tools/arch-lint` `qualified_core` tests; `arch-lint qualified-core` in `make ci`) |
| TC-768 | Core operations read no ambient input and write nothing to the process streams | Integration | P1 | FR-284-AC-2, FR-284-AC-3, FR-284-AC-4 | 🚧 Partial: step 3 (FR-284-AC-4) passes (`tools/arch-lint` `qualified_core` tests; `arch-lint qualified-core` in `make ci`); steps 1 and 2 wait on FR-275's typed lifecycle API |
| TC-785 | The compile command is the composition of the front-end operations | Unit | P1 | FR-027-AC-11 | 🚧 Planned |
| TC-786 | run's exit statuses come from the exit function, with undefined at 10 | Unit | P1 | FR-100-AC-10, FR-100-AC-11 | ✅ Passed locally |
| TC-770 | Library outcomes serialize to one outcome document, with the undefined label on non-proof outcomes only | Unit | P1 | FR-286-AC-1, FR-286-AC-2, FR-286-AC-3, FR-286-AC-4, FR-286-AC-5, FR-357-AC-12 | 🚧 Partial: steps 1, 2 and 5 over real outcomes; steps 3 and 4 build their `analyze` and `monitor` items with the public outcome builders until QSL-596 (FR-281) and QSL-597 (FR-283) re-run them over real outcomes |
| TC-771 | QSL builds no binary, and command is a library operation that writes nothing | Integration | P1 | FR-287-AC-1, FR-287-AC-2 | 🚧 Planned |
| TC-769 | The exit function maps every category and multi-item outcome, with undefined at 10 | Unit | P1 | FR-285-AC-1, FR-285-AC-2, FR-285-AC-3, FR-285-AC-4, FR-285-AC-5 | 🚧 Partial: steps 2 to 4 pass locally. Step 1 passes for the bare categories and the `refuted` terminal record; the inconclusive `analyze` item with cause `CertificateRejected` (FR-281) and FR-283-AC-1's pending supplied-trace clause are not built. Step 3 passes for the existing variants, for `StageFailure::Cancelled` and `CallFailure::Cancelled` (exit 22) and for `StageFailure::Fault` (internal failure). Step 5 passes for `run`: the source byte limit at 1 and `s3.nodes` at 1 each refuse `stage_limit_exceeded` in category incomplete, exit 22 |
| TC-776 | analyze requests and records have one canonical form that changes only with key members | Unit | P1 | FR-292-AC-1, FR-292-AC-2, FR-292-AC-3 | 🚧 Planned |
| TC-777 | QSL records tell the cache what it never stores, and hold no wall time | Unit | P1 | FR-293-AC-1, FR-293-AC-2 | 🚧 Planned |
| TC-758 | Every bound is a caller limit that names itself when reached | Unit | P1 | FR-277-AC-1, FR-277-AC-2, FR-277-AC-3 | 🚧 Partial: step 1 passes for the fields of the source, model-normalization (but `dispatch_candidates` and `family_steps`, which no spine operation reaches), type-environment, dependency (`dependency.libraries`, `dependency.import_edges`, `dependency.source_bytes`) and checking limits, over `parse`, `select` and `check`; the other operations' limits belong to their own requirements, step 2 belongs to the ADR-030 D-1 depth work (the B-series), and step 6 passes for `execute`'s `work_units` (cumulative) and the high-water counter rule |
| TC-759 | The front-end operations compose to spine compile | Unit | P1 | FR-278-AC-1, FR-278-AC-2, FR-278-AC-3 | ✅ Implemented: the I2 read-back and the refusals run over the operations in `qsl-replay`, and the comparison with the `compile` command's bytes runs in `tests/it/compile_command.rs` |
| TC-756 | No lifecycle operation panics on arbitrary input | Property | P1 | FR-275-AC-4 | 🚧 Partial: a deterministic byte generator drives `parse`, `select`, `check` and `package`; `monitor` (FR-283) and `replay` (FR-098) take no byte input yet |
| TC-757 | A cancelled operation stops within one charge and emits nothing | Unit | P1 | FR-276-AC-1, FR-276-AC-2, FR-276-AC-3, FR-276-AC-4, FR-276-AC-5 | 🚧 Partial: steps 1 to 3 and 5 pass for the operations that exist, including a cancel inside one large body, inside `select` and inside `package`; step 2 runs at 200,000 declarations; step 4 needs `analyze` (FR-281) |
| TC-760 | execute returns what CheckedPackage::call and evaluate return | Unit | P1 | FR-279-AC-1, FR-279-AC-2, FR-279-AC-3 | 🚧 Planned |
| TC-761 | QSL engine manifests register, and the provider entry returns analyze's records | Unit | P1 | FR-280-AC-1, FR-280-AC-2 | 🚧 Planned |
| TC-782 | inspect builds each typed view from the source map | Unit | P1 | FR-297-AC-1, FR-297-AC-2, FR-297-AC-3, FR-297-AC-4 | 🚧 Planned |
| TC-783 | render produces text and DOT from typed values only, leaving JSON unchanged | Unit | P1 | FR-298-AC-1, FR-298-AC-2, FR-298-AC-3, FR-298-AC-4 | 🚧 Planned |
| TC-784 | One Value-family source runs through check, package and execute with documents and exit codes | Integration | P1 | FR-299-AC-1, FR-299-AC-2, FR-299-AC-3, FR-299-AC-4 | 🚧 Planned |
| TC-762 | analyze settles proved, refuted and unsupported items with one record each | Integration | P1 | FR-281-AC-1, FR-281-AC-2, FR-281-AC-3 | 🚧 Planned |
| TC-763 | analyze settles rejected certificates, unreproduced counterexamples and budgets without a false verdict | Integration | P1 | FR-281-AC-4, FR-281-AC-5, FR-281-AC-6, FR-281-AC-7 | 🚧 Planned |

## Arbitrary nesting depth (FR-255 to FR-264, FR-356) coverage
FR-255 to FR-264 carry ADR-030: limit outcomes that name the setting which
raises them, uniform across the library builder, the replay request and
the settings operation the driver CLI's `--limit` calls (FR-255), and for each component the iterative walk, the resource
limit that bounds it and its deep-input criteria: S1 (FR-256), S2 (FR-257),
S3 (FR-258), identities and QSL's use of `quire-canonical`
(FR-259), semantic-IR intake (FR-260), the other untrusted JSON reads
(FR-261), the evaluator (FR-262), replay (FR-263) and the v2 wire (FR-264).
FR-356 is the walker toolkit those walks run on (the crate
`quire-walk` in `agent-ix/quire-walk`, which holds all of its FR-356 ACs and
TC-898 and TC-899), the qualified core's
iterative-only rule with its deep entry-point tests, the `maybe_grow`
wrapper outside the core, and the deep-input fuzz target. TC-720 to TC-739,
TC-902 and TC-903 back every AC that stays here (AC-1 is backed by arch-lint's `tc_arch_lint_metadata_009` and `tc_arch_lint_direction_004`), all `🚧 Planned`. Each 100,000-deep case
sets its own limits and runs on a thread with a 512 KiB stack. TC-729 waits
on the `quire-canonical` capabilities FR-259 names, TC-730 and TC-731 on the
semantic-IR crate's move onto that reader, and TC-739's malformed-wire step
on IR's reader enforcing QSpec FR-322's stratified body grammar.
