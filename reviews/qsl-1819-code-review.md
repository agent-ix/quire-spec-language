---
id: SR-20261108210000000002
title: "Code and Rust review of authored-clause request routing"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1819; PR 2603; qsl-analyze/src/{analyze.rs,provider.rs}, qsl-analyze/tests/it/{clause_item.rs,clause_route.rs,main.rs}"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quire-spec-language/FR-281", type: references}
  - {target: "ix://agent-ix/quire-spec-language/FR-280", type: references}
---

## Summary

Independent final-candidate review of PR 2603 at 8a10a70a160e096e9a56a0a133d39577c7fafda5. Rust API, ownership, routing, cancellation, admission fallbacks, settlement, test oracles and the six inherited review fixes were examined. No actionable defect was found in this diff.

## Verdict

PASS for the scoped change. Request-owned package and binding build the EN-1 work; an unsupported backend never executes that work. Provider fallback routes preserve each item's request index without cloning the whole request a second time. New matches over RoutedWork are exhaustive; no unsafe operation, new panic path, duplicate semantic implementation, file-tracking system or weakened CI gate was introduced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

No applicable AssuranceProfile was found under spec/. Rust review used the installed API, implementation, safety, boundary and testing guidance; no Tokio changes apply. The following units were examined, with remaining clauses recorded as context rather than complete new coverage:

- FR-281-AC-2, examined: "Over a finite model whose invariant one transition breaks, an EN-1 item claiming that invariant settles `refuted`, the record carries the counterexample, and replaying that counterexample through `replay` reproduces it."
- FR-280-AC-2, examined: "For each TC-762 item, a `RoutedItem` routing that item to its QSL engine, passed to `qsl_analyze::provider::run` with the `CheckedPackage`, `SubjectBinding` and `AnalyzeLimits` of the same run, returns a terminal record equal to the one `analyze` returns, over `Engines::qsl()`, for that item settled `Supported` on the same backend."
- FR-281, examined: authored-clause negotiated-member paragraph, including request-owned package/binding, EN-1-only routing, and absent-clause diagnostic.
- FR-281-AC-11, examined: the EN-1 unbounded-root refusal and existing refusal-table test; the route test covers the code/cause while the internal record test covers the payload.
- FR-280-AC-6, context_only: reversed item ordering and refused/incomplete binding fallbacks, retained by the shared generic path.

The refuted route test is intentionally partial evidence for AC-2: clause_item.rs tests counterexample possession and production replay; the public terminal record keeps no counterexample. The supplementary EN-1 state-budget test does not claim EN-5-specific AC-6. Matrix call sites on the separate unmerged branch were not built. Empty generic request vectors need a type annotation, as disclosed in the PR.

Verified recorded checks: workspace all-targets check exit 0, library 141 passed, integration 1387 passed, with exit=0 in /tmp/qsl1819b-{workspace,lib,it}-permissions-fixed.log. Prior format, clippy and FR-281 validation are reported by the coder. No aggregate gate was run by this reviewer.

The actual codex exec review completed with no actionable findings (/tmp/qsl1819b-codex-review.txt). Its subprocess unexpectedly attempted bare cargo formatting/testing without the required thermal wrapper; that attempted test is excluded from evidence. No such process remained afterward. Only the separately verified thermal-lock logs support execution claims.
