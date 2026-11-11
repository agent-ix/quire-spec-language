---
id: SR-2026111015000000000008
title: "gap analysis of QSL PR 2675 checked dispatch bridge TypeEnvironment"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2675 @ 1724d1035; qsl-semantics/src/check/checked_dispatch.rs, qsl-semantics/src/check/dispatch_link.rs, qsl-eval/tests/it/dispatch_calls.rs"
review_set: subset
---
# SR-2026111015000000000008: gap analysis of QSL PR 2675

## Summary

Ticket: AGE-1959. Plan completion: not assessed. This run is planless and scoped to the PR diff. The
ticket text was read as data and re-measured against HEAD.

The new test `bridge_environment_admits_a_subtype_argument_at_a_supertype_parameter` traces
QSpec-TC-196 and QSpec-FR-151. It asserts a check result, and a mutation that drops supertypes fails it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new refusal path has no test. `DispatchBridgeRefusal::TypeEnvironment` and its `link_causes` arm are never constructed or asserted. The sibling FR-302-AC-4 unit test covers only the empty-Ambiguous arm. The reachable inputs (an object type under a process supertype, and a raised-limit environment) are exactly the untested ones. | qsl-semantics/src/check/checked_dispatch.rs:217, qsl-semantics/src/check/dispatch_link.rs:639 |
| FND-002 | medium | The ticket asks (untrusted ticket text, re-measured) for a statement of the intended DomainPackage-TypeEnvironment equivalence. It asks which side is authoritative for dangling supertypes, cycles, deep chains, totality and cross-package supertypes. The PR adds no spec, ADR or doc text on any of these, and closes only item 2 (types left at Default). Re-measured: `MAX_CONFORMANCE_DEPTH` no longer exists in qsl-semantics/src, so the ticket's divergence table is partly stale. Merging this PR as the ticket's whole delivery would close the ticket with its design deliverable undone. Re-scope the ticket or keep it open. | qsl-semantics/src/check/checked_dispatch.rs:352-396 |

## Verdict

Two gaps. The code-change gaps are recorded in SR-2026111015000000000007 (FND-001 high). Neither
gap here blocks on its own, but FND-001 should land with that fix, as a test of the process-supertype
input.

## Dispositions

Renumbered in disposition pass 1 from SR-2026111015000000000003 to SR-2026111015000000000008: SR-2026111015000000000003 collided with QSL-1400's spec reviews (#2674, on main). The review-pass Linear comment carries SR-2026111015000000000003.

Disposition pass 1, against fix commit 662c24b80.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `DispatchBridgeRefusal::TypeEnvironment` and its `link_causes` arm are deleted, so no untested refusal path remains. The reachable input that motivated it (an object type under a process supertype) has a test, `bridge_links_an_object_type_specializing_a_process`. |
| FND-002 | deferred | The lead re-scopes AGE-1959 after merge. The intended-equivalence design statement is not this PR's deliverable. |
