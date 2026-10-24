---
id: SR-8709
title: "Code review of quire-spec-language PR #806 (QSL-742): signature type refusals keep their node and member path"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@17468cbaacb0946f6bb50a72e20f402a81e6bd0d; qsl-semantics/src/model/normalize.rs, qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/refusal.rs, qsl-semantics/tests/it/business_meanings.rs, qsl-semantics/tests/it/variant_type.rs"
review_set: subset
---
# Code review of quire-spec-language PR #806

## Summary

Ticket: QSL-742. Method: code-review with its rust-review lane and a test-oracle check. PR #806 (base `impl/business-model-meanings` at ffaefa335) makes `signature_type_refusal` keep the operation node and a typed `MemberPath`. A forbidden meaning now refuses as `IntakeMalformedDeclaration { node, member }`. An undeclared type refuses as `UnknownValueType`, which gains a `member` field. The coder wrote the tests after the code, so this review judged by reading whether each test would fail on the old code. Reviewer-only: no build or test run.

Examined:
- `signature_type_refusal`, normalize.rs:1864-1935 (examined)
- the two call sites in `check_node`, normalize.rs:2271-2305 (examined)
- `refusal_node` and every caller, intake.rs:4250-4460 and normalize.rs:2623 (examined)
- `ModelRefusalCause::UnknownValueType` and its fixture, refusal.rs:382-395 and 1504-1509 (examined)
- every `UnknownValueType` constructor, pattern and consumer in the workspace and sibling checkouts (examined)
- `an_operation_signature_naming_a_variant_type_refuses_as_malformed` and `assert_malformed`, variant_type.rs:244-255 and 623-647 (examined)
- `operation_signature_type_meaning_matrix`, business_meanings.rs:627-737 (examined)
- the other normalize-stage `IntakeMalformedDeclaration` sites (normalize.rs:1116, 1168, 1190, 1300, 1692, 1780, 1812, 2021, 2373, 2390) and `DomainPackage::original` (domain_package.rs:893-897) (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Both tests use a single parameter, so only `params[0]` is ever asserted. If the code always wrote `.index(0)`, or used a 1-based position, the variant test would still pass for index 0 only by accident and the matrix test would still pass. Nothing proves that `enumerate()`'s position reaches the path. The ticket's own AC 2 asks for a variant-typed parameter at index 0 **and** index 1. Fix: add a second parameter (for example a native one at position 0 and `Tier` at position 1, or `Tier` at both) and assert `Operation.params[1].typeRef`. | qsl-semantics/tests/it/variant_type.rs:629-647; qsl-semantics/tests/it/business_meanings.rs:660-668; qsl-semantics/src/model/normalize.rs:2274-2283 |
| FND-002 | medium | The new refusal sets `artifact: None, span: None`, but the operation's FCD origin is available. Intake records it in `DomainPackage::originals` for every operation member (intake.rs:4397-4430), and `DomainPackage::original(&operation)` returns it. The PR body says "the operation record carries no origin". That is true of `OperationMemberRecord`, but not of the package the function already receives. `IntakeMalformedDeclaration`'s own doc (refusal.rs:735-747) says `artifact`/`span` are `None` only when the origin is generated, absent or malformed. QSpec FR-154 and QSpec native-diagnostics say `malformed-declaration` keeps the node's FCD `origin`. Every other normalize-stage `IntakeMalformedDeclaration` site has the same gap, including the `Field.typeRef` one this PR copies, so this is a systemic gap, not one this PR introduced. Fix: fill `artifact`/`span` from `package.original(operation)` in one helper used by every normalize-stage site, or defer to a new ticket covering all ten sites. | qsl-semantics/src/model/normalize.rs:1906-1912; qsl-semantics/src/model/refusal.rs:735-747; qsl-semantics/src/model/domain_package.rs:893-897 |
| FND-003 | low | The new `refusal_node` arm for `UnknownValueType` is never reached. `refusal_node` is only applied to intake refusals (the semantic-IR and reader refusals in `read_domain_package`, and the `intake_refusals` sort in `build`). `UnknownValueType` is only raised later, by `validate_references`. No test covers the arm. The PR body presents it as how the dangling case keeps its node, but the node is actually kept by the `operation` field. Fix: delete the arm, or route `validate_references` refusals through `refusal_node` where the ordering needs it, and test that. | qsl-semantics/src/model/intake.rs:4460; qsl-semantics/src/model/normalize.rs:2623 |

## Verdict

The mechanism is correct:
- The parameter index comes from `op.parameters.iter().enumerate()`. The reader builds `parameters` from the IR `params` array in array order (intake.rs:2590-2598), so `i` is the IR's zero-based index.
- The spellings `Operation.params[i].typeRef` and `Operation.returns.typeRef` match the IR member names (`params`, `returns.typeRef`) and QSpec PR #242 (QSpec FR-154-AC-11, TC-235 M29). Parameter-then-result order matches M29.
- Both causes of `signature_type_refusal` (forbidden meaning, undeclared type) and both callers (parameter, result) now carry the operation node and the member path.
- `UnknownValueType`'s new field is handled everywhere. The only constructor is `signature_type_refusal`. The fixture (refusal.rs:1504), the unit-test pattern (normalize.rs:4645, `..`) and the catalog/tag matches (`{ .. }`) are updated or field-agnostic. No crate outside `qsl-semantics` names the variant, and the only `src/` consumer (`src/command/output.rs`) does not match on it.
- No new string comparison in production code. The `#[string_edge]` scan has nothing to cover.
- No panic surface, no integer conversion, no `unsafe`.

Test oracles: on the old code, the variant test fails `assert_malformed`'s `IntakeMalformedDeclaration` match (old cause was the unit `MalformedDeclaration`). For the forbidden meanings, the matrix test panics with "unexpected cause". The new `Missing` row would not compile against the old `UnknownValueType` (no `member` field). Each test would therefore have failed on the old code. The weak point is the index (FND-001).

**Mergeable once FND-001 is fixed.** FND-002 can be deferred to its own ticket, since it is systemic. FND-003 is a cleanup.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The undeclared-type branch of `signature_type_refusal` repeats `package.original(operation).map_or((None, None), \|original\| origin_locus(&original.origin))` rather than sharing it with `malformed_at`. Two copies of the origin lookup can drift, and QSL-744 will add more sites. Fix: one `fn origin_of(package, key) -> (Option<String>, Option<LocatedSpan>)` used by `malformed_at` and the `SignatureSite` construction. | qsl-semantics/src/model/normalize.rs:1868-1882, 1931-1946 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | fb56a8756: `a_later_parameter_naming_a_variant_type_keeps_its_own_position` declares Integer, Tier, Integer and asserts `Operation.params[1].typeRef`; a constant `.index(0)` or 1-based index now fails |
| FND-002 | fixed | fb56a8756: `malformed_at` and the `SignatureSite` fill artifact/span from `DomainPackage::original(operation).origin` via `intake::origin_locus`; the variant test asserts artifact `spec` and span 1:1. The other ten normalize-stage sites are deferred to QSL-744 (exists, Specced) |
| FND-003 | fixed | fb56a8756: the unreachable `UnknownValueType` arm is removed from `refusal_node` |
