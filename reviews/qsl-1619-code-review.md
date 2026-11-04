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
