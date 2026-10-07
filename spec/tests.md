---
id: TM-001
title: "Root test matrix"
type: TestMatrix
---

## Overview

The root matrix indexes the test cases that no area matrix owns.

## Requirements Traceability

A `#[trace]` tag with a bare id (`TC-196`, `FR-075-AC-4`) names a QSL
artifact. A tag that names a quire-specification requirement or test case
carries the `QSpec-` prefix (`QSpec-TC-196`, `QSpec-FR-151-AC-2`), because
the two repositories number their artifacts independently and the same id
names different artifacts in each.

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-059 | FR-059-AC-1 | TC-156 |  |
| FR-059 | FR-059-AC-2 | TC-156 |  |
| FR-059 | FR-059-AC-3 | TC-156 |  |
| FR-059 | FR-059-AC-4 | TC-156 |  |
| FR-059 | FR-059-AC-5 | TC-156 |  |
| FR-059 | FR-059-AC-6 | TC-156 |  |
| FR-059 | FR-059-AC-8 | TC-156 |  |
| FR-059 | FR-059-AC-9 | TC-156 |  |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-156 | Report FB-05 and FB-11 violations over the four-repository backend dependency graph | Integration | P1 | FR-059-AC-1..FR-059-AC-6, FR-059-AC-8, FR-059-AC-9 | ✅ |
| TC-157 | Report pending, passing and failing T-12 API-surface rules | Integration | P1 | FR-060-AC-1..FR-060-AC-4 | ✅ |
| TC-160 | Every family implements the six-part checked contract with no bypass | Unit | P1 | FR-062-AC-1..FR-062-AC-7, FR-062-AC-9, FR-062-AC-13, FR-057-AC-10 | ✅ |
| TC-161 | The seam probe demonstrates exhaustiveness at every S1-S4 seam | Integration | P1 | FR-063-AC-1..FR-063-AC-7, FR-062-AC-8, FR-067-AC-4 | ✅ |
| TC-162 | The string-edge scan reports every unmarked string dispatch | Integration | P1 | FR-064-AC-1..FR-064-AC-6 | ✅ |
| TC-163 | Function identity and provenance survive checking and package conversion | Integration | P1 | FR-065-AC-1..FR-065-AC-3, FR-065-AC-8 | ✅ |
| TC-164 | A call receives the same verdict from a declaration body and from a clause expression | Integration | P1 | FR-065-AC-5 | ✅ |
| TC-165 | The migration recipe names every required test, conversion, removal condition and remaining family | Manual | P1 | FR-066-AC-1..FR-066-AC-4 | ✅ |
| TC-166 | The replay executor selects a function by typed QualifiedName, never by string | Unit | P1 | FR-062-AC-10, FR-065-AC-6 | ✅ |
| TC-167 | The S2 forms stage refuses on a recovering CST and mints no identity | Unit | P1 | FR-067-AC-1, FR-067-AC-2, FR-067-AC-3, FR-067-AC-7, FR-067-AC-8 | ✅ |
| TC-168 | SEAM-5 (LoweredSourceGraph) is deleted in the same change as the S2 forms core | Integration | P1 | FR-067-AC-5, FR-067-AC-6 | ✅ |
| TC-170 | check-stage modules move to check with no duplication | Unit | P1 | FR-068-AC-1, FR-068-CON-1, FR-068-CON-3 | ✅ |
| TC-171 | CheckedPackage/CheckedExpression/CheckedFunction constructors are private to check | Manual | P1 | FR-068-AC-2 | ✅ |
| TC-172 | check's real import graph has no edge into value::expression or into checking, and value::expression re-exports no check item | Integration | P1 | FR-068-AC-3, FR-068-AC-10 (retired) | ✅ |
| TC-173 | Refusal split: check causes in check, InputRefusal in value::expression | Unit | P1 | FR-068-AC-4, FR-068-AC-8 | ✅ |
| TC-174 | Checking and evaluation produce identical results before and after the split | Integration | P1 | FR-068-AC-5 | ✅ |
| TC-175 | The move stays inside M-5: no early M-2 work, no edge widening | Integration | P1 | FR-068-AC-6, FR-068-AC-7 | 🚧 |
| TC-176 | The interim `model` -> `check` edge stays bounded to two files and thirteen names, imported directly | Unit | P1 | FR-068-AC-9, FR-068-CON-5 | ❌ |
| TC-193 | Candidate set matches registered backends advertising the requested kind | Unit | P1 | FR-075-AC-1, FR-075-AC-5 | ✅ |
| TC-194 | Registry candidate sets are invariant under registration-order permutation | Property | P1 | FR-075-AC-2, FR-075-AC-4, FR-075-AC-7, FR-080-AC-1 | 🚧 |
| TC-195 | An unregistered named backend yields a distinct unknown-backend marker | Unit | P1 | FR-075-AC-3 | ✅ |
| TC-196 | A conflicting backend identity registration refuses both and withdraws the held registration | Unit | P1 | FR-075-AC-4 | ✅ |
| TC-433 | A backend member is its identity, kept verbatim | Unit | P1 | FR-075-AC-6 | ✅ |
| TC-447 | Duplicate backend identity registration matches quire-specification TC-282 under every order | Unit | P1 | FR-075-AC-4, FR-075-AC-7 | 🚧 |
| TC-448 | An identical repeat registration is idempotent | Unit | P1 | FR-075-AC-7 | ✅ |
| TC-449 | The request builder writes one item per requirement record | Unit | P1 | FR-075-AC-8 | ✅ |
| TC-197 | Empty candidate set carries the data an unsupported warning needs | Unit | P1 | FR-076-AC-1, FR-076-AC-2 | ✅ |
| TC-198 | Backend absence never settles as a refusal or a hold at the registry | Unit | P1 | FR-076-AC-3 | ✅ |
| TC-199 | requests::report takes no Backend parameter and has no capability/family disposition | Unit | P1 | FR-077-AC-1, FR-077-AC-2 | ✅ |
| TC-200 | Requests are still recorded as data after negotiation removal | Unit | P1 | FR-077-AC-3 | ✅ |
| TC-201 | value::ieee and value::division carry no negotiate_* function | Unit | P1 | FR-078-AC-1, FR-078-AC-2 | ✅ |
| TC-202 | value::ieee and value::division evaluation is unchanged by negotiate_* removal | Unit | P1 | FR-078-AC-3 | ⛔ |
| TC-205 | cargo-deny denies inventory, linkme and ctor | Integration | P1 | FR-080-AC-2 | ✅ |
| TC-206 | The registry module lint gate finds no static, OnceLock or thread_local | Integration | P1 | FR-080-AC-3 | ✅ |
| TC-207 | One unit test exists and passes per ADR-012 §5.2 row | Unit | P1 | FR-080-AC-4 | ✅ |
| TC-208 | The S7 seam probe fails to compile the registry arm on an unhandled capability-kind variant | Integration | P1 | FR-080-AC-5 | 🚧 |
| TC-177 | The proof-result envelope maps every FR-331 outcome to its exact O-16 category | Property | P1 | FR-069-AC-1 | ✅ |
| TC-178 | The proof-result reader refuses an oversized envelope | Unit | P1 | FR-069-AC-4 | ✅ |
| TC-179 | A positive proof-result envelope round-trips its backend identity and dispositions exactly | Unit | P1 | FR-069-AC-3 | ✅ |
| TC-180 | The witness envelope stores the transcript once and derives every other fact from it | Unit | P1 | FR-070-AC-1 | ✅ |
| TC-181 | The witness envelope refuses a malformed transcript, an out-of-domain digest, or an oversized encoding | Property | P1 | FR-070-AC-2, FR-070-AC-6, FR-070-AC-7 | ✅ |
| TC-182 | A positive witness envelope round-trips its transcript and every O-25 member exactly | Unit | P1 | FR-070-AC-3 | ✅ |
| TC-183 | The witness envelope refuses reconstruction when any one O-25 member is missing | Property | P1 | FR-070-AC-4 | ✅ |
| TC-184 | A family adds a typed witness payload through a typed extension point, not an untyped map | Unit | P1 | FR-070-AC-5 | ✅ |
| TC-185 | The replay request carries exactly the O-26 members and round-trips them exactly | Unit | P1 | FR-071-AC-1 | ✅ |
| TC-186 | The replay request's byte provision is reachable only by digest, never by path, is complete, and stays within the size bound | Property | P1 | FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7, FR-071-AC-9, FR-071-AC-10, FR-071-AC-11 | ✅ |
| TC-187 | The replay request's function selection accepts only a typed QualifiedName, never a bare string | Unit | P1 | FR-071-AC-3 | 🚧 |
| TC-189 | The replay result keeps the Witness arm and Input arm distinct, each with its own settlement | Unit | P1 | FR-072-AC-1 | ✅ |
| TC-190 | A replay disagreement settles inconclusive with a typed cause and is never repairable | Unit | P1 | FR-072-AC-2 | ✅ |
| TC-191 | A replay result's nested witness record round-trips exactly, compares without display-text interpretation, and refuses an oversized encoding | Unit | P1 | FR-072-AC-3, FR-072-AC-5 | ✅ |
| TC-192 | #217's function exemplar builds on the existing result/request/witness types with no new type | Integration | P1 | FR-072-AC-4 | 🚧 |
| TC-209 | The witness envelope's Debug and Display rendering never reproduces the full transcript | Unit | P1 | FR-073-AC-1 | ✅ |
| TC-210 | The replay request's Debug and Display rendering never reproduces a byte-provision entry's raw bytes | Unit | P1 | FR-073-AC-2 | ✅ |
| TC-211 | A refusal cause from any of the four envelopes renders with no unredacted transcript, byte or value content, while the typed accessor stays fully readable | Unit | P1 | FR-073-AC-3 | ✅ |
| TC-213 | Original declaration keys survive normalization unchanged | Unit | P1 | FR-081-AC-1 | ❌ |
| TC-214 | Structurally identical declarations from distinct originals never collapse to one effective identity | Unit | P1 | FR-081-AC-2 | 🚧 |
| TC-215 | A dominated redefinition is retained for provenance, not deleted | Unit | P1 | FR-081-AC-3 | ✅ |
| TC-216 | Changing only a display name leaves every key, identity and ordering unchanged | Property | P1 | FR-081-AC-4 | 🚧 |
| TC-217 | The model binder is a pure function with no cross-run ambient state | Unit | P1 | FR-081-AC-5 | 🚧 |
| TC-218 | Redefinition variance checking reports every failing axis, not only the first | Unit | P1 | FR-082-AC-1 | ✅ |
| TC-219 | A missing redefinition target and a supertype cycle each refuse with a named cause | Unit | P1 | FR-082-AC-2, FR-082-AC-6 | ✅ |
| TC-220 | A conformance ancestor walk that exhausts its edge limit stops incomplete instead of truncating | Unit | P1 | FR-082-AC-3, FR-082-AC-6, FR-082-AC-7 | ✅ |
| TC-221 | A narrowing field redefinition requires an established postcondition | Unit | P1 | FR-082-AC-4 | ✅ |
| TC-222 | Dispatch selects the descendant candidate independent of declaration order | Property | P1 | FR-083-AC-1 | ✅ |
| TC-223 | No-applicable-candidate and multiple-undominated-candidates are named separately | Unit | P1 | FR-083-AC-2 | ✅ |
| TC-224 | An ambiguous family produces no dispatch table, not even for subtypes that resolved cleanly | Unit | P1 | FR-083-AC-3 | ✅ |
| TC-225 | A dispatch family walk that exhausts its edge limit stops incomplete instead of reporting a false unique winner | Unit | P1 | FR-083-AC-4 | ✅ |
| TC-226 | Population admission distinguishes unknown closure from a genuine refusal | Unit | P1 | FR-084-AC-1 | ✅ |
| TC-227 | allInstances requires both object and subtype closure, never a partial set | Unit | P1 | FR-084-AC-2 | 🚧 |
| TC-228 | lookup returns the declared absence mode for a genuinely unmatched key | Unit | P1 | FR-084-AC-3 | ✅ |
| TC-229 | Two members sharing universe and object identity but differing type refuse admission outright | Unit | P1 | FR-084-AC-4 | ✅ |
| TC-230 | A relationship end naming an undeclared type refuses the whole relationship record | Unit | P1 | FR-085-AC-1 | 🚧 |
| TC-231 | A resolved relationship end retains its declared role and multiplicity exactly | Unit | P1 | FR-085-AC-2 | 🚧 |
| TC-232 | A relationship between two object types carries no systems-model kind | Unit | P1 | FR-085-AC-3 | 🚧 |
| TC-233 | Systems classification reports every no-kind cascade, not only the first | Unit | P1 | FR-086-AC-1 | 🚧 |
| TC-234 | Connection admission checks all three conditions independently and reports every failure | Unit | P1 | FR-086-AC-2 | 🚧 |
| TC-235 | An allocation whose target does not classify as Part refuses wrong-export | Unit | P1 | FR-086-AC-3 | ✅ |
| TC-236 | Two declarations sharing a display title classify and resolve independently by key | Unit | P1 | FR-086-AC-4 | 🚧 |
| TC-237 | A derivation conflict with no descendant redefiner exposes no effective member for either path | Unit | P1 | FR-081-AC-6 | ✅ |
| TC-238 | Replaying normalization over a permuted IR node order reproduces the same correspondence | Property | P1 | FR-081-AC-7 | ✅ |
| TC-239 | An arity mismatch refuses without checking per-parameter axes, while result and effect axes are still checked | Unit | P1 | FR-082-AC-5 | ✅ |
| TC-240 | allInstances and lookup return the FR-153 typed result shape and its bound/foreign/ineligible refusals | Unit | P1 | FR-084-AC-5 | 🚧 |
| TC-241 | Kind mapping runs interfaces first, and a port whose interface type is not an Interface refuses wrong-export | Unit | P1 | FR-086-AC-5 | 🚧 |
| TC-242 | A selected object's reference key names the same most-specific type through every conforming query | Unit | P1 | FR-084-AC-6 | ✅ |
| TC-243 | Typestate constructors are private to their stage module | Manual | P1 | FR-087-AC-1 | 🚧 |
| TC-244 | compile_fail matrix over every forbidden typestate construction (R-10, O-15) | Unit | P1 | FR-087-AC-2 | ✅ |
| TC-245 | PackageNodeKey has exactly one shape and declared equality | Unit | P1 | FR-087-AC-5 | ✅ |
| TC-246 | ResolvedSourcePackage is retired, with no dangling caller | Integration | P1 | FR-087-AC-7, FR-087-CON-4 | ✅ |
| TC-247 | The canonical EmittedPackage/CheckedPackage stay distinct from their pre-existing namesakes | Integration | P1 | FR-087-AC-8, FR-087-AC-10 | 🚧 |
| TC-248 | Frame identity's subject sets resolve to DeclarationKey through the model correspondence | Unit | P1 | FR-088-AC-2 | ✅ |
| TC-249 | Clause identity is the checked node id; the occurrence key disambiguates structurally identical clauses | Unit | P1 | FR-088-AC-3 | ✅ |
| TC-250 | Clause-kind wire-string totality in both directions, with mutation coverage | Property | P1 | FR-088-AC-4 | ✅ |
| TC-251 | Qualified-name resolution is confined to the check stage (R-06) | Integration | P1 | FR-088-AC-5 | ✅ |
| TC-252 | Checked type node to kernel ValueType is total, tested per type-node form including a sum | Unit | P1 | FR-088-AC-9, FR-088-AC-10 | ✅ |
| TC-253 | The verified binding admits VerifiedPackage only under all three conditions, refusing each failure independently | Integration | P1 | FR-087-AC-3, FR-087-AC-14 | ✅ |
| TC-254 | library converts VerifiedPackage to ImportView without resolving any name | Unit | P1 | FR-087-AC-4 | ✅ |
| TC-255 | NodeKey is never minted from a WireNodeId; E4 and E9 resolve by lookup, never by construction | Integration | P1 | FR-087-AC-6 | 🚧 |
| TC-256 | check and package's dependency edge is one direction, and CheckedPackage wraps CheckedGraph | Integration | P1 | FR-087-AC-9 | ✅ |
| TC-257 | Exactly one closed checked clause-kind enum exists, and syntax::ClauseKind gains no variant | Unit | P1 | FR-088-AC-1 | ✅ |
| TC-258 | QualifiedName is used only as a declared preimage component, never as an identity | Manual | P1 | FR-088-AC-6 | 🚧 |
| TC-259 | A package type's identity is its checked node id, scoped only by owner | Unit | P1 | FR-088-AC-7 | 🚧 |
| TC-260 | ValueTypeRef is exactly the two-member union Native/Package | Manual | P1 | FR-088-AC-8 | 🚧 |
| TC-261 | M-2's items are relocated to check and absent from model | Unit | P1 | FR-074-AC-1, FR-074-AC-2 | ✅ |
| TC-262 | The model -> check edge is fully closed after M-2 | Unit | P1 | FR-074-AC-3 | ✅ |
| TC-281 | value::library and value::package_identity relocate into the new top-level library module, per the R-10/T-3 shapes | Integration | P1 | FR-087-AC-11 | 🚧 |
| TC-282 | Every resolve_libraries refusal classifies to an I2 rule, a §4 condition, E3 resolution, or a named exception | Unit | P1 | FR-087-AC-12 | ✅ |
| TC-291 | PopulationId is deterministic over its admission preimage and distinguishes distinct admissions | Unit | P1 | FR-089-AC-1, FR-089-AC-7 | ✅ |
| TC-293 | The evaluator resolves a Value::Population identity through the recorded correspondence, not a carried payload | Unit | P1 | FR-089-AC-3 | ✅ |
| TC-294 | An unresolved PopulationId refuses with a typed cause, not a panic or Undefined | Unit | P1 | FR-089-AC-4 | ✅ |
| TC-295 | The QSL layer admits a Value::Population identity under ValueType::Population by its resolved binding's declared maximum | Unit | P1 | FR-089-AC-5 | ✅ |
| TC-296 | A standalone Direct admission and an invocation's Post binding over the same domain package and population_key mint distinct PopulationIds | Unit | P1 | FR-089-AC-1 | ✅ |
| TC-376 | Function application checking accepts a well-typed call and refuses wrong arity, an unknown name and a type mismatch | Unit | P1 | FR-065-AC-4 | ✅ |
| TC-377 | Function declaration checking accepts a well-typed declaration, reports its calls, and refuses an ill-typed body | Unit | P1 | FR-065 | ✅ |
| TC-378 | On a nested fixture, the node limit is the proximate cause of a function-declaration stop | Unit | P1 | FR-062-AC-7, FR-096-AC-11 | 🚧 |
| TC-379 | E3 resolves an imported name to its PackageNodeKey and refuses a missing or ambiguous one | Unit | P1 | FR-087-AC-13 | 🚧 |
| TC-380 | The function-declaration contract check refuses an ill-typed declaration and admits a well-typed one | Unit | P1 | FR-065-AC-7 | ✅ |
| TC-381 | The expression-node limit bounds the whole checked package, not each declaration | Unit | P1 | FR-062-AC-11 | ✅ |
| TC-382 | S6a returns each kernel outcome unchanged in FamilyOutcome::Evaluated | Unit | P1 | FR-090-AC-1 | ✅ |
| TC-384 | S6a invariant breaks are InternalFaults, not panics or refusals | Unit | P1 | FR-090-AC-3 | ✅ |
| TC-385 | S6a's input type admits no Relation, and FamilyOutcome has exactly two arms | Unit | P1 | FR-090-AC-4 | ✅ |
| TC-386 | F diagnostic maps every snapshot-cause and model-refusal catalog code to category refusal | Unit | P1 | FR-090-AC-5 | ✅ |
| TC-387 | The ProtocolClause snapshot cause maps each WrongSnapshotCause to wrong_snapshot | Unit | P1 | FR-090-AC-6 | ✅ |
| TC-388 | An evaluation-time wrong-anchor snapshot reaches the caller as a coded QSL refusal, not a kernel refusal | Integration | P1 | FR-090-AC-7 | ✅ |
| TC-389 | A refused model query reaches the caller with the ModelRefusal's own catalog code, not a kernel refusal | Integration | P1 | FR-090-AC-8 | ✅ |
| TC-390 | FamilyOutcome, FamilyResult and EvalOutcome live once in the check core, no lower layer names them, and the check core names no family cause | Unit | P1 | FR-090-AC-9 | ✅ |
| TC-391 | An unresolved or mismatched population argument is refused at admission, and is an InternalFault inside S6a | Unit | P1 | FR-090-AC-10 | ✅ |
| TC-392 | S2 returns one Value form per declaration, in source order, with its span and the unit edition | Unit | P1 | FR-091-AC-1 | ✅ |
| TC-393 | The forms FunctionDeclaration carries its name, using alias, type forms, measure and body | Unit | P1 | FR-091-AC-2 | ✅ |
| TC-394 | Each Value expression construct maps to its Expression variant, with grouping from the CST | Unit | P1 | FR-091-AC-3 | ✅ |
| TC-395 | S2 refuses an inadmissible source and a unit holding a declaration with no dispatch entry | Unit | P1 | FR-091-AC-4, FR-091-AC-5, FR-091-AC-6 | ✅ |
| TC-396 | S2 refuses unrepresented constructs, and the check stage refuses another family's construct with that family's cause | Integration | P1 | FR-091-AC-7, FR-091-AC-8 | ✅ |
| TC-397 | S2 builds every body S1 admits, whatever its depth | Unit | P1 | FR-091-AC-9 | 🚧 |
| TC-398 | The Value form builder depends only on layer 2, layer 1, F and K, and its forms hold no ValueType or NodeKey | Unit | P1 | FR-091-AC-11 | ✅ |
| TC-399 | Source compiled through S1, S2 and the assembler checks and evaluates a called function | Integration | P1 | FR-091-AC-12, FR-091-AC-13 | ✅ |
| TC-400 | The assembler refuses unresolved and ambiguous names, ill-formed bounds and alias cycles, reporting every error | Unit | P1 | FR-091-AC-14, FR-091-AC-15, FR-091-AC-16, FR-091-AC-17 | ✅ |
| TC-401 | The assembler builds record and tuple declarations with check-minted keys and resolves names to them | Unit | P1 | FR-091-AC-18 | 🚧 |
| TC-402 | The assembler lives in the check core and its non-test code has no edge to qsl-cst | Unit | P1 | FR-091-AC-20 | ✅ |
| TC-403 | Every Value Expression node carries the span of its CST node | Unit | P1 | FR-091-AC-10 | ✅ |
| TC-404 | format takes the qsl-cst ParsedSource, formats complete-V1 source and refuses inadmissible input | Unit | P1 | FR-003-AC-7, FR-003-AC-8 | ✅ |
| TC-405 | The assembler admits floating types and refuses unresolved model references | Unit | P1 | FR-091-AC-19, FR-091-AC-23, FR-091-AC-24 | ✅ |
| TC-406 | Each S2 and assembler cause maps to its catalog code with an exhaustive match | Unit | P1 | FR-091-AC-21 | 🚧 |
| TC-407 | A false dispatched precondition reaches the caller as a family-owned undefined result, not a kernel Undefined | Integration | P1 | FR-090-AC-11 | ✅ |
| TC-408 | An absent lookup key reaches the caller as a StateModel undefined result, and an absent-refused lookup as a refusal | Integration | P1 | FR-090-AC-12 | ✅ |
| TC-409 | An enum value's VariantId is its FR-141 member node key, and its rank orders sets and bags | Unit | P1 | FR-088-AC-11 | ✅ |
| TC-410 | Each connected supertype component has its own object universe, and a reference key carries the authored object identity | Unit | P1 | FR-084-AC-7 | 🚧 |
| TC-411 | A quantity UnitId is a declared unit's node key or a compound unit's digest, and the two never compare equal | Unit | P1 | FR-088-AC-12 | ✅ |
| TC-412 | The assembler resolves each using alias to a declared profile selection and refuses an undeclared one | Unit | P1 | FR-091-AC-22 | ✅ |
| TC-413 | Type and declared record nodes key to the structural-node golden vectors, scoped only by owner | Unit | P1 | FR-092-AC-1, FR-092-AC-2, FR-092-AC-3, FR-092-AC-7, FR-092-AC-8, FR-092-AC-9, FR-092-AC-11, FR-092-AC-12 | ✅ |
| TC-414 | Parameter, literal and function nodes key to the golden vectors, and a function key carries its owner | Unit | P1 | FR-092-AC-4, FR-092-AC-5, FR-092-AC-6, FR-092-AC-10 | ✅ |
| TC-415 | Each checked Value expression lowers to its FR-322 node with its catalogued operation | Unit | P1 | FR-093-AC-1, FR-093-AC-2, FR-093-AC-3, FR-093-AC-4, FR-093-AC-5, FR-093-AC-6, FR-093-AC-8, FR-093-AC-10, FR-093-AC-11, FR-093-AC-14, FR-093-AC-15 | 🚧 |
| TC-416 | The v2 emission arm writes the nodes check lowered, and each emitted node recomputes to its node id | Integration | P1 | FR-093-AC-7, FR-093-AC-9, FR-093-AC-12, FR-093-AC-13, FR-093-AC-16, FR-093-AC-17, FR-093-AC-18, FR-093-AC-19, FR-093-AC-20 | 🚧 |
| TC-417 | Model declaration, Reference and Population nodes key to the golden vectors under ModelOwner | Unit | P1 | FR-094-AC-1, FR-094-AC-2, FR-094-AC-3, FR-094-AC-4, FR-094-AC-7 | ✅ |
| TC-418 | Clause function nodes key to the golden vectors with the operation member's ModelOwner | Unit | P1 | FR-094-AC-5 | ✅ |
| TC-419 | A declared unit's quantity type is its unit node, and a compound unit's keys to the golden vectors | Unit | P1 | FR-094-AC-6, FR-094-AC-7 | ✅ |
| TC-420 | Occurrence keys and source regions are lexical values, and every checked node has an occurrence | Unit | P1 | FR-095-AC-1, FR-095-AC-2 | ✅ |
| TC-421 | The package source map carries the wire's source map, and a location resolves or refuses by cause | Integration | P1 | FR-095-AC-3, FR-095-AC-4 | ✅ |
| TC-422 | Each Locus variant resolves to regions by its own rule, and the artifact pointer is RFC 6901 | Unit | P1 | FR-095-AC-5, FR-095-AC-6 | ✅ |
| TC-423 | The default checking ceilings bind wide and long leaf lists, admit large enum packages, and are recorded with the result | Unit | P1 | NFR-011-M-1 (node refusal: step 1), NFR-011-M-2 (step 3), NFR-011-M-3 (byte refusal before work: step 4), NFR-011-M-4 (work refusal: step 5; enum package admitted: step 4) | ✅ |
| TC-424 | An admitted source carries the source reference its caller named, and its node keys ignore the content digest | Unit | P1 | FR-001-AC-5, FR-001-AC-6, FR-001-AC-7, FR-001-AC-8, FR-001-AC-9, FR-001-AC-10, FR-001-AC-11, FR-001-AC-12 | 🚧 |
| TC-425 | parse and format take the two source labels and report the source reference | Integration | P1 | FR-010-AC-11 | 🚧 |
| TC-426 | A check location resolves to the region of the unit it was read from, or to none | Unit | P1 | FR-096-AC-1 | ✅ |
| TC-427 | A stage limit names its kind, bound, actual counter and locus | Unit | P1 | FR-096-AC-2, FR-096-AC-3, FR-096-AC-4, FR-096-AC-5, FR-096-AC-16, FR-096-AC-17 | ✅ |
| TC-428 | A refusal record carries its code, category, locus and the catalog's fields | Unit | P1 | FR-096-AC-6, FR-096-AC-7, FR-096-AC-8, FR-096-AC-13, FR-096-AC-15 | ✅ |
| TC-429 | The I2 reader locates its version refusal and its limits in the artifact | Integration | P1 | FR-096-AC-9, FR-096-AC-10, FR-096-AC-18 | ✅ |
| TC-430 | Native run and compile requests and their outputs carry the two source labels | Integration | P1 | FR-026-AC-6, FR-027-AC-4, FR-031-AC-5 | 🚧 |
| TC-431 | Runtime input artifacts carry the two labels, and bytes missing one refuse | Unit | P1 | FR-018-AC-8, FR-024-AC-6 | 🚧 |
| TC-432 | A family check's stage limit names its kind, bound and actual counter, and the counter is where the limit stops | Unit | P1 | FR-062-AC-12 | ✅ |
| TC-434 | Model limits have finite defaults, and normalization work stops at the limit | Unit | P1 | NFR-012 (defaults and recording: steps 1 and 2; bounded work: steps 3 and 4; meter memory: step 5; shared paths: step 6; ancestor-steps order: step 7) | ✅ |
| TC-436 | Proof-bound and interval-key constructors refuse empty ranges, and domain keys order by node then path | Unit | P1 | FR-097-AC-1 | ✅ |
| TC-437 | The extent rule names each unbounded type position once, by node and path, under a node-count ceiling | Unit | P1 | FR-097-AC-2 | ✅ |
| TC-438 | The request writer computes the available finite bound, writes a bounded request as its own item, and refuses bad bounds before writing | Unit | P1 | FR-097-AC-3, FR-097-AC-4 | ✅ |
| TC-439 | An exploration outcome maps to its O-16 category and keeps its frontier | Unit | P1 | FR-097-AC-5 | ✅ |
| TC-440 | QSL's extent agrees with IR's requires-bound at the pinned IR revision | Integration | P1 | FR-097-AC-6 | ✅ |
| TC-441 | An unbounded collection never refuses for cardinality and stops only on the caller's meter | Unit | P1 | FR-097-AC-7, FR-097-AC-8 | ✅ |
| TC-453 | Exploration orders successors canonically and keys states by their JCS bytes | Unit | P1 | FR-101-AC-1, FR-101-AC-2, FR-101-AC-9 | ✅ |
| TC-454 | The sampler reproduces its vectors, and sampled traces replay | Unit | P1 | FR-101-AC-3, FR-101-AC-4, FR-101-AC-5, FR-101-AC-10 | ✅ |
| TC-455 | Stopped explorations stay incomplete, and unbounded requests require a bound | Unit | P1 | FR-101-AC-6, FR-101-AC-7, FR-101-AC-8 | ✅ |
| TC-442 | Spine compile admits a domain package and locks its model selection | Integration | P1 | FR-027-AC-9, FR-056-AC-9 | ✅ |
| TC-443 | A model field's multiplicity and presence give its assembled value type | Unit | P1 | FR-056-AC-10 | ✅ |
| TC-444 | The replay executor recompiles, selects, calls, and refuses each O-26 case | Unit | P1 | FR-098-AC-1, FR-098-AC-2, FR-098-AC-3, FR-098-AC-4, FR-098-AC-5, FR-098-AC-6, FR-098-AC-7 | ✅ |
| TC-446 | Spine compile resolves imports against supplied libraries | Integration | P1 | FR-099-AC-1, FR-099-AC-2, FR-099-AC-3, FR-099-AC-4, FR-099-AC-5, FR-099-AC-6, FR-099-AC-7, FR-027-AC-10 | ✅ |
| TC-490 | E3 resolves header profile selections against the DefinitionLock catalog | Integration | P1 | FR-110-AC-1, FR-110-AC-2, FR-110-AC-3, FR-110-AC-5, FR-110-AC-6, FR-110-AC-7, FR-110-AC-8, FR-110-AC-9 | 🚧 |
| TC-491 | library::bundle links a complete-V1 bundle and refuses each closure, facet and limit defect | Integration | P1 | FR-111-AC-1, FR-111-AC-2, FR-111-AC-3, FR-111-AC-4, FR-111-AC-5, FR-111-AC-6, FR-111-AC-7 | ✅ |
| TC-450 | CLI run routes a program by its declared edition and calls a 1-draft function | Integration | P1 | FR-100-AC-1, FR-100-AC-2, FR-100-AC-3 | 🚧 |
| TC-451 | Spine run binds arguments by name and maps each outcome and refusal to its exit code | Integration | P1 | FR-100-AC-4, FR-100-AC-5, FR-100-AC-6 | ✅ |
| TC-452 | The spine run entry is qsl_replay::spine::run, agrees with the CLI, and maps every outcome | Unit | P1 | FR-100-AC-7, FR-100-AC-8, FR-100-AC-9 | ✅ |
| TC-456 | S2 builds state clause forms and the self, result and reaches expressions | Unit | P1 | FR-102-AC-1, FR-102-AC-2, FR-102-AC-3 | 🚧 |
| TC-457 | S2 state clause dispatch is thin, bounded and seam-probed | Unit | P1 | FR-102-AC-4, FR-102-AC-5, FR-102-AC-6 | 🚧 |
| TC-458 | Spine intake and assembly admit operations and frames | Integration | P1 | FR-103-AC-1, FR-103-AC-2, FR-103-AC-3, FR-103-AC-4, FR-103-AC-5 | 🚧 |
| TC-459 | S3 checks the ConfigVersion state clauses and types self, result and pre | Unit | P1 | FR-104-AC-1, FR-104-AC-2, FR-104-AC-7, FR-104-AC-8 | ✅ |
| TC-460 | S3 refuses ill-formed state clauses with their catalog codes | Unit | P1 | FR-104-AC-3, FR-104-AC-4 | ✅ |
| TC-461 | S3 records one operation-contract requirement per state clause and frame | Unit | P1 | FR-104-AC-5, FR-104-AC-6 | ✅ |
| TC-462 | S4 emits state clause, operation anchor and frame nodes with their bodies | Integration | P1 | FR-105-AC-1, FR-105-AC-2, FR-105-AC-5 | ✅ |
| TC-463 | The state package reads back through I2, keeps its identity rules and emits all or nothing | Integration | P1 | FR-105-AC-3, FR-105-AC-4, FR-105-AC-6 | ✅ |
| TC-464 | Snapshot and invocation documents read and admit into an observation set | Unit | P1 | FR-106-AC-1, FR-106-AC-2, FR-106-AC-6, FR-106-AC-8, FR-106-AC-9 | 🚧 |
| TC-465 | Admission refuses or reports incomplete for each input defect, in check order | Unit | P1 | FR-106-AC-3, FR-106-AC-4, FR-106-AC-5, FR-106-AC-7, FR-106-AC-11, FR-106-AC-12, FR-106-AC-13 | 🚧 |
| TC-466 | S6a evaluates state clauses over their observations, pre reads and reaches | Integration | P1 | FR-107-AC-1, FR-107-AC-2, FR-107-AC-3 | ✅ |
| TC-467 | S6a clause entry refuses bad selections, reports exhaustion and is deterministic | Integration | P1 | FR-107-AC-4, FR-107-AC-5, FR-107-AC-6 | 🚧 |
| TC-468 | The spine clause run entry reports typed dispositions and exit codes | Integration | P1 | FR-109-AC-1, FR-109-AC-2, FR-109-AC-3, FR-109-AC-4, FR-109-AC-5, FR-109-AC-6, FR-109-AC-7 | 🚧 |
| TC-469 | The ConfigVersion spine corpus gives native-equal typed dispositions | Integration | P1 | FR-108-AC-1, FR-108-AC-2, FR-108-AC-3, FR-108-AC-4, FR-108-AC-5, FR-108-AC-6 | ✅ |
| TC-470 | runtime_invariant exits 30 and outranks other diagnostics | Unit | P1 | FR-096-AC-12 | ✅ |
| TC-471 | Model successors follow operations, arguments, frames and contracts | Integration | P1 | FR-120-AC-1, FR-120-AC-2, FR-120-AC-3, FR-120-AC-4 | 🚧 |
| TC-472 | Invariant-violating successors are recorded, and undecided expansions stop the run | Integration | P1 | FR-120-AC-5, FR-120-AC-6, FR-120-AC-7, FR-120-AC-8, FR-120-AC-9, FR-120-AC-13 | 🚧 |
| TC-473 | Model effects and results are trace data, and ambient-state reads refuse at S3 | Integration | P1 | FR-120-AC-10, FR-120-AC-11, FR-120-AC-12 | 🚧 |
| TC-474 | The engine records findings, stops on an expansion stop, and replays a stopped trace | Integration | P1 | FR-101-AC-12, FR-101-AC-13, FR-101-AC-14, FR-097-AC-5 | ✅ |
| TC-480 | S2 builds enum and predicate forms | Unit | P1 | FR-091-AC-25, FR-091-AC-26 | ✅ |
| TC-481 | The assembler admits source enums and predicates, which check and lowering then use | Integration | P1 | FR-091-AC-27, FR-091-AC-28, FR-091-AC-29, FR-091-AC-30, FR-092-AC-13 | ✅ |
| TC-482 | S2 builds dimension and unit forms | Unit | P1 | FR-091-AC-31 | ✅ |
| TC-483 | The assembler admits source dimensions and units into a UnitGraph and refuses each source error | Unit | P1 | FR-091-AC-32, FR-091-AC-33, FR-091-AC-34, FR-091-AC-35 | 🚧 |
| TC-500 | A sum seed or running total outside its domain is a located undefined outcome | Unit | P1 | FR-096-AC-14 | ✅ |
| TC-510 | S2 builds protocol scoped anchor forms with their scope and segments | Unit | P1 | FR-112-AC-1, FR-112-AC-2, FR-112-AC-3 | 🚧 |
| TC-511 | S3 resolves scoped anchors through nested scopes and refuses a missing anchor or member | Unit | P1 | FR-113-AC-1, FR-113-AC-2, FR-113-AC-3 | 🚧 |
| TC-512 | S3 refuses ambiguous, shadowing, wrong-kind and wrong-channel names, in builder order | Unit | P1 | FR-113-AC-4, FR-113-AC-5, FR-113-AC-6, FR-113-AC-7 | 🚧 |
| TC-513 | S3 binds a protocol attempt to its operation's one anchor and frame | Integration | P1 | FR-114-AC-1, FR-114-AC-2, FR-114-AC-3, FR-114-AC-4 | 🚧 |
| TC-514 | The spine run entry checks an invocation against its operation frame | Integration | P1 | FR-115-AC-1, FR-115-AC-2, FR-115-AC-3, FR-115-AC-4, FR-115-AC-5, FR-115-AC-6 | ✅ |
| TC-515 | The replay facade replays a frame counterexample and keeps its identities | Integration | P1 | FR-116-AC-1, FR-116-AC-2, FR-116-AC-3, FR-116-AC-4, FR-116-AC-5, FR-116-AC-6 | ✅ |
| TC-516 | call_site names a function's parameters and an operation's and a state clause's identities, matching what replay accepts | Unit | P1 | FR-121-AC-1, FR-121-AC-2, FR-121-AC-3, FR-121-AC-4, FR-121-AC-5, FR-121-AC-6, FR-121-AC-7, FR-121-AC-8, FR-121-AC-9, FR-121-AC-10, FR-121-AC-11, FR-121-AC-12, FR-121-AC-13, FR-121-AC-14, FR-121-AC-15, FR-121-AC-16, FR-121-AC-17, FR-121-AC-18, FR-121-AC-19, FR-121-AC-20, FR-121-AC-21, FR-121-AC-22 | ✅ |
| TC-517 | The replay facade replays a state-clause counterexample and keeps its identities | Integration | P1 | FR-122-AC-1, FR-122-AC-2, FR-122-AC-3, FR-122-AC-4, FR-122-AC-5, FR-122-AC-6, FR-122-AC-7 | 🚧 |
| TC-740 | S6a stop reports and the decision path derive a state clause's basis and witness | Unit | P1 | FR-265-AC-1, FR-265-AC-2, FR-265-AC-3, FR-265-AC-4, FR-265-AC-5, FR-265-AC-6 | 🚧 |
| TC-741 | Clause run reports carry a settlement basis on every disposition and a witness only when decisive | Integration | P1 | FR-266-AC-1, FR-266-AC-2, FR-266-AC-3 | 🚧 |
| TC-745 | The checked-input gate rejects pre-check signatures and reconstruction and passes the workspace | Unit | P1 | FR-270-AC-1, FR-270-AC-2, FR-270-AC-3, FR-270-AC-4 | ✅ |
| TC-746 | Canonical doc tags define the canonical set, and misplaced or duplicate tags fail | Unit | P1 | FR-271-AC-1, FR-271-AC-2 | ✅ |
| TC-747 | The canonical-types gate finds namesakes, re-exports and same-shaped copies | Unit | P1 | FR-272-AC-1, FR-272-AC-2, FR-272-AC-3, FR-272-AC-4 | ✅ |
| TC-748 | Once every namesake is deleted or renamed, the canonical-types gate and make ci pass | Integration | P1 | FR-273-AC-1 | 🚧 |
| TC-790 | Each model form types through StateModel and refuses with a StateModel cause | Integration | P1 | FR-300-AC-1, FR-300-AC-2 | 🚧 |
| TC-791 | The seam probe reports the StateModel arms at S1, S2 and S3 | Integration | P1 | FR-300-AC-3 | 🚧 |
| TC-792 | StateModel causes raised under Value and ProtocolClause evaluation render with the state-model prefix | Integration | P1 | FR-301-AC-1, FR-301-AC-2 | 🚧 |
| TC-793 | Unit compile carries one linked dispatch table per called operation, named by each call | Integration | P1 | FR-302-AC-1 | 🚧 |
| TC-794 | Unit compile refuses ambiguous and inapplicable dispatch with no checked package | Integration | P1 | FR-302-AC-2, FR-302-AC-3, FR-302-AC-4 | 🚧 |
| TC-795 | Unit compile refuses a dispatch family past the caller's family_steps limit | Integration | P1 | FR-302-AC-5 | 🚧 |
| TC-796 | The model correspondence faults on a second, different entry in either direction | Integration | P1 | FR-303-AC-1, FR-303-AC-2, FR-303-AC-3, FR-303-AC-4 | ✅ |
| TC-797 | An abstraction relation checks into one binding per key, total or partial | Unit | P1 | FR-304-AC-1, FR-304-AC-2 | 🚧 |
| TC-798 | An abstraction binding whose model key resolves to nothing refuses missing-name | Unit | P1 | FR-304-AC-3 | 🚧 |
| TC-799 | Duplicate and conflicting abstraction bindings refuse conflicting-binding | Unit | P1 | FR-304-AC-4 | 🚧 |
| TC-800 | Abstraction bindings with malformed parameter or field maps refuse malformed-declaration | Unit | P1 | FR-304-AC-5 | 🚧 |
| TC-801 | RustPath, RustField and RustReceiver segments are checked against Rust syntax | Unit | P1 | FR-304-AC-6 | 🚧 |
| TC-802 | A frame binding checks without a naming clause and relates to the frame and anchor when one exists | Integration | P1 | FR-305-AC-1, FR-305-AC-2 | 🚧 |
| TC-803 | An inherited operation's frame binds only at its declaring type | Unit | P1 | FR-305-AC-3 | 🚧 |
| TC-804 | The abstraction relation is emitted as a v2 node and enters the package_id | Integration | P1 | FR-306-AC-1, FR-306-AC-2 | 🚧 |
| TC-805 | The abstraction relation changes no requirement record, route result or clause run | Integration | P1 | FR-306-AC-3, FR-306-AC-4 | 🚧 |
| TC-806 | The export refuses each item with an unbound element and returns the others | Integration | P1 | FR-307-AC-1, FR-307-AC-2, FR-307-AC-3 | 🚧 |
| TC-807 | The export references the receiver's static type and the operation key | Integration | P1 | FR-307-AC-4, FR-307-AC-5 | 🚧 |
| TC-809 | Evaluation selects the dispatched body from the linked table by the receiver's most-specific type | Integration | P1 | FR-302-AC-6 | 🚧 |
| TC-810 | Abstraction binding keys are unique across all of a unit's declarations | Integration | P1 | FR-304-AC-7 | 🚧 |
| TC-811 | Abstraction declarations parse from source and refuse unsupported or malformed forms | Integration | P1 | FR-304-AC-8 | 🚧 |
| TC-518 | S3 checks fairness constraints and interval operators of infinite-trace clauses | Unit | P1 | FR-123-AC-1, FR-123-AC-2, FR-123-AC-3, FR-123-AC-4 | 🚧 |
| TC-519 | Terminal declarations check, and the request carries one deadlock-freedom item per subject | Unit | P1 | FR-124-AC-1, FR-124-AC-2, FR-124-AC-3 | 🚧 |
| TC-520 | A model subject's behaviours read as temporal traces, with terminal stutter and interval wrap | Integration | P1 | FR-125-AC-1, FR-125-AC-2, FR-125-AC-3, FR-125-AC-4, FR-125-AC-5 | 🚧 |
| TC-521 | The explicit-state model checker proves, refutes and stops over model subjects | Integration | P1 | FR-126-AC-1, FR-126-AC-2, FR-126-AC-3, FR-126-AC-4, FR-126-AC-5, FR-126-AC-6, FR-126-AC-7 | 🚧 |
| TC-522 | Model-check outcomes settle as QSpec FR-331 terminal records with their strength | Unit | P1 | FR-127-AC-1, FR-127-AC-2, FR-127-AC-3, FR-127-AC-4, FR-127-AC-5, FR-127-AC-7, FR-127-AC-8, FR-127-AC-9, FR-127-AC-10 | 🚧 |
| TC-523 | The replay facade replays a model counterexample through ModelSystem | Integration | P1 | FR-128-AC-1, FR-128-AC-2, FR-128-AC-3, FR-128-AC-4 | 🚧 |
| TC-524 | A checked temporal clause emits as a v2 temporal clause node and reads back | Integration | P1 | FR-337-AC-1, FR-337-AC-2, FR-337-AC-3, FR-337-AC-4 | 🚧 |
| TC-525 | An EN-1 closure certificate is accepted or rejected by the core checker | Unit | P1 | FR-338-AC-1, FR-338-AC-2, FR-338-AC-3 | 🚧 |
| TC-526 | An EN-1 component certificate is accepted or rejected by the core checker | Unit | P1 | FR-339-AC-1, FR-339-AC-2 | 🚧 |
| TC-893 | An SMT proof certificate is accepted or rejected by the core checker | Unit | P1 | FR-314-AC-1, FR-314-AC-2 | 🚧 |
| TC-900 | The SMT-LIB transition-relation encoding is canonical and refuses unencodable constructs | Unit | P1 | FR-315-AC-1, FR-315-AC-2, FR-315-AC-3, FR-315-AC-4, FR-315-AC-5 | 🚧 |
| TC-901 | The quire fences of spec artifacts are checked against the objects they declare | Unit | P1 | FR-355-AC-1, FR-355-AC-2, FR-355-AC-3, FR-355-AC-4, FR-355-AC-5, FR-355-AC-6 | 🚧 |
| TC-530 | S3 checks strong fairness constraints and the unmarked fairness kind | Unit | P1 | FR-129-AC-1, FR-129-AC-2 | 🚧 |
| TC-531 | The explicit-state model checker decides strong fairness by SCC refinement | Integration | P1 | FR-130-AC-1, FR-130-AC-2, FR-130-AC-3, FR-130-AC-4, FR-130-AC-5 | 🚧 |
| TC-532 | Replay checks strong fairness on a model counterexample | Integration | P1 | FR-131-AC-1, FR-131-AC-2, FR-131-AC-3 | 🚧 |
| TC-533 | A fairness constraint over a supplied trace settles as a missing premise | Unit | P1 | FR-132-AC-1, FR-132-AC-2 | 🚧 |
| TC-534 | A refuted liveness record names the strong constraint that would exclude its lasso | Integration | P1 | FR-133-AC-1, FR-133-AC-2, FR-133-AC-3 | 🚧 |
| TC-535 | Candidates carry and are filtered by the fairness kinds their backends advertise | Unit | P1 | FR-134-AC-1, FR-134-AC-2 | 🚧 |
| TC-536 | Exploration and model-check limits publish their defaults | Unit | P1 | FR-101-AC-15, FR-126-AC-8 | 🚧 |
| TC-537 | An undefined claim evaluation refutes in the explicit-state model checker | Integration | P1 | FR-125-AC-6, FR-126-AC-9 | 🚧 |
| TC-538 | An undefined-evaluation counterexample settles refuted with its cause | Unit | P1 | FR-127-AC-6 | 🚧 |
| TC-539 | Replay reproduces an undefined claim evaluation at its position | Integration | P1 | FR-128-AC-5, FR-128-AC-6 | 🚧 |
| TC-835 | S2 parses temporal operators with an optional interval, independent of profile | Unit | P1 | FR-325-AC-1, FR-325-AC-2, FR-325-AC-3, FR-325-AC-4 | 🚧 |
| TC-836 | S3 admits temporal operators by the unit's temporal profile and records the requirement | Unit | P1 | FR-326-AC-1, FR-326-AC-2, FR-326-AC-3, FR-326-AC-4, FR-326-AC-5 | 🚧 |
| TC-837 | S6a evaluates a temporal clause over a finite trace with typed positions and metered work | Unit | P1 | FR-327-AC-1, FR-327-AC-2, FR-327-AC-3, FR-327-AC-4 | 🚧 |
| TC-838 | S6a evaluates an infinite-trace clause three-valued over a finite prefix | Unit | P1 | FR-328-AC-1, FR-328-AC-2, FR-328-AC-3, FR-328-AC-4 | 🚧 |
| TC-839 | S6a evaluates an infinite-trace clause exactly over a fair lasso | Unit | P1 | FR-329-AC-1, FR-329-AC-2, FR-329-AC-3, FR-329-AC-4, FR-329-AC-5 | 🚧 |
| TC-840 | run_clause runs a selected temporal clause over a supplied trace | Integration | P1 | FR-330-AC-1, FR-330-AC-2, FR-330-AC-3, FR-330-AC-4 | 🚧 |
| TC-841 | The replay facade replays a temporal counterexample over an observed trace | Integration | P1 | FR-331-AC-1, FR-331-AC-2, FR-331-AC-3, FR-331-AC-5 | 🚧 |
| TC-842 | An infinite-trace item settles only through negotiation | Integration | P1 | FR-332-AC-1, FR-332-AC-2, FR-332-AC-3, FR-332-AC-4 | 🚧 |
| TC-843 | Collection and population types resolve with an optional bound | Unit | P1 | FR-333-AC-1, FR-333-AC-2, FR-333-AC-3 | 🚧 |
| TC-844 | Collection and population nodes are keyed by the root definitions' identity preimage | Integration | P1 | FR-334-AC-1, FR-334-AC-2, FR-334-AC-3, FR-334-AC-4 | 🚧 |
| TC-845 | A claim over an unbounded declaration settles end to end | Integration | P1 | FR-335-AC-1, FR-335-AC-2, FR-335-AC-3, FR-335-AC-4, FR-335-AC-5 | 🚧 |
| TC-846 | A model subject is finite by its universes, apart from proof bounds | Integration | P1 | FR-336-AC-1, FR-336-AC-2, FR-336-AC-3, FR-336-AC-4 | 🚧 |
| TC-847 | An undefined letter fails a clause over a supplied trace or lasso, and replays | Integration | P1 | FR-327-AC-5, FR-328-AC-5, FR-329-AC-6, FR-330-AC-5, FR-331-AC-4 | 🚧 |
| TC-848 | A fairness premise over a supplied trace is missing, and the clause settles unsupported | Integration | P1 | FR-328-AC-6, FR-329-AC-7, FR-330-AC-6 | 🚧 |
| TC-860 | The spec-versioning gate reads each pair, refuses a malformed one, and runs every case against both revisions | Integration | P1 | FR-340-AC-1, FR-340-AC-2, FR-340-AC-3, FR-340-AC-4, FR-340-AC-5 | 🚧 |
| TC-861 | Spine compile results classify into exactly one refinement class, reading every cause | Unit | P1 | FR-341-AC-1, FR-341-AC-2, FR-341-AC-3, FR-341-AC-4, FR-341-AC-5, FR-341-AC-6 | 🚧 |
| TC-862 | Clause-run dispositions classify into one refinement class with their stage | Unit | P1 | FR-342-AC-1, FR-342-AC-2, FR-342-AC-3, FR-342-AC-4, FR-342-AC-5 | 🚧 |
| TC-863 | The versioning comparison returns the table's result for every class pair | Unit | P1 | FR-343-AC-1 | 🚧 |
| TC-864 | A seeded spec-versioning regression fails the gate naming exactly that case | Integration | P1 | FR-340-AC-6, FR-343-AC-2, FR-343-AC-3, FR-343-AC-4, FR-344-AC-5 | 🚧 |
| TC-865 | An unsupported or incomplete superseding run is unresolved, never holds, and names its limit | Integration | P1 | FR-343-AC-5, FR-344-AC-4 | 🚧 |
| TC-866 | The refinement report orders every result, lists every regression, and gives FR-301's verdict and exit | Unit | P1 | FR-344-AC-1, FR-344-AC-2, FR-344-AC-3, FR-344-AC-6, FR-344-AC-7 | 🚧 |
| TC-867 | The layering comparison returns the table's result for every parent and child class | Unit | P1 | FR-345-AC-1, FR-345-AC-2, FR-345-AC-3 | 🚧 |
| TC-868 | A seeded profile-layering regression fails the gate naming the case and its edge | Integration | P1 | FR-345-AC-4, FR-345-AC-5, FR-345-AC-6, FR-345-AC-7, FR-345-AC-8, FR-345-AC-9 | 🚧 |
| TC-875 | The conformance runner executes vectors through public entry points and compares typed results | Integration | P1 | FR-350-AC-1, FR-350-AC-2 | 🚧 |
| TC-876 | The conformance runner reports unsupported, incomplete and tool-failure vectors on their own | Integration | P1 | FR-350-AC-3, FR-350-AC-4, FR-350-AC-5 | 🚧 |
| TC-877 | Each capability settles from its vectors in the stated precedence, and an empty capability is uncovered | Integration | P1 | FR-351-AC-1, FR-351-AC-2 | 🚧 |
| TC-878 | Capability outcomes come only from vectors executed in the run, over the scope the caller selects | Integration | P1 | FR-351-AC-3, FR-351-AC-4 | 🚧 |
| TC-879 | The conformance report gives outcome counts, coverage and each failing vector, in a stable order | Integration | P1 | FR-352-AC-1, FR-352-AC-2, FR-352-AC-3 | 🚧 |
| TC-880 | The verdict is complete-V1 qualified only when every in-scope capability passes | Integration | P1 | FR-353-AC-1, FR-353-AC-2 | 🚧 |
| TC-881 | A scoped run's verdict names its scope, and a refused run has no verdict | Integration | P1 | FR-353-AC-3, FR-353-AC-4 | 🚧 |
| TC-882 | An extension's declaration form parses, checks against its typed-node schema and packages as a typed node | Unit | P1 | FR-354-AC-1, FR-354-AC-2 | 🚧 |
| TC-883 | Malformed, colliding, cyclic and unselected extensions refuse with the definitions involved | Unit | P1 | FR-354-AC-3, FR-354-AC-4 | 🚧 |
| TC-884 | An extension changes no other unit's identity, backends change no admission, and a reader without it refuses | Unit | P1 | FR-354-AC-5 | 🚧 |
| TC-885 | Formatting keeps the checked package identity of every complete-V1 fixture unit | Integration | P1 | FR-003-AC-9 | 🚧 |
| TC-650 | The protocol subject builds, refuses bad bindings and keys states canonically | Unit | P1 | FR-205-AC-1, FR-205-AC-2, FR-205-AC-3, FR-205-AC-4 | 🚧 |
| TC-651 | The protocol system takes each step kind with structural moves folded | Integration | P1 | FR-206-AC-1, FR-206-AC-2, FR-206-AC-3, FR-206-AC-4, FR-206-AC-5, FR-206-AC-6 | 🚧 |
| TC-652 | Compensations register, activate, attempt, close and end with their recovery status | Integration | P1 | FR-207-AC-1, FR-207-AC-2, FR-207-AC-3, FR-207-AC-4 | 🚧 |
| TC-653 | Replicated role instances spawn under max, act, retire by lifetime and key by object | Integration | P1 | FR-208-AC-1, FR-208-AC-2, FR-208-AC-3 | 🚧 |
| TC-654 | Activation on each starts instances under max_live_instances and states the budget used | Integration | P1 | FR-209-AC-1, FR-209-AC-2, FR-209-AC-3, FR-209-AC-4 | 🚧 |
| TC-655 | The protocol system admits exactly the causal interleavings | Integration | P1 | FR-210-AC-1, FR-210-AC-2, FR-210-AC-3, FR-210-AC-4 | 🚧 |
| TC-656 | Protocol terminal declarations check and protocol deadlocks are reported with blocked threads | Integration | P1 | FR-211-AC-1, FR-211-AC-2, FR-211-AC-3, FR-211-AC-4 | 🚧 |
| TC-657 | Scheduler fairness is derived per thread and turned off by scheduling adversarial | Integration | P1 | FR-212-AC-1, FR-212-AC-2, FR-212-AC-3, FR-212-AC-4 | 🚧 |
| TC-658 | Interval operators over a protocol measure distance in counted steps | Integration | P1 | FR-213-AC-1, FR-213-AC-2, FR-213-AC-3 | 🚧 |
| TC-659 | Every protocol step class needs an explicit refinement row | Unit | P1 | FR-214-AC-1, FR-214-AC-2, FR-214-AC-3, FR-214-AC-4 | 🚧 |
| TC-660 | ProtocolSystem implements TransitionSystem with canonical identities and the shared application rule | Integration | P1 | FR-215-AC-1, FR-215-AC-2, FR-215-AC-3, FR-215-AC-4 | 🚧 |
| TC-661 | Protocol steps carry static footprints, enabling footprints and visibility | Unit | P1 | FR-216-AC-1, FR-216-AC-2, FR-216-AC-3, FR-216-AC-4, FR-216-AC-5 | 🚧 |
| TC-662 | The replay facade replays a protocol counterexample through ProtocolSystem | Integration | P1 | FR-217-AC-1, FR-217-AC-2, FR-217-AC-3, FR-217-AC-4 | 🚧 |
| TC-663 | S3 checks every protocol control construct and records scopes | Unit | P1 | FR-218-AC-1, FR-218-AC-2, FR-218-AC-3, FR-218-AC-4, FR-218-AC-5, FR-218-AC-6, FR-218-AC-7 | 🚧 |
| TC-887 | S2 builds the complete protocol forms with their members and spans | Unit | P1 | FR-308-AC-1, FR-308-AC-2, FR-308-AC-3, FR-308-AC-4, FR-308-AC-5 | 🚧 |
| TC-755 | Every lifecycle operation takes a typed request, limits and Cancel, and rejects wrong-stage input at compile time | Unit | P1 | FR-275-AC-1, FR-275-AC-2, FR-275-AC-3, FR-275-AC-5 | 🚧 |
| TC-664 | The memory clause checks and the request selects the resolved model | Unit | P1 | FR-219-AC-1, FR-219-AC-2, FR-219-AC-3, FR-219-AC-4, FR-219-AC-5 | 🚧 |
| TC-665 | S3 checks orderings and fences and classifies every access | Unit | P1 | FR-220-AC-1, FR-220-AC-2, FR-220-AC-3, FR-220-AC-4, FR-220-AC-5 | 🚧 |
| TC-666 | The tso memory model explores store buffers, flushes and locked accesses | Integration | P1 | FR-221-AC-1, FR-221-AC-2, FR-221-AC-3, FR-221-AC-4 | 🚧 |
| TC-667 | The ra memory model explores messages, views, fences and garbage collection | Integration | P1 | FR-222-AC-1, FR-222-AC-2, FR-222-AC-3, FR-222-AC-4 | 🚧 |
| TC-668 | The SC event graph orders seq_cst events and prunes cyclic choices | Integration | P1 | FR-223-AC-1, FR-223-AC-2, FR-223-AC-3, FR-223-AC-4 | 🚧 |
| TC-669 | Non-atomic locations explore and the race-freedom item reports races once | Integration | P1 | FR-224-AC-1, FR-224-AC-2, FR-224-AC-3, FR-224-AC-4 | 🚧 |
| TC-670 | Memory bounds limit stores, default to 4 and are stated in every result | Integration | P1 | FR-225-AC-1, FR-225-AC-2, FR-225-AC-3, FR-225-AC-4 | 🚧 |
| TC-671 | S3 warns of load-buffering shapes and negotiation routes by resolved model | Unit | P1 | FR-226-AC-1, FR-226-AC-2, FR-226-AC-3 | 🚧 |
| TC-672 | Weak-memory counterexamples carry memory components and replay | Integration | P1 | FR-227-AC-1, FR-227-AC-2, FR-227-AC-3, FR-227-AC-4 | 🚧 |
| TC-673 | Flush and visibility fairness join every infinite-trace fairness set | Integration | P1 | FR-228-AC-1, FR-228-AC-2, FR-228-AC-3, FR-228-AC-4 | 🚧 |
| TC-674 | The memory component, observations, atoms and footprints flow through the seam | Integration | P1 | FR-229-AC-1, FR-229-AC-2, FR-229-AC-3, FR-229-AC-4, FR-229-AC-5 | 🚧 |
| TC-888 | S2 builds the memory clause, access ordering and fence forms | Unit | P1 | FR-309-AC-1, FR-309-AC-2, FR-309-AC-3 | 🚧 |
| TC-590 | S3 checks state-graph claims, refuses non-state predicates and fairness, and the request carries deadlock-freedom items | Unit | P1 | FR-165-AC-1, FR-165-AC-2, FR-165-AC-3 | 🚧 |
| TC-591 | Phase 0 samples seeded witnesses for possible claims, on by default | Integration | P1 | FR-166-AC-1, FR-166-AC-2, FR-166-AC-3, FR-166-AC-4 | 🚧 |
| TC-592 | EN-1 explores a state-graph subject once, labels it and tracks open nodes | Integration | P1 | FR-167-AC-1, FR-167-AC-2, FR-167-AC-3, FR-167-AC-4, FR-167-AC-5 | 🚧 |
| TC-593 | Backward reachability and path counting decide state-graph claims with canonical evidence | Integration | P1 | FR-168-AC-1, FR-168-AC-2, FR-168-AC-3, FR-168-AC-4, FR-168-AC-6 | 🚧 |
| TC-594 | State-graph outcomes settle as terminal records that state their settlement method | Unit | P1 | FR-169-AC-1, FR-169-AC-2, FR-169-AC-3, FR-169-AC-4, FR-169-AC-5 | 🚧 |
| TC-595 | The replay facade replays witnesses, traps and path pairs through ModelSystem | Integration | P1 | FR-170-AC-1, FR-170-AC-2, FR-170-AC-3, FR-170-AC-4, FR-170-AC-6 | 🚧 |
| TC-612 | An undefined predicate refutes a state-graph claim and replays | Integration | P1 | FR-168-AC-5, FR-169-AC-6, FR-170-AC-5 | 🚧 |
| TC-613 | A witness proves possible only after exploration rules out an undefined evaluation | Integration | P1 | FR-166-AC-5, FR-168-AC-7, FR-169-AC-7 | 🚧 |
| TC-614 | A witness on a stopped run settles inconclusive with well-definedness unchecked | Integration | P1 | FR-166-AC-6, FR-168-AC-8, FR-169-AC-8 | 🚧 |
| TC-618 | Phase 0 walks stop at their step budget, and the horizon is not a limit | Integration | P1 | FR-166-AC-7 | 🚧 |
| TC-619 | An explored witness ends at a target node on a run with open nodes | Integration | P1 | FR-168-AC-9 | 🚧 |
| TC-643 | State-graph certificates of true claims are accepted and certify the proof | Integration | P1 | FR-169-AC-9 | 🚧 |
| TC-644 | State-graph certificates that do not hold are rejected and never prove | Integration | P1 | FR-169-AC-10 | 🚧 |
| TC-596 | S2 builds forms for hyper clauses over behaviours and relations over model executions | Unit | P1 | FR-171-AC-1, FR-171-AC-2, FR-171-AC-3 | 🚧 |
| TC-597 | S3 binds trace variables to models, types indexed atoms and step labels, splits the match and checks align skip | Unit | P1 | FR-172-AC-1, FR-172-AC-2, FR-172-AC-3, FR-172-AC-4 | 🚧 |
| TC-598 | Hyper and relation clauses classify into HP-1 to HP-6 and HP-4 settles unsupported | Unit | P1 | FR-173-AC-1, FR-173-AC-2, FR-173-AC-3, FR-173-AC-4 | 🚧 |
| TC-599 | Hyper products admit reductions by their rows and halve by compiler-checked copy-swap | Integration | P1 | FR-174-AC-1, FR-174-AC-2, FR-174-AC-3, FR-174-AC-4 | 🚧 |
| TC-600 | The evaluator reads a hyper body over a tuple of lassos in lockstep | Unit | P1 | FR-175-AC-1, FR-175-AC-2, FR-175-AC-3 | 🚧 |
| TC-601 | EN-1 checks a universal hyperproperty over the self-composition product | Integration | P1 | FR-176-AC-1, FR-176-AC-2, FR-176-AC-3, FR-176-AC-4 | 🚧 |
| TC-602 | EN-1 checks a forall-exists safety hyperproperty by witness sets | Integration | P1 | FR-177-AC-1, FR-177-AC-2, FR-177-AC-3, FR-177-AC-4, FR-177-AC-5, FR-177-AC-6, FR-183-AC-6 | 🚧 |
| TC-603 | EN-1 checks a projection-aligned hyperproperty over the projected product | Integration | P1 | FR-178-AC-1, FR-178-AC-2, FR-178-AC-3, FR-178-AC-4, FR-178-AC-5, FR-178-AC-6 | 🚧 |
| TC-604 | EN-1 checks a step relation over every tuple of reachable transitions | Integration | P1 | FR-179-AC-1, FR-179-AC-2, FR-179-AC-3 | 🚧 |
| TC-605 | A bound step relation hands over its code claim, and its Kani counterexample replays | Integration | P1 | FR-180-AC-1, FR-180-AC-2, FR-180-AC-3 | 🚧 |
| TC-606 | A single-existential claim is proved by a lasso witness and refuted by a trap | Integration | P1 | FR-181-AC-1, FR-181-AC-2, FR-181-AC-3, FR-181-AC-4 | 🚧 |
| TC-615 | A single-existential witness proves only after exploration rules out an undefined evaluation | Integration | P1 | FR-181-AC-5 | 🚧 |
| TC-616 | A single-existential witness on a stopped run settles well-definedness unchecked | Integration | P1 | FR-181-AC-6 | 🚧 |
| TC-645 | The product-closure certificate checker accepts the certificates of true hyper proofs | Integration | P1 | FR-163-AC-1 | 🚧 |
| TC-646 | The product-closure certificate checker rejects tampered and false certificates and stops at a limit | Integration | P1 | FR-163-AC-2, FR-163-AC-3, FR-163-AC-4 | 🚧 |
| TC-607 | Hyper and step-relation outcomes settle as terminal records with their causes | Unit | P1 | FR-182-AC-1, FR-182-AC-2, FR-182-AC-3, FR-182-AC-4, FR-182-AC-7, FR-182-AC-8 | 🚧 |
| TC-608 | The replay facade replays hyper counterexamples of every kind through ModelSystem | Integration | P1 | FR-183-AC-1, FR-183-AC-2, FR-183-AC-3, FR-183-AC-4 | 🚧 |
| TC-609 | max_witness_set and max_relation_tuples are caller budgets with published defaults | Unit | P1 | FR-184-AC-1, FR-184-AC-2, FR-184-AC-3 | 🚧 |
| TC-610 | An HP-1 relation whose first refuting tuple is undefined settles refuted with UndefinedEvaluation | Integration | P1 | FR-179-AC-4, FR-182-AC-5, FR-183-AC-5 | 🚧 |
| TC-611 | An HP-1 relation whose first refuting tuple is false settles refuted ahead of later undefined tuples | Integration | P1 | FR-179-AC-5, FR-182-AC-6 | 🚧 |
| TC-540 | S3 checks a refinement declaration's subjects, population map and object map | Unit | P1 | FR-135-AC-1, FR-135-AC-2, FR-135-AC-3, FR-135-AC-4 | 🚧 |
| TC-541 | S3 checks a refinement's step rows, over a model subject and a protocol subject | Unit | P1 | FR-136-AC-1, FR-136-AC-2, FR-136-AC-3, FR-136-AC-4 | 🚧 |
| TC-542 | S3 checks a refinement's assume and ensure rows into F_C and F_A | Unit | P1 | FR-137-AC-1, FR-137-AC-2, FR-137-AC-3, FR-137-AC-4 | 🚧 |
| TC-543 | History fields check at S3 and compute along a behaviour without changing the concrete model | Unit | P1 | FR-138-AC-1, FR-138-AC-2, FR-138-AC-3, FR-138-AC-4 | 🚧 |
| TC-544 | Population-valued expressions type and evaluate inside a refinement only | Unit | P1 | FR-139-AC-1, FR-139-AC-2, FR-139-AC-3, FR-139-AC-4, FR-139-AC-5 | 🚧 |
| TC-545 | The refinement mapping builds the abstract state from a concrete state and its history | Unit | P1 | FR-140-AC-1, FR-140-AC-2, FR-140-AC-3, FR-140-AC-4 | 🚧 |
| TC-546 | check_step decides initial states and steps by the stutter, abstract-operation and any rules | Unit | P1 | FR-141-AC-1, FR-141-AC-2, FR-141-AC-3, FR-141-AC-4, FR-141-AC-5 | 🚧 |
| TC-547 | The refinement product proves, refutes, leaves undetermined and stops the safety half | Integration | P1 | FR-142-AC-1, FR-142-AC-2, FR-142-AC-3, FR-142-AC-4, FR-142-AC-5, FR-142-AC-6, FR-142-AC-7 | 🚧 |
| TC-548 | The liveness half reads abstract fairness through the mapping under concrete fairness | Integration | P1 | FR-143-AC-1, FR-143-AC-2, FR-143-AC-3, FR-143-AC-4, FR-143-AC-5 | 🚧 |
| TC-549 | Refinement items request one temporal-satisfaction record and settle as one terminal record | Unit | P1 | FR-144-AC-1, FR-144-AC-2, FR-144-AC-3, FR-144-AC-4, FR-144-AC-5 | 🚧 |
| TC-550 | The replay facade replays a refinement counterexample and recomputes what the abstract model saw | Integration | P1 | FR-145-AC-1, FR-145-AC-2, FR-145-AC-3, FR-145-AC-4 | 🚧 |
| TC-551 | Per-step simulation records are written for functional refinements and never settle refuted | Unit | P1 | FR-146-AC-1, FR-146-AC-2, FR-146-AC-3, FR-146-AC-4 | 🚧 |
| TC-552 | S3 checks a refinement whose abstract side is a protocol and writes a refinement record | Unit | P1 | FR-147-AC-1, FR-147-AC-2, FR-147-AC-3, FR-147-AC-4 | 🚧 |
| TC-553 | Concrete steps are decided against an abstract protocol with internal closure and observation labels | Integration | P1 | FR-148-AC-1, FR-148-AC-2, FR-148-AC-3, FR-148-AC-4, FR-148-AC-5 | 🚧 |
| TC-554 | An undefined mapping row or history update refutes a refinement and replays | Integration | P1 | FR-138-AC-5, FR-141-AC-6, FR-142-AC-8, FR-145-AC-5 | 🚧 |
| TC-555 | A refused or incomplete mapping row or argument leaves the refinement undetermined, and an undefined argument refutes | Unit | P1 | FR-140-AC-5, FR-141-AC-7, FR-144-AC-6 | 🚧 |
| TC-556 | The simulation certificate checker accepts a true relation and rejects a false one | Integration | P1 | FR-149-AC-1, FR-149-AC-2, FR-149-AC-3, FR-149-AC-4, FR-144-AC-7 | 🚧 |
| TC-889 | S2 builds the refinement form with every row in source order | Unit | P1 | FR-310-AC-1, FR-310-AC-2, FR-310-AC-3 | 🚧 |
| TC-620 | S3 admits random parameters, workloads and rewards and refuses malformed ones | Unit | P1 | FR-185-AC-1, FR-185-AC-2, FR-185-AC-3, FR-185-AC-4 | 🚧 |
| TC-890 | S2 builds random, reward and workload declaration forms | Unit | P1 | FR-311-AC-1, FR-311-AC-2, FR-311-AC-3 | 🚧 |
| TC-621 | S3 checks probabilistic claim forms, thresholds, units and confidence parameters | Unit | P1 | FR-186-AC-1, FR-186-AC-2, FR-186-AC-3, FR-186-AC-4 | 🚧 |
| TC-622 | ModelSystem gives actions, step probabilities and rewards, and reports NotMarkov | Integration | P1 | FR-187-AC-1, FR-187-AC-2, FR-187-AC-3, FR-187-AC-4 | 🚧 |
| TC-623 | The QSpec sampler draws weighted choices exactly and reproducibly | Integration | P1 | FR-188-AC-1, FR-188-AC-2, FR-188-AC-3 | 🚧 |
| TC-624 | EN-4 decides probabilistic claims with Okamoto and SPRT, with Bonferroni, symmetry and activation | Integration | P1 | FR-189-AC-1, FR-189-AC-2, FR-189-AC-3, FR-189-AC-4, FR-189-AC-5 | 🚧 |
| TC-625 | EN-4 measures a long-run fraction by regeneration with asymptotic coverage | Integration | P1 | FR-190-AC-1, FR-190-AC-2, FR-190-AC-3 | 🚧 |
| TC-626 | Statistical runs stop on caller-set budgets and reproduce from their provenance | Integration | P1 | FR-191-AC-1, FR-191-AC-2, FR-191-AC-3, FR-191-AC-4 | 🚧 |
| TC-627 | Statistical results settle on the measured axis, fail the pipeline when rejected, and never count as proof | Integration | P1 | FR-192-AC-1, FR-192-AC-2, FR-192-AC-3, FR-192-AC-4 | 🚧 |
| TC-628 | Sampled witnesses are kept in trace order and replay through the model | Integration | P1 | FR-193-AC-1, FR-193-AC-2, FR-193-AC-3 | 🚧 |
| TC-629 | Window aggregates evaluate exactly over past windows and keep the temporal layer Boolean | Integration | P1 | FR-194-AC-1, FR-194-AC-2, FR-194-AC-3, FR-194-AC-4 | 🚧 |
| TC-630 | S3 checks exact-only forms and fairness sets, and EN-5 advertises exact evidence | Unit | P1 | FR-195-AC-1, FR-195-AC-2, FR-195-AC-3, FR-195-AC-4 | 🚧 |
| TC-631 | EN-5 builds the DTMC or MDP product with monitors, accumulators and intermediate states | Integration | P1 | FR-196-AC-1, FR-196-AC-2, FR-196-AC-3, FR-196-AC-4 | 🚧 |
| TC-632 | Backward induction decides finite-horizon forms exactly, or over dyadic intervals with precision doubling | Integration | P1 | FR-197-AC-1, FR-197-AC-2, FR-197-AC-3, FR-197-AC-4 | 🚧 |
| TC-633 | Unbounded reachability and expected rewards are decided by graph precomputation, interval iteration and exact policy iteration | Integration | P1 | FR-198-AC-1, FR-198-AC-2, FR-198-AC-3, FR-198-AC-4, FR-198-AC-5 | 🚧 |
| TC-634 | Long-run fractions are decided exactly by bottom components and maximal end components | Integration | P1 | FR-199-AC-1, FR-199-AC-2, FR-199-AC-3 | 🚧 |
| TC-635 | Every-scheduler claims with a fairness set are decided over fair schedulers through fair end components | Integration | P1 | FR-200-AC-1, FR-200-AC-2, FR-200-AC-3, FR-200-AC-4 | 🚧 |
| TC-636 | check_probability_certificate accepts sound certificates and rejects flawed ones in exact rationals | Integration | P1 | FR-201-AC-1, FR-201-AC-2, FR-201-AC-3, FR-201-AC-4, FR-201-AC-5 | 🚧 |
| TC-637 | replay_probabilistic_witness reproduces path-set and subsystem refutations and refuses altered witnesses | Integration | P1 | FR-202-AC-1, FR-202-AC-2, FR-202-AC-3, FR-202-AC-4, FR-202-AC-6 | 🚧 |
| TC-638 | Exact runs stop on caller-set budgets and settle with ExactValue or ValueBounds | Integration | P1 | FR-203-AC-1, FR-203-AC-2, FR-203-AC-3, FR-203-AC-4 | 🚧 |
| TC-639 | Closed probabilistic timed automata are decided exactly through digital clocks, with the workload resolving delays | Integration | P1 | FR-204-AC-1, FR-204-AC-2, FR-204-AC-3, FR-204-AC-4 | 🚧 |
| TC-640 | A sampled undefined evaluation is a rejection and replays | Integration | P1 | FR-189-AC-6, FR-192-AC-5, FR-193-AC-4 | 🚧 |
| TC-641 | An undefined state reached with positive probability refutes an exact claim | Integration | P1 | FR-196-AC-5, FR-202-AC-5, FR-203-AC-5 | 🚧 |
| TC-642 | A delay no distribution gives is decided by its minimum or maximum under a workload | Integration | P1 | FR-204-AC-5 | 🚧 |
| TC-685 | S3 checks time declarations, clocks, clock constraints, time invariants and urgency | Unit | P1 | FR-230-AC-1, FR-230-AC-2, FR-230-AC-3, FR-230-AC-4 | 🚧 |
| TC-686 | A timed subject's behaviours read as timed traces with delays, urgency, idle tails and digital time | Integration | P1 | FR-231-AC-1, FR-231-AC-2, FR-231-AC-3, FR-231-AC-4 | 🚧 |
| TC-687 | The request carries one time-lock-freedom item per timed subject, and deadlocks, fairness and vacuity read over time | Unit | P1 | FR-232-AC-1, FR-232-AC-2, FR-232-AC-3, FR-232-AC-4 | 🚧 |
| TC-688 | Model claims bind model-time or model-steps, with defaults, identity keys and interval units | Unit | P1 | FR-233-AC-1, FR-233-AC-2, FR-233-AC-3 | 🚧 |
| TC-689 | S3 checks timed intervals with open or closed ends and classifies TT-1 to TT-4, with the punctual-interval boundary | Unit | P1 | FR-234-AC-1, FR-234-AC-2, FR-234-AC-3, FR-234-AC-4 | 🚧 |
| TC-690 | Timed outcomes settle as FR-331 terminal records with zone-certified, exhaustive and new-cause rows | Unit | P1 | FR-235-AC-1, FR-235-AC-2, FR-235-AC-3, FR-235-AC-4 | 🚧 |
| TC-691 | Timed counterexamples carry exact rational delays, a final delay for time-locks, and a time-divergent lasso | Unit | P1 | FR-236-AC-1, FR-236-AC-2, FR-236-AC-3, FR-236-AC-4 | 🚧 |
| TC-692 | The replay facade replays timed counterexamples and time-locks in exact arithmetic | Integration | P1 | FR-237-AC-1, FR-237-AC-2, FR-237-AC-3, FR-237-AC-4, FR-237-AC-6 | 🚧 |
| TC-693 | Zones as difference-bound matrices in exact integer arithmetic | Unit | P1 | FR-238-AC-1, FR-238-AC-2, FR-238-AC-3 | ✅ |
| TC-694 | The zone engine decides timed claims by symbolic search, with finite abstraction, budgets and determinism | Integration | P1 | FR-239-AC-1, FR-239-AC-2, FR-239-AC-3, FR-239-AC-4 | 🚧 |
| TC-695 | Timed liveness under time divergence and fairness, and time-lock search, on the symbolic graph | Integration | P1 | FR-240-AC-1, FR-240-AC-2, FR-240-AC-3, FR-240-AC-4, FR-241-AC-5 | 🚧 |
| TC-696 | Symbolic counterexamples concretize canonically to exact rational delays | Unit | P1 | FR-241-AC-1, FR-241-AC-2, FR-241-AC-3, FR-241-AC-4 | 🚧 |
| TC-697 | Timed formulas translate to claim automata that agree with the evaluator | Unit | P1 | FR-242-AC-1, FR-242-AC-2, FR-242-AC-3 | 🚧 |
| TC-698 | Timed items route to the zone engine, and only exact probabilistic claims to the digital-clock route | Integration | P1 | FR-243-AC-1, FR-243-AC-2, FR-243-AC-3 | 🚧 |
| TC-699 | Every zone-engine proof carries a canonical zone certificate | Unit | P1 | FR-244-AC-1, FR-244-AC-2, FR-244-AC-3, FR-244-AC-4 | 🚧 |
| TC-700 | The in-core checker accepts valid zone certificates and rejects every tampering | Integration | P1 | FR-245-AC-1, FR-245-AC-2, FR-245-AC-3, FR-245-AC-4, FR-245-AC-5, FR-245-AC-6 | 🚧 |
| TC-701 | S3 checks task sets and routes schedulability claims to EN-7 | Unit | P1 | FR-246-AC-1, FR-246-AC-2 | 🚧 |
| TC-702 | EN-7 computes fixed-priority, EDF and AMC verdicts in exact arithmetic | Unit | P1 | FR-247-AC-1, FR-247-AC-2, FR-247-AC-3, FR-247-AC-4 | 🚧 |
| TC-703 | Closed-form verdicts settle only after check_closed_form recomputes their evidence | Unit | P1 | FR-248-AC-1, FR-248-AC-2, FR-248-AC-3, FR-248-AC-4 | 🚧 |
| TC-704 | Task automata lower to timed subjects for deadline checking, and the stopwatch class is unsupported | Integration | P1 | FR-249-AC-1, FR-249-AC-2 | 🚧 |
| TC-705 | Hybrid solver results map to proved or inconclusive, never refuted | Unit | P1 | FR-250-AC-1, FR-250-AC-2 | 🚧 |
| TC-706 | Tick-based monitor plans round soundly, size buffers from the event rate and fault on counter errors | Unit | P1 | FR-251-AC-1, FR-251-AC-2, FR-251-AC-3 | 🚧 |
| TC-707 | QSL writes monitor-agreement, tick-arithmetic and timestamp-contract obligations, and disposes elapsed-time obligations unsupported | Unit | P1 | FR-252-AC-1, FR-252-AC-2 | 🚧 |
| TC-708 | Delay distributions check, windows are exact and the race resolves ties by the workload | Unit | P1 | FR-253-AC-1, FR-253-AC-2, FR-253-AC-3, FR-253-AC-4 | 🚧 |
| TC-709 | Timed runs sample with exact rational delays, measure timed events and replay | Integration | P1 | FR-254-AC-1, FR-254-AC-2, FR-254-AC-3, FR-254-AC-4 | 🚧 |
| TC-710 | An undefined timed claim evaluation refutes with a timed prefix and replays | Integration | P1 | FR-239-AC-5, FR-237-AC-5, FR-235-AC-5 | 🚧 |
| TC-565 | S3 carries the symmetric annotation and refuses a fold over its references | Unit | P1 | FR-150-AC-1, FR-150-AC-2 | 🚧 |
| TC-566 | Every identity-observing form is refused in every clause kind, and transparent forms check | Unit | P1 | FR-150-AC-3, FR-150-AC-4 | 🚧 |
| TC-567 | Symmetry declarations admit on an annotated population and refuse malformed forms | Unit | P1 | FR-151-AC-1, FR-151-AC-2 | 🚧 |
| TC-568 | Initial states not closed under the generators settle SymmetryBroken; a finer class admits | Unit | P1 | FR-151-AC-3, FR-151-AC-4 | 🚧 |
| TC-569 | The sort canonicaliser maps each state into its orbit and coalesces orbits | Unit | P1 | FR-152-AC-1, FR-152-AC-2, FR-152-AC-3, FR-152-AC-4 | 🚧 |
| TC-570 | Weak each fairness is decided on the annotated quotient | Integration | P1 | FR-153-AC-1, FR-153-AC-2 | 🚧 |
| TC-571 | Strong each fairness refines on the quotient, and every verdict equals the unreduced verdict | Integration | P1 | FR-153-AC-3, FR-153-AC-4 | 🚧 |
| TC-572 | A receiver-scoped modifies entry limits candidates and check_frame to the receiver | Integration | P1 | FR-154-AC-1, FR-154-AC-2, FR-154-AC-5 | 🚧 |
| TC-573 | The receiver scope holds in the Frame run and admission, and in the emitted frame node | Integration | P1 | FR-154-AC-3, FR-154-AC-4 | 🚧 |
| TC-574 | Read, write and enabling footprints derive from clauses and frames, and decide independence | Unit | P1 | FR-155-AC-1, FR-155-AC-2 | 🚧 |
| TC-575 | Footprints are enforced: a read outside the footprint is an internal fault, a write outside the frame a violation | Integration | P1 | FR-155-AC-3, FR-155-AC-4 | 🚧 |
| TC-576 | Ample sets reduce ADR-021 §7.2 to one interleaving and keep the deadlock | Integration | P1 | FR-156-AC-1, FR-156-AC-2 | 🚧 |
| TC-577 | The breadth-first proviso prevents ignoring, the closure reaches through disabled members, and choice is deterministic | Integration | P1 | FR-156-AC-3, FR-156-AC-4 | 🚧 |
| TC-578 | Fairness visibility keeps a fair violation that C0 to C3 alone would discard | Integration | P1 | FR-157-AC-1, FR-157-AC-2 | 🚧 |
| TC-579 | Fairness visibility applies only under a non-empty fairness set, at either granularity, and over protocols | Integration | P1 | FR-157-AC-3, FR-157-AC-4 | 🚧 |
| TC-580 | A state constraint stores boundary states, reports real violations and never proves | Integration | P1 | FR-158-AC-1, FR-158-AC-2 | 🚧 |
| TC-581 | Undefined, absent and initially-false constraints, bounds, identity and determinism | Integration | P1 | FR-158-AC-3, FR-158-AC-4 | 🚧 |
| TC-582 | The preservation table admits or settles each selected reduction before expansion | Unit | P1 | FR-159-AC-1, FR-159-AC-2 | 🚧 |
| TC-583 | Reductions route only to advertising candidates, need footprints, and enter the obligation identity | Integration | P1 | FR-159-AC-3, FR-159-AC-4 | 🚧 |
| TC-584 | Reduced proofs, reduction causes, depth and refutations settle through the one map | Unit | P1 | FR-160-AC-1, FR-160-AC-2, FR-160-AC-3, FR-160-AC-4, FR-160-AC-7 | 🚧 |
| TC-585 | Symmetry counterexamples concretise to subject traces, closing a loop in one or more passes | Integration | P1 | FR-161-AC-1, FR-161-AC-2 | 🚧 |
| TC-586 | Each-fairness loops take every identity, and every reduced counterexample is one ordinary trace kind | Integration | P1 | FR-161-AC-3, FR-161-AC-4 | 🚧 |
| TC-587 | TransitionSystem hooks offer reductions, the product delegates them, and simulation stays unreduced | Integration | P1 | FR-162-AC-1, FR-162-AC-2, FR-162-AC-3, FR-162-AC-4 | 🚧 |
| TC-588 | An undefined claim evaluation found on a reduced run refutes with a concrete prefix | Integration | P1 | FR-160-AC-5 | 🚧 |
| TC-589 | Complete enabling footprints and membership locations keep safety violations under partial-order reduction | Integration | P1 | FR-155-AC-5, FR-155-AC-6, FR-156-AC-5, FR-156-AC-6 | 🚧 |
| TC-617 | Complete enabling footprints and membership keep fair violations under partial-order reduction | Integration | P1 | FR-157-AC-5, FR-157-AC-6 | 🚧 |
| TC-886 | Reduced and unreduced runs agree over the model-check corpus | Integration | P1 | FR-160-AC-6 | 🚧 |
| TC-815 | S1 and S2 build union declaration and case forms | Unit | P1 | FR-313-AC-1, FR-313-AC-2 | 🚧 |
| TC-816 | S1 parses a case scrutinee without a top-level record and keeps protocol case apart | Unit | P1 | FR-313-AC-3 | 🚧 |
| TC-817 | S3 admits union declarations, including an escaping recursive union | Unit | P1 | FR-316-AC-1, FR-316-AC-2 | 🚧 |
| TC-818 | S3 refuses ill-formed union declarations and bounds them only by checking ceilings | Unit | P1 | FR-316-AC-2, FR-316-AC-3, FR-316-AC-4 | 🚧 |
| TC-819 | S3 checks union construction and refuses FR-143's four construction errors | Unit | P1 | FR-317-AC-1, FR-317-AC-2, FR-317-AC-4 | 🚧 |
| TC-820 | S3 resolves qualified member names by candidate and refuses ambiguity | Unit | P1 | FR-317-AC-3 | 🚧 |
| TC-821 | S3 checks a case, types its binders per arm and decides its result type by QSpec FR-146 | Unit | P1 | FR-318-AC-1, FR-318-AC-2 | 🚧 |
| TC-822 | S3 refuses each exhaustiveness obligation with its payload and locus | Unit | P1 | FR-318-AC-3 | 🚧 |
| TC-823 | S3 reports exactly one refusal per case, in builder order | Unit | P1 | FR-318-AC-4 | 🚧 |
| TC-824 | A case checks under each clause kind and adds no requirement record | Unit | P1 | FR-318-AC-5 | 🚧 |
| TC-825 | Deeply nested case checks under default ceilings and stops only at a named ceiling | Unit | P1 | FR-318-AC-6 | 🚧 |
| TC-826 | Union, member and case identities are stable and member identity is a retyped VariantId | Unit | P1 | FR-319-AC-1, FR-319-AC-2, FR-319-AC-3 | 🚧 |
| TC-827 | Union nodes lower and emit in QSpec's spelling and survive the I2 read and recompile | Integration | P1 | FR-320-AC-1, FR-320-AC-2 | 🚧 |
| TC-828 | A value-validity item containing case settles unsupported downstream | Integration | P1 | FR-320-AC-3 | 🚧 |
| TC-829 | Argument admission refuses ill-formed supplied union values before any charge | Unit | P1 | FR-321-AC-1, FR-321-AC-2 | 🚧 |
| TC-830 | Union values round-trip through v2 union_value nodes at any depth | Unit | P1 | FR-321-AC-3, FR-321-AC-4 | 🚧 |
| TC-831 | S6a evaluates case and construction, propagates stopped operands and faults on a broken invariant | Unit | P1 | FR-322-AC-1, FR-322-AC-2, FR-322-AC-4 | 🚧 |
| TC-832 | S6a charges union construction at QSpec's accounting point and nothing for case selection | Unit | P1 | FR-322-AC-3 | 🚧 |
| TC-833 | Union values as collection elements, in queries and in predicates | Unit | P1 | FR-323-AC-1, FR-323-AC-2, FR-323-AC-3, FR-046-AC-9 | 🚧 |
| TC-834 | Unions compile under the value profile with the same lock selections as records | Integration | P1 | FR-324-AC-1 | 🚧 |
| TC-720 | A reached limit names its kind, bound, count and setting | Unit | P1 | FR-255-AC-1, FR-255-AC-2, FR-255-AC-3 | 🚧 |
| TC-721 | Every setting raises its limit through the library, the replay request and the settings operation the driver CLI calls | Integration | P1 | FR-255-AC-4, FR-255-AC-5, FR-255-AC-6 | 🚧 |
| TC-749 | The root native parser parses 100,000-deep sources on a small stack | Integration | P1 | FR-256-AC-4 | ✅ |
| TC-750 | `quire-spec parse` takes caller limits and parses a 100,000-deep source | Integration | P1 | FR-256-AC-5 | ✅ |
| TC-722 | Deep sources parse on a small stack under raised S1 limits | Unit | P1 | FR-256-AC-1 | 🚧 |
| TC-723 | S1 parses to its default limits and names the setting it reached | Unit | P1 | FR-256-AC-2, FR-256-AC-3 | 🚧 |
| TC-724 | Deep forms and control anchors build on a small stack | Unit | P1 | FR-257-AC-1, FR-257-AC-2, FR-257-AC-3 | 🚧 |
| TC-725 | Deep expressions check, lower and emit on a small stack | Unit | P1 | FR-258-AC-1, FR-258-AC-5 | 🚧 |
| TC-726 | Deep types key and lower their text leaves on a small stack | Unit | P1 | FR-258-AC-2 | 🚧 |
| TC-727 | Checker defaults admit any depth that fits, and its limit outcomes name the setting | Unit | P1 | FR-258-AC-3, FR-258-AC-4 | 🚧 |
| TC-728 | A deep package's identities do not depend on the stack, and byte limits report as limits | Unit | P1 | FR-259-AC-1, FR-259-AC-2, FR-259-AC-4 | 🚧 |
| TC-729 | QSL identities, digests and allocation failures over quire-canonical at any depth | Unit | P1 | FR-259-AC-3, FR-259-AC-5 | 🚧 |
| TC-730 | Intake judges a deep package document on its content | Integration | P1 | FR-260-AC-1, FR-260-AC-2, FR-260-AC-5 | 🚧 |
| TC-731 | Intake reports composite cycles of any length | Integration | P1 | FR-260-AC-3 | 🚧 |
| TC-732 | The intake byte limit names its setting and clears when raised | Integration | P1 | FR-260-AC-4 | 🚧 |
| TC-733 | Library preimage and observation reads judge deep documents on their content | Integration | P1 | FR-261-AC-1, FR-261-AC-2, FR-261-AC-3 | 🚧 |
| TC-734 | A 100,000-deep recursive call completes on work fuel | Unit | P1 | FR-262-AC-1 | 🚧 |
| TC-735 | Deep values and value types evaluate, key, compare, clone and drop | Unit | P1 | FR-262-AC-2 | 🚧 |
| TC-736 | A deep source and value replay to the proving run's verdict | Integration | P1 | FR-263-AC-1, FR-070-AC-10 | ✅ |
| TC-737 | Replay passes every stage limit through and names the setting it reached | Integration | P1 | FR-263-AC-2, FR-263-AC-3 | 🚧 |
| TC-738 | A deep package emits, reads back and verifies, in the stratified grammar | Integration | P1 | FR-264-AC-1, FR-264-AC-2 | 🚧 |
| TC-739 | The v2 read refuses an inline nested term and names its limits' settings | Unit | P1 | FR-264-AC-3, FR-264-AC-4, FR-264-AC-5 | 🚧 |
| TC-902 | Core entry points and maybe_grow sites run 100,000 deep on a small stack | Unit | P1 | FR-356-AC-5, FR-356-AC-6 | 🚧 |
| TC-903 | A deep-input fuzz target drives the parser and the checker | Property | P1 | FR-356-AC-7 | 🚧 |
| TC-904 | Scalar-parity claims replay to diverged, agrees, refused and faulted outcomes, and never to Refuted | Unit | P1 | FR-357-AC-1, FR-357-AC-2, FR-357-AC-3, FR-357-AC-4, FR-357-AC-5, FR-357-AC-6, FR-357-AC-7, FR-357-AC-8, FR-357-AC-9, FR-357-AC-10, FR-357-AC-11, FR-357-AC-13, FR-357-AC-14, FR-357-AC-15, FR-357-AC-16, FR-357-AC-17, FR-357-AC-18, FR-357-AC-19 | ✅ |
| TC-905 | A witness value text decodes exactly for every composite and leaf family, and each malformed form refuses | Unit | P1 | FR-070-AC-8, FR-070-AC-9, FR-070-AC-11, FR-070-AC-12 | ✅ |
| TC-906 | Composite and leaf-family arguments replay, and refuse by kind, by domain and at the request's limits | Unit | P1 | FR-098-AC-8, FR-098-AC-9, FR-098-AC-10 | 🚧 |
| TC-907 | A composite equality-parity claim settles Diverged, Agrees or refused when falsified, and by rows V-1 to V-5 when verified | Unit | P1 | FR-358-AC-1, FR-358-AC-2, FR-358-AC-3, FR-358-AC-4, FR-358-AC-5, FR-358-AC-6, FR-358-AC-7, FR-358-AC-8 | 🚧 |
| TC-908 | The replay facade compiles QSL source to the checked-package bytes the spine emits | Integration | P2 | FR-060-AC-5, FR-060-AC-6, FR-060-AC-7 | ✅ |
| TC-909 | Integer bounds and literals up to i128 check, and one beyond refuses at check | Unit | P1 | FR-091-AC-36, FR-091-AC-37, FR-091-AC-38 | ✅ |
| TC-910 | Wide integer ranges key to their vectors, and counters keep one exact spelling | Unit | P1 | FR-092-AC-14, FR-092-AC-15 | ✅ |
| TC-911 | Field refinement decides wide integer domains and literals exactly | Unit | P1 | FR-056-AC-16, FR-082-AC-9 | ✅ |
| TC-912 | The IR wire round-trips integer bounds and literals up to i128 | Integration | P1 | FR-033-AC-6 | 🚧 |
| TC-913 | A wide-range source recompiles to its package_id and replays | Integration | P1 | FR-098-AC-11 | ✅ |
| TC-894 | An imported call discharges its preconditions as a local call does | Integration | P1 | FR-099-AC-8 | 🚧 |
| TC-895 | The native checker reads a field's optionality from its presence | Unit | P1 | FR-016-AC-10 | 🚧 |
| TC-896 | A field redefinition narrows presence and multiplicity on separate axes | Unit | P1 | FR-082-AC-8 | 🚧 |
| TC-897 | A Text scalar type is admitted with its QSpec profile | Unit | P1 | FR-056-AC-11 | 🚧 |
| TC-742 | native-run-result/2 carries basis and witness, and its strict reader refuses malformed and /1 documents | Integration | P1 | FR-267-AC-1, FR-267-AC-2, FR-267-AC-3 | 🚧 |
| TC-743 | Replay re-derives a state-clause witness and checks that the element separates the clause | Integration | P1 | FR-268-AC-1, FR-268-AC-2, FR-268-AC-3, FR-268-AC-4 | 🚧 |
| TC-744 | A witness disagreement settles inconclusive with a typed Witness cause that round-trips | Unit | P1 | FR-269-AC-1, FR-269-AC-2, FR-269-AC-3 | 🚧 |
| TC-891 | A clause-run request file runs through run_clause and writes a /2 document | Integration | P1 | FR-312-AC-1, FR-312-AC-2, FR-312-AC-3, FR-312-AC-4 | 🚧 |
| TC-779 | Each non-interpreter backend matches the interpreter over the conformance corpus and generated programs | Integration | P1 | FR-295-AC-1 | 🚧 |
| TC-780 | Backends match the interpreter on budgets, limits and internal errors, and the outcome names no backend | Integration | P1 | FR-295-AC-2, FR-295-AC-3, FR-295-AC-4 | 🚧 |
| TC-781 | The request selects the execution backend and no other backend runs in its place | Integration | P1 | FR-296-AC-1, FR-296-AC-2, FR-296-AC-3 | 🚧 |
| TC-778 | The interpreter implements the execution seam, and the seam admits checked input only | Unit | P1 | FR-294-AC-1, FR-294-AC-2, FR-294-AC-3, FR-294-AC-4 | 🚧 |
| TC-773 | Installing, removing or reordering providers changes nothing outside the touched candidate sets | Unit | P1 | FR-289-AC-1, FR-289-AC-2, FR-289-AC-3 | 🚧 |
| TC-774 | Plugin results settle as typed records, and plugin proofs carry the trusted label | Unit | P1 | FR-290-AC-1, FR-290-AC-2, FR-290-AC-3, FR-290-AC-4 | 🚧 |
| TC-775 | Plugin-run failures settle as typed records of the plugin's own items | Unit | P1 | FR-291-AC-1, FR-291-AC-2, FR-291-AC-3 | 🚧 |
| TC-772 | One manifest conversion serves compile-time providers and plugins | Unit | P1 | FR-288-AC-1, FR-288-AC-2, FR-288-AC-3, FR-288-AC-4, FR-288-AC-5, FR-288-AC-6 | 🚧 |
| TC-764 | Certificate checkers accept genuine certificates and reject altered or misbound ones in a core-only process | Integration | P1 | FR-282-AC-1, FR-282-AC-2, FR-282-AC-3, FR-282-AC-4, FR-282-AC-5, FR-282-AC-6, FR-282-AC-7, FR-282-AC-8 | 🚧 |
| TC-765 | monitor reports violations, pending and tested verdicts over finite traces and lassos | Unit | P1 | FR-283-AC-1, FR-283-AC-2, FR-283-AC-5 | 🚧 |
| TC-766 | monitor refuses inadmissible traces and agrees with replay's trace evaluation | Unit | P1 | FR-283-AC-3, FR-283-AC-4, FR-283-AC-6 | 🚧 |
| TC-767 | The direction check keeps frontends, analyze, the cache and the plugin host out of the qualified core | Integration | P1 | FR-284-AC-1, FR-280-AC-3 | ✅ |
| TC-768 | Core operations read no ambient input and write nothing to the process streams | Integration | P1 | FR-284-AC-2, FR-284-AC-3, FR-284-AC-4 | 🚧 |
| TC-785 | The compile command is the composition of the front-end operations | Unit | P1 | FR-027-AC-11 | 🚧 |
| TC-786 | run's exit statuses come from the exit function, with undefined at 10 | Unit | P1 | FR-100-AC-10, FR-100-AC-11 | ✅ |
| TC-770 | Library outcomes serialize to one outcome document, with the undefined label on non-proof outcomes only | Unit | P1 | FR-286-AC-1, FR-286-AC-2, FR-286-AC-3, FR-286-AC-4, FR-286-AC-5, FR-286-AC-6, FR-357-AC-12 | 🚧 |
| TC-771 | QSL builds no binary, and command is a library operation that writes nothing | Integration | P1 | FR-287-AC-1, FR-287-AC-2 | 🚧 |
| TC-769 | The exit function maps every category and multi-item outcome, with undefined at 10 | Unit | P1 | FR-285-AC-1, FR-285-AC-2, FR-285-AC-3, FR-285-AC-4, FR-285-AC-5 | 🚧 |
| TC-776 | analyze requests and records have one canonical form that changes only with key members | Unit | P1 | FR-292-AC-1, FR-292-AC-2, FR-292-AC-3 | 🚧 |
| TC-777 | QSL records tell the cache what it never stores, and hold no wall time | Unit | P1 | FR-293-AC-1, FR-293-AC-2 | 🚧 |
| TC-758 | Every bound is a caller limit that names itself when reached | Unit | P1 | FR-277-AC-1, FR-277-AC-2, FR-277-AC-3 | 🚧 |
| TC-759 | The front-end operations compose to spine compile | Unit | P1 | FR-278-AC-1, FR-278-AC-2, FR-278-AC-3 | ✅ |
| TC-756 | No lifecycle operation panics on arbitrary input | Property | P1 | FR-275-AC-4 | 🚧 |
| TC-757 | A cancelled operation stops within one charge and emits nothing | Unit | P1 | FR-276-AC-1, FR-276-AC-2, FR-276-AC-3, FR-276-AC-4, FR-276-AC-5 | 🚧 |
| TC-760 | execute returns what CheckedPackage::call and evaluate return | Unit | P1 | FR-279-AC-1, FR-279-AC-2, FR-279-AC-3 | 🚧 |
| TC-761 | QSL engine manifests register, and the provider entry returns analyze's records | Unit | P1 | FR-280-AC-1, FR-280-AC-2 | 🚧 |
| TC-782 | inspect builds each typed view from the source map | Unit | P1 | FR-297-AC-1, FR-297-AC-2, FR-297-AC-3, FR-297-AC-4 | 🚧 |
| TC-783 | render produces text and DOT from typed values only, leaving JSON unchanged | Unit | P1 | FR-298-AC-1, FR-298-AC-2, FR-298-AC-3, FR-298-AC-4 | 🚧 |
| TC-784 | One Value-family source runs through check, package and execute with documents and exit codes | Integration | P1 | FR-299-AC-1, FR-299-AC-2, FR-299-AC-3, FR-299-AC-4 | 🚧 |
| TC-762 | analyze settles proved, refuted and unsupported items with one record each | Integration | P1 | FR-281-AC-1, FR-281-AC-2, FR-281-AC-3 | 🚧 |
| TC-763 | analyze settles rejected certificates, unreproduced counterexamples and budgets without a false verdict | Integration | P1 | FR-281-AC-4, FR-281-AC-5, FR-281-AC-6, FR-281-AC-7 | 🚧 |
