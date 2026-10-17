---
id: SR-11764
title: "Failure-domain review of FR-367 (QSL-818, PR #915)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@35042dc696ecfa1a71a62ecbb58f10529b8fb0f4; spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/test-cases/TC-1053-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/spec.md"
review_set: subset
---
# Failure-domain review of FR-367

## Summary

Ticket: QSL-818. Looked for reachable inputs that FR-367's refusal set does not name. One: a `let` or quantifier binder operand.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSL lowers every `let`/quantifier binder to a `value`/`parameter` node at the next level (lowering.rs `bind`), so in `let x = a in x + 1` operand 0 passes `parameter_declaration` but joins no parameter of the called function. Behavior 4 assumes the join succeeds; no `BindRefusal` covers it, and the internal-failure clause only covers a name mismatch at a joined node and claims the package "cannot produce" the rest, which is false here. | FR-367 Behavior 4, FR-367 Outputs |

## Verdict

FIX. The other failure modes (missing/extra/duplicate/mistyped argument, wrong occurrence, nested application, unbounded range, out-of-i64 value) are each named.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 530ecc8c4 |
