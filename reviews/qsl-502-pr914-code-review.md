---
id: SR-11758
title: "QSL-502 quire-spec-language PR 914 code review: call-target seam and union construction"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c763259750f0863aa35df16792044fbcd6276ac9; qsl-semantics/src/check/{call_target,sum_case,check/typing,claims,ir,lowering,mod,refusal}.rs, qsl-semantics/src/value/enumeration.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-replay/src/spine/dependency_tests.rs, qsl-semantics/src/check/sum_case/tests.rs"
review_set: subset
---

## Summary

Ticket: QSL-502 (remainder). Code review with the rust-review lane, scoped
to `git diff origin/main...HEAD`. Read in place. No build was run (brief: no
builds). The coder's `make ci` was not re-run.

Checked:

- `resolve_target` counts union, enum and import candidates for `q::m`,
  refuses more than one as `AmbiguousQualified`, resolves a lone union
  candidate to `UnionMember`, and leaves unqualified names and enum-only
  names on `Value`'s existing path. The `Call` and `Name` arms each call it
  once.
- `UnionMember::admit_shape` refuses all four FR-143 shape errors at the
  construction. Payloads are checked against their position types through
  the existing application frame.
- `NodeKind::UnionValue` is added to every exhaustive match (children,
  claims, lowering, eval). Lowering refuses it as unlowered, so no unkeyed
  node leaves check.
- The only new `NodeKey` mint in production code is
  `mint_union_member_key` in `check::sum_case`. No production caller uses
  it yet (union sealing in `check()` is QSL-504).

Not reported (known, recorded by the lead): FR-318-AC-5, union sealing in
`check()` (QSL-504), FR-319 application-node keys and FR-323/324 (503/504).

## Verdict

PASS for this method. No high finding. The two medium findings are recorded,
not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `mint_union_variant_id` is a public function outside `check` that recomputes the FR-441 digest. FR-319 says the `VariantId` is the check-minted member key's bytes retyped with no fresh computation, and no other stage re-derives it. Its doc comment says "retyped with no fresh computation", but the body calls `union_member_digest` again. `mint_union_member_key` is also re-exported `pub` from `check`, so qsl-replay can mint member keys itself, as it already does for enums with `mint_variant_id`. Take a `NodeKey` and use `VariantId::from_digest(*key.as_bytes())`, and keep both functions `pub(crate)` until a real caller needs them. | qsl-semantics/src/value/enumeration.rs:412-431, qsl-semantics/src/check/sum_case.rs:154-159, qsl-semantics/src/check/mod.rs:118 |
| FND-002 | medium | FR-317 says a construction shape refusal names the expected and actual shape. `admit_shape` and the undeclared-member branch return a bare `IllTyped(TypeMismatch)` with no member, arity or payload. A user who writes `Shape::Rect(1)` gets a type mismatch with no "expected 2 arguments". Either carry the shape in the refusal or drop the clause from FR-317. | qsl-semantics/src/check/sum_case.rs:192-207, qsl-semantics/src/check/call_target.rs:112-117 |
