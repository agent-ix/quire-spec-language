---
id: SR-1303
title: "Spec review of PR #634 (QSL-351: ADR-011 layer 6, ADR-013 O-16/O-24, FR-121)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6725aa7a610aa33a18f3c2d76443018dd10259ff; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md"
review_set: subset
---

## Summary

Ticket: QSL-351. PR agent-ix/quire-spec-language#634. Base review of the
three spec edits.

- ADR-011 §6.1 layer 6 now lists `quire-contract-model` for `Std001Code`,
  matching layer 4's existing entry and the code.
- ADR-013 O-16 refusal row and O-24 Public type say `DeclineCode` is `Qsl`
  or `Std001`, a code is never remapped between the two registries, and the
  `Std001` arm records no issuer and refuses no unregistered code. They
  agree with each other, with FR-121 and with `DeclineCode`'s doc comment.
  The old "IR's arm lands when IR exports its typed code" text is gone from
  all three places.
- FR-121's statement says the same and keeps `from_call_site_refusal`
  producing `DeclineCode::Qsl` only, which matches the code.
- `quire validate --scope .` over the three files exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The three records state the ruling consistently and match the code.
