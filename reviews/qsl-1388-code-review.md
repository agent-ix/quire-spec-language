---
id: SR-202611021388
title: 'Code review (rust lane) of QSL PR #2088: callable fma/totalOrder/bitIdentical and Float ordering (QSL-1388)'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@b3c776659; git diff origin/main...HEAD (PR #2088, branch peter/qsl-1388-float-callables): qsl-semantics/src/check/{check.rs,check/typing.rs,claims.rs,family.rs,lowering.rs,ir.rs,references.rs}, qsl-eval/src/{smt/expression.rs,value/expression/evaluate.rs}, tests under qsl-eval and qsl-replay, FR-093 table; ticket QSL-1388'
review_set: subset
date: '2026-11-02'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-397
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-093
  type: reviews
---

## Summary

Ticket: QSL-1388. PR: quire-spec-language#2088, head b3c776659. Static review of the diff only; no cargo run. Cargo.lock moves quire-eval and quire-checked to 7a4135ad with no Cargo.toml change and no added allow attribute.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An intrinsic call argument is not typed from its sibling operand as FR-397 requires. Application::parameter gives every argument only the whole-call float hint, and the Boolean intrinsics get none, so totalOrder(a, float32(bits: 0x0)) with a: Float32[nearest-even] types the constructor exact and refuses type-mismatch. No test mixes a parameter with a constructor in one intrinsic call. | qsl-semantics/src/check/family.rs:1169-1180; spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:74-80 |
| FND-002 | medium | The FR-093 row the PR adds for IeeeSquareRoot says operator unary, but the QSpec operation catalog (checked-operation-catalog-v1 at the locked quire-verification-contracts) lists quire.op.ieee.float32.sqrt and float64.sqrt with operator call. fma is call in both. The new row records an existing lowering that contradicts the catalog. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:351; qsl-semantics/src/check/lowering.rs:3940-3961 |
| FND-003 | medium | The float ordering composition shares operand node ids, but the evaluator walks node children as a tree, so a float operand is evaluated three times (numericEqual of itself, totalOrder, numericEqual of the pair) and each evaluation re-charges the meter and re-records its IEEE flag loss at the same location. FR-397 says only that nodes are shared; charging and evaluation count are unspecified and are routed to the planner. No test uses a non-constant operand in an ordering or asserts the flag-loss count. | qsl-semantics/src/check/check.rs:3135-3176; qsl-eval/src/value/expression/evaluate.rs:1396-1401,1864-1866 |
| FND-004 | low | A float ordering inside a claim or state clause now carries synthesized and/or/not nodes at the ordering's single location. The short-circuit evaluator records an FR-265 decision for each of them (decide), which the source did not author. No test evaluates a float ordering inside a clause or claim; AC-20 uses only !=. | qsl-semantics/src/check/check.rs:3157-3175; qsl-eval/src/value/expression/evaluate.rs:1621-1626,3804-3810 |
| FND-005 | low | AC-7 asserts the arguments are in call order, but the test only requires the two argument references to differ, which holds in either order. The test also keeps a dead width binding (let _ = width). | qsl-replay/tests/it/compile_ieee_profile.rs:244-266 |
| FND-006 | low | The SMT refusal of total_order and bit_identical, the float ordering reaching the SMT reader, and the observed outcome of the omitted zero-argument sqrt() each have no test. The refusal is sound (an explicit refuse), but nothing pins it, and the claim that the grammar rejects sqrt() is not recorded by any test. | qsl-eval/src/smt/expression.rs:416-419; spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md (FR-397-AC-16) |

## Verdict

Not clean for merge as written: FND-001 is a spec-mandated typing rule left unimplemented for the new binary intrinsics, and FND-002 puts catalog-contradicting text in FR-093. FND-003 needs either a test that records current behaviour or the planner ruling. The rest is low.

Answers to the disclosed points. (a) The claims.rs exclusion is correct: key_claims requires exactly one scalar application per location (claims.rs:1046-1060), and an ordering lowers to four IEEE applications at one location, so they cannot be claim sites. The relations return a Boolean over total operations, so dropping the former float = claim site loses no requirement; scalar_equality already treats Float as not scalar. (b) The result-hint pass to the first arithmetic operand is spec-backed (FR-397 expected-type rule), and the contextual-operand swap in typing.rs means only two constructor operands are affected; F04b would not type without it. (c) fma as call with a rounding mode matches the catalog. (d) See FND-003. (e) The omitted zero-arg cases and the arity-before-profile order are spec gaps correctly not guessed; nothing else is narrowed, but FND-006 asks for the omission to be recorded. (f) NaN, signed-zero and flag behaviour in the AC-13 vectors and F04b are asserted at both widths; SMT refuses explicitly. (g) The flipped tests (x05, the old ineligible loop, the old missing-name test) are justified by FR-397-AC-12 and AC-7. (h) Lock and Cargo.toml are as required.

## New findings (disposition pass 1)

Head 1da095f91, rebased on origin/main 347e9769e. Full `git diff origin/main...HEAD` reviewed by the code-review, rust-review and gap-analysis lanes.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | Nothing pins the evaluator's suppression of synthesized float-ordering decisions. The only test (a_float_ordering_adds_no_connective_a_decision_could_attach_to) inspects checker node locations, never evaluates a decision. With the guard in Task::Short replaced by an always-true condition, the whole qsl-eval and qsl-replay it suites stayed green (mutation run). The guard also keys on a location coincidence, which is how the evaluator tells synthesized from authored connectives, with no explicit marker. | qsl-eval/src/value/expression/evaluate.rs:1630-1634; qsl-eval/tests/it/equality_matrix.rs:2616-2650 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Application::intrinsic_hint types an argument from the first float sibling else the call's expected float, and arguments are checked contextual-last. Test bit_constructors_take_their_type_from_a_sibling_operand_in_every_intrinsic passes; it went red when intrinsic_hint ignored siblings. |
| FND-002 | fixed | Sqrt now lowers as Operator::Call; numeric_equal moved to call as well; catalog checked-operation-catalog-v1 at quire-verification-contracts 5070338 lists sqrt, fma, numeric_equal, total_order, bit_identical all as call. FR-093 row text now agrees. Changing sqrt back to unary turned strict_ir_reader_admits_fma_relations_and_float_orderings red. |
| FND-003 | fixed | float_ordering binds each non-slot, non-literal operand in a Let so it is evaluated once; tests each_ordering_operand_is_evaluated_exactly_once and an_ordering_spends_each_operand_once_and_stops_at_its_limit pass and both went red with the hoisting disabled. |
| FND-004 | fixed | Behavior fixed in Task::Short (no decision for a connective at its left operand's location). The regression coverage is weak; see FND-007. |
| FND-005 | fixed | The test now compares the call against the exchanged call, asserting arguments[0] equals exchanged[1] and arguments[1] equals exchanged[0]; the dead width binding is gone. |
| FND-006 | fixed | smt_query.rs ieee_relations_and_float_orderings_are_refused_as_out_of_fragment pins the total_order, bit_identical and ordering refusals; zero_argument_intrinsic_calls_are_refused_by_the_parser and zero_argument_sqrt_is_a_parse_refusal_with_and_without_a_float_form pin the sqrt() and bitIdentical() outcome (invalid_syntax at the closing parenthesis), matching FR-397-AC-16 and AC-25 as now written on main. |

## Verdict (round 1)

All six prior findings are fixed. One new low finding (FND-007). Builds and tests: qsl-eval (164 lib, 879 it), qsl-replay (919 lib, 195 it plus facade tests), qsl-semantics (749 passed) all green except the known check::lowering::model::tests::reference_types_key_over_their_model_nodes_and_record_the_correspondence, which also fails on clean main and is fixed by PR #2093. cargo fmt --check clean. No new allow attribute, no unwrap or expect on a production path (all in tests), no pins outside Cargo.lock; the Cargo.lock change moves only quire-checked and quire-eval to 7a4135ad and leaves contract-ir at main's commit. Mergeable.
