---
id: SR-12038
title: "QSL-813 PR #919 code review: reaches_end admits finite multi-valued destinations"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@607a2285856cf5330fee3a2b5c91df459d39f9b3; Cargo.lock, qsl-semantics/src/check/state_model.rs, qsl-semantics/tests/it/relationship_reachability.rs, qsl-package/src/emit/tests/reachability.rs"
review_set: subset
---

## Summary

Ticket: QSL-813. PR quire-spec-language#919. Code review with the rust-review lane over the diff against origin/main. Read only; no build was run (the author reported `make ci` EXIT=0 before a final main merge).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. `reaches_end` checks operand conformance, then direction, then destination multiplicity (undeclared, unbounded, ordered), then destination type, in that order, so the code follows the order FR-365 states. Dropping the `upper != 1` clause is the only behaviour change: a finite multi-valued end of the near end's type is now admitted, and a foreign end still refuses `operator-ineligible`. Read without the `state_model.rs` change, C17 and the TC-1047 row 8 emit test would fail (`kids`/`litter` refused by `upper != 1`). C18 would pass without it, because the operand check already ran first; it is a regression test for the order AC-3 states, not a test of the new behaviour. The C10 rewrite keeps a multi-valued foreign case (`manyOwners`). The `Cargo.lock` bump moves the one package sourced from the quire-contract-ir repo, the crate `quire-contract-model`, to 64f884e7ce10ae3d87e8f30b18603fded8f2e76f, the merge commit of quire-contract-ir#422 (IR-824, reaches admits finite multi-valued destination ends). No new panics, unsafe or integer conversions in production code.
