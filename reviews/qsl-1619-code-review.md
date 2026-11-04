---
id: SR-2277001
title: 'Code review of PR #2277: decimal(c, s) source constructor, code-review with rust-review lane'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@79c8e075d; git diff origin/main...79c8e075d (PR #2277, branch qsl-1619-decimal-source-forms): qsl-forms, qsl-semantics/src/check, qsl-package/src/emit; ticket QSL-1619'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1619 (overlaps QSL-1394 implementation half). PR: quire-spec-language#2277, head 79c8e075d, based on origin/main 4308e48d3. Method: code-review with the rust-review lane. S2 builds `ExprNode::DecimalValue`, the checker types it by its unique expected `Decimal[..]` type with a by-value membership test, a nonzero literal divisor discharges the nonzero obligation, and lowering spells the wire literal. Gates are green except 19 known qsl-replay failures. Two defects are in the production change: a panic on a member literal with scale 65535 or more, and the peer rule FR-104 states is not implemented for a literal beside a typed operand when no expected type exists.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| decimal_literal | qsl-semantics/src/check/check.rs:2947-2980 | examined |
| arithmetic peer hint | qsl-semantics/src/check/check/typing.rs:1851-1870 | examined |
| nonzero literal divisor | qsl-semantics/src/check/facts.rs:1140-1165 | examined |
| LiteralValue::decimal | qsl-semantics/src/check/node_key/mod.rs:1557-1577 | examined |
| S2 ExactNumber mapping | qsl-forms/src/value.rs:1887-1900 | examined |
| emit literal_value | qsl-package/src/emit.rs:1550 | examined |
| identity encodings | qsl-semantics/src/check/family.rs, hyper_clause.rs | examined |
| FR-104-AC-13, FR-104-AC-16, FR-091-AC-57, FR-091-AC-58, FR-140-AC-8 | spec | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | LiteralValue::decimal panics ("Formatting argument out of range") for any member literal with scale 65535 or more, such as decimal(0, 4294967295); the checker must refuse or spell it. | qsl-semantics/src/check/node_key/mod.rs:1573 |
| FND-002 | high | A decimal literal beside a typed decimal operand takes no type from the peer when the position supplies no expected type: `p + decimal(1, 2) = p` is refused ambiguous-literal although FR-104-AC-13 says it checks with the literal typed P. | qsl-semantics/src/check/check/typing.rs:1851-1870 |
| FND-003 | medium | decimal(-0, 2) passes S1 and is refused by S2 as UnexpectedShape; FR-091-AC-58 requires invalid_syntax/invalid-token at S1. decimal(+1, 2) is refused with cause unexpected-token, not invalid-token. | qsl-forms/src/value.rs:1887-1900 |
| FND-004 | low | A scale above u32::MAX (decimal(1, 4294967296)) is refused by S2 as the generic UnexpectedShape, with no cause that names the scale. | qsl-forms/src/value.rs:1890-1895 |
| FND-005 | low | -decimal(5, 0) with expected type P is refused ambiguous-literal; FR-104 types a unary minus by its operand's decimal type. | qsl-semantics/src/check/check/typing.rs |

## Verdict

Not mergeable until FND-001 and FND-002 are fixed or FND-002 is explicitly accepted as out of this ticket's scope. D24, D26, D27 and D28 behave as QSpec TC-185 states.

Evidence, in a trial merge on origin/main 4308e48d3 (the branch is one commit ahead of it) with CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 QSPEC_DIR=quire-specification@ad180f162 and an own target dir:

- fmt, check --locked --workspace --all-targets, clippy -D warnings default and --no-default-features, arch-lint qualified-core and api-surface --qsl . --qsl-only: exit 0.
- qsl-forms 25+192+2 passed; qsl-semantics lib 795 passed; qsl-semantics --test it 529 passed, 0 failed; qsl-package 618 passed, 0 failed; qsl-eval 182+1059+42 passed, 0 failed; root --test it 878 passed, 0 failed.
- qsl-replay: 19 failures, the known set. lib 14: spine::clause::tests::frame_occurrence_region::frame_occurrence_region_is_the_first_naming_clause_or_attempt_in_source_order and spine::clause::tests::layers::equality::{core_option, core_record, core_reference, core_sequence, graph_option, graph_record, graph_sequence, nested_reference_families_require_both_layers, queries_option, queries_record, queries_reference, queries_sequence, value_reference}. dispatch_link_compile 4: a_family_with_no_body_for_a_concrete_subtype_refuses_at_s3, a_unit_with_several_ambiguous_families_is_refused_once_naming_each, an_unrelated_family_of_the_same_name_refuses_only_when_a_call_reaches_it, unit_compile_refuses_a_family_past_the_callers_family_steps. doctest 1: qsl-replay/src/execute.rs - execute::replay_observed_temporal (line 815).
- The coder's 29 qsl-semantics `it` failures and 6 qsl-package lib failures do not reproduce: qsl-semantics --test it is 529 passed, 0 failed on both clean origin/main and the branch, and qsl-semantics lib is 795 passed on both.

Probes (scratch copy of the branch, not the PR): decimal(0, 65534) completes; decimal(0, 65535) and decimal(0, 4294967295) panic at node_key/mod.rs:1573 (Rust format width arguments are limited to 65535). The member test normalizes zero to scale 0, so every zero coefficient is a member whatever its scale. A nonzero coefficient with a huge scale is not a member (normalized scale above smax) and is refused type-mismatch, so the panic, not an unbounded allocation, is the failure; no limit in the spec bounds a literal's scale below u32::MAX, and FR-091-AC-57 requires S2 to build scale 4294967295.

Mutation checks in a scratch copy: letting a non-member literal through fails d24, d28 and the expected-type test; dropping the nonzero discharge fails d27; removing the literal hint for the first operand fails five of six decimal_source_forms tests. Soundness of the discharge: it fires only for a `NodeKind::Literal` of `Value::Decimal` with a nonzero coefficient, so decimal(0, 2), a parameter, `p / q` and a `(if .. then decimal(5,2) else decimal(6,2))` divisor are all refused unproved-nonzero; a nonzero coefficient is nonzero at every scale. Rule correctness: the literal-takes-its-unique-expected-type, no-expected-type ambiguous-literal, non-decimal type-mismatch and non-member type-mismatch rules match FR-104-AC-16 e1, e2, e3, e4, e5, e6 (probed) and QSpec FR-140-AC-8.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | A quotient of two operands of one typed decimal type is refused ill_typed/ambiguous-literal when no expected type exists (`let s = a / b in s = a`), and the test asserts that refusal; FR-104 says a binary `/` is typed the same way as `+`, `-` and `*`, wherever it is written and whether or not its position supplies an expected type. The code comment cites FR-140-AC-3, which says nothing about typing. | qsl-semantics/src/check/check.rs:3541-3555 |
| FND-007 | medium | MAX_LITERAL_SCALE (65535) refuses `decimal(0, 65536)` and `decimal(0, 4294967295)` as ill_typed/type-mismatch although a zero coefficient is a member of every type (FR-140-AC-5: membership is total up to scale u32::MAX; FR-140-AC-8: a member literal is admitted and retained as spelled) and S2 builds scale 4294967295 (FR-091-AC-57). The cause misdescribes the refusal and no spec text states the bound. | qsl-semantics/src/check/check.rs:2224-2228, 2992-2996 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | The running total of literal scales is an AtomicU64 in qsl_foundation::tally held by the check Scope, which keeps it out of the TC-160 / FR-062-AC-3 source scan that forbids atomic types under src/check by name; it is neither global nor thread-local and is deterministic for the sequential check (probed: no double counting at 7 literal positions), but the scan's rule and FR-062-AC-3 say nothing that allows a per-check counter, so the allowance rests on the wrapper's location. | qsl-foundation/src/tally.rs; qsl-semantics/src/check/family.rs:3203-3310 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The spelling no longer uses a format width; `decimal(0, 65534)` and `decimal(0, 65535)` build, a larger scale is refused at check. Test a_decimal_constructor_lowers_with_its_written_scale_and_a_scale_past_65535_is_ill_typed_type_mismatch passes; mutation MAX_LITERAL_SCALE = 65_534 fails it. The bound itself is FND-007. |
| FND-002 | fixed | A decimal literal takes its typed peer's type with no expected type: `p + decimal(1, 2) = p`, `p * decimal(1, 2) = p`, `r + decimal(15, 1) = r`, `convert<P>(decimal(15, 1) + r)` check; a typed pair of `+`, `-`, `*` is typed with no expected type. Mutations: peer hint removed fails 7 decimal_source_forms tests; typed-sum made ambiguous fails sums_differences_and_products_of_one_typed_pair_need_no_expected_type. Division is FND-006. |
| FND-003 | fixed | qsl-cst now refuses `decimal(-0, 2)`, `decimal(-00, 2)`, `decimal(+1, 2)`, `decimal(01, 2)`, `decimal(1, 02)`, `decimal(1, -2)`, `decimal(1.5, 2)` as invalid_syntax/invalid-token at S1; test a_numeric_token_that_is_no_signed_int_or_uint_is_an_invalid_token passes; mutation disabling the negative-zero test fails it. |
| FND-004 | fixed | `decimal(1, 4294967296)` is refused invalid_syntax/invalid-token at S1 (asserted in the same test), naming the scale in the diagnostic message. |
| FND-005 | fixed | `-decimal(5, 0)` takes the expected type: `function f using v(): P pure { -decimal(5, 0) }`, `p + -decimal(5, 0) = p` and `-decimal(5, 0) = p` check; mutation removing the negated literal from `contextual` fails a_decimal_literal_takes_the_type_of_its_typed_peer. |
| FND-006 | fixed | A quotient is typed by its operands with no expected type: `let s = a / b in s = a` is refused undefined_expression/unproved-nonzero (never ambiguous-literal), as FR-104 and FR-104-AC-13 f, g, h now state. Mutation restoring the ambiguous-literal arm for a typed quotient fails a_quotient_is_typed_by_its_operands..., sums_differences_and_products... and x02_decimal_arithmetic_takes_its_target_and_retains_the_loss. |
| FND-007 | fixed | MAX_LITERAL_SCALE is replaced by s3.decimal_literal_scale (default 65535) and s3.decimal_literal_scale_total (default 1048576, counted per checked package), refused stage_limit_exceeded/work-budget-exceeded naming bound and actual; a type-less or wrong-typed position refuses first and the limits precede membership (`decimal(1, 65536)` at Z is a limit refusal, not a mismatch). 16 x decimal(0, 65535) check, the 17th refuses with actual 1114095; total 10 with 5, 5, 1 refuses at 11. Mutations: `>=` on the scale limit fails 2 tests, `>=` on the total fails the total test, a non-accumulating tally fails the total test. |

## Disposition verdict (pass 1)

FND-001 to FND-005 are fixed on head 530aee5c8. Two new defects (FND-006, FND-007) are routed back; see the Linear comments. Gates on head 530aee5c8, per-package, own target dir, QSPEC_DIR=quire-specification@b840f0985: fmt, cargo check --locked --workspace --all-targets, clippy -D warnings (default and --no-default-features), arch-lint qualified-core: exit 0. Tests: qsl-cst --test it 95 passed; qsl-forms 25+193+2; qsl-semantics lib 798, --test it 529; qsl-package 640 plus doctests; qsl-eval 183+1075+43: all 0 failed. qsl-replay: 19 failures, exactly the known set (lib 14, dispatch_link_compile 4, doctest 1). Mutations (all fail the named tests, then reverted): nonzero discharge forced true for a zero coefficient (only_a_nonzero_literal_divisor_discharges...); literal peer hint removed (7 tests); typed +,-,* made ambiguous again (sums_differences_and_products...); MAX_LITERAL_SCALE off by one (the 65535 lowering test); negated literal removed from `contextual`; negative-zero check disabled (qsl-cst decimal_constructor). Ruling check: FR-140-AC-7 does support typing +,-,* of one typed pair (decimal operands and result share one named type) and FR-104's text states it; the division carve-out contradicts FR-104 (FND-006); the negated literal rule agrees with FR-104 ("a unary `-` SHALL have its operand's decimal type").

## Disposition verdict (pass 2)

Head 0b062ccd5, trial against origin/main with QSPEC_DIR=quire-specification@860e08902 and own target dirs. FND-006 and FND-007 are fixed. FND-008 (low) is new and does not block. Gates exit 0: fmt; check --locked --workspace --all-targets; clippy -D warnings default and --no-default-features; arch-lint qualified-core and api-surface --qsl-only; per-package qsl-foundation, qsl-cst, qsl-forms, qsl-semantics, qsl-package, qsl-eval (0 failed); root `--test it` plain and `--all-features` (0 failed). qsl-replay: 20 failures, exactly the known set (lib 15 = the 14 known plus spine::lifecycle::tests::aggregate_package_accepts_temporal_selection_from_later_source, dispatch_link_compile 4, doctest 1). Mutations, each failing the named tests and reverted: nonzero discharge forced for a zero coefficient (2 tests); scale limit `>=` (2); total `>=` (1); typed quotient ambiguous again (3, including x02); non-accumulating tally (1); literal peer hint removed (6). Design judgments: (a) AssemblyLimits.decimal_literal mirrors the existing S3DecimalScale precedent: assembly limits reach `CallerLimits::set_bound` through assembly.set_bound (qsl-replay/src/spine.rs), the Setting variants are in the settings table and the spine stage map, and CheckingLimitKind carries the refusal; FR-255 lists them as CheckingLimits columns, a text drift the announced spec fix resolves. (b) The Tally is per-check state (zero for a new Scope, copied by Clone), is sequential and deterministic (no parallel check path exists), is Sync through an atomic, and counted each literal once in all seven positions probed; it does not violate TC-160's intent (no global or thread-local state) but the scan text should say a per-check counter is allowed (FND-008). (c) AC-57 Q case: FR-140 keeps every result at (v x 10^T, T) with T = smax = 2, as D26 and D27 do, so the evaluator's (-500, 2) follows FR-140 and the AC-57 value (-5, 0) does not; the fix belongs in the AC. The facts.rs interval arm removal is required by FR-146 ("A Decimal divisor satisfies the nonzero obligation only as a decimal(c,s) literal with c != 0") and FR-104; the restated x02, s6a and a_reused_env tests keep their loss, outcome and refusal assertions and cite that rule.
