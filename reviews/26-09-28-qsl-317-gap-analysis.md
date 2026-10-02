---
id: SR-781
title: "QSL-317 gap analysis of PR 517 (qsl-replay call-site facade and request-type re-exports)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-replay/src/call_site.rs; qsl-replay/src/lib.rs; ticket QSL-317 and upstream IR-309 (acceptance text); quire-contract-codegen tests/it/skeleton_spine.rs (read, the consumer this replaces)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
## Summary

Ticket: QSL-317. PR: quire-spec-language#517. There is no plan
bundle and no FR for this ticket. The ticket body is the acceptance text, so
this analysis checks the diff against QSL-317's two work items and against
the consumer they exist for: CG's `tests/it/skeleton_spine.rs`
(`compile_native_twin` and `request`) in quire-contract-codegen.

| Ticket item | Delivered | Evidence |
| --- | --- | --- |
| 1. Re-export `Identifier`, `ScalarLimits`, `WireNodeId`, `SourceIdentity`, `DigestRecord`, `ByteDigest`, `DigestDomain` | Yes, all seven | `qsl-replay/src/lib.rs:67-69`. They cover every `qsl_foundation`/`quire_exact` import CG's test has (`skeleton_spine.rs:28-31,44`). |
| 2. Facade entry: package id and each function parameter's node id for a QSL source | Yes, per named function | `call_site` returns `package_id: DigestRecord` and `parameters: Vec<WireNodeId>`. Tests: `call_site_returns_the_package_id_and_the_named_functions_parameter_node_ids`, `call_site_refuses_an_unknown_function_name`. |

Scope discipline: CG's `compile_native_twin` calls `spine::compile` with an
empty model map, `DependencyInput::default()` and `SpineLimits::default()`.
`call_site` takes exactly the same fixed inputs, so leaving out dependency,
package and limits parameters matches the real need. It is neither
under- nor over-scoped on inputs. On outputs, see FND-001.

Expiry: once CG depends on a QSL with this change, CG can drop its
`qsl_replay::spine` import and both dev-dependencies. That depends on
FND-001: without names, CG still pairs parameters by position.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `CallSite.parameters` is a bare `Vec<WireNodeId>`. The consumer it replaces pairs each parameter's *name* with its node id (`compile_native_twin` zips `callable.parameters` names with the node keys into `(name, node_id)`, then builds `ReplayParameter { argument, node_id }`). With this entry, CG has to zip its own hard-coded name list against the vector by position. ADR-013 O-25 says the join between witness rows and parameters is "by declared identity, never by position". `callable.parameters` already holds each name. Return `Vec<(Identifier, WireNodeId)>`, or a small struct, so CG joins by name. | qsl-replay/src/call_site.rs:38,121-127 |
| FND-002 | low | The new public facade entry has no owning FR or AC, and both tests trace only the ticket id (`#[trace("QSL-317")]`). Those are the only `QSL-*` trace tags in the repo; every other test traces a TC and an FR-AC. ADR-011's crate-map row for `qsl-replay` (§6.2) does not mention the entry either. Add an AC for the call-site entry (FR-098 or FR-100 are the natural owners) and trace the tests to it, or record why a ticket-only trace is enough here. | qsl-replay/src/call_site.rs:154,193 |

## Verdict

Both ticket items are delivered, and the input scope matches CG's real
call. FND-001 is a real fit gap against the consumer and against ADR-013
O-25. Fix it before CG takes this change, or CG ends up joining by position.
FND-002 is traceability only.
