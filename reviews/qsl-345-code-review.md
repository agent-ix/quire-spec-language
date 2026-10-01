---
id: SR-915
title: "QSL-345 code review (with rust-review lane) of PR 545, items 1 and 2"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@17895e2ca8666075a7b8d9ec9884fcc1e089f48a; qsl-replay/src/lib.rs (bound re-exports); qsl-replay/src/identity.rs (DeclaredDomain::new doctest); qsl-replay/src/call_site.rs (CallSite.package, AC-14 and AC-15 tests); qsl-package/src/checked.rs (EmittedPackage, unchanged, context); qsl-foundation/src/bound.rs (unchanged, context); spec/functional/FR-060-check-qsl-api-surface-boundary.md (T-12 rules, unchanged, context). Item 4 (FieldName, FieldSite, CheckedGraph::field_domain, model_node_content, qsl-replay/src/spine/clause/tests/call_site.rs) excluded: it leaves this PR by coordinator ruling."
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
---
## Summary

Ticket: QSL-345 (PR 1 of 2). PR: quire-spec-language#545 at 17895e2c.
Reviewed items 1 (bound and kernel integer re-exports) and 2 (`CallSite.package`).
Item 4 is being removed from this PR and was not reviewed.

Item 1, forgery check. T-12 governs four constructors (FR-060 T12-B..T12-E):
`NodeKey::from_digest`, `EffectiveId::from_digest`, `PopulationId::from_digest`
and `attest_ir_admitted_v2`. None of the eight re-exported types reaches any of them.
`DomainKey::new` takes a `WireNodeId`, which the facade already re-exported
together with `WireNodeId::from_digest` before this PR. `DeclaredDomain::new` was
already public. `ProofBound` has public fields. `FiniteBound`, `IntegerInterval`
and `Integer` are plain value types whose constructors only refuse empty ranges.
No re-exported type returns a `NodeKey`, `EffectiveId` or `PopulationId`.
So the re-exports let CG build nothing it could not already spell. They only remove
CG's need to depend on qsl-foundation and quire-exact directly.

Item 2. `call_site` returns `compiled.emitted.bytes().to_vec()` from the same
`EmittedPackage` whose `package_id()` it records. `EmittedPackage` stores `bytes`
and `package_id` together in its sole constructor (qsl-package/src/checked.rs:478-518).
So the bytes come from the same emission and are copied, not re-serialized.

Gate: coder's make ci log on 17895e2c reports exit=0. Both new tests and the
`DeclaredDomain::new` doctest appear as ok in it. Not re-run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The AC-15 test `a_declared_domain_is_built_through_the_facade_alone` reaches the re-exports as `crate::FiniteBound` and similar paths. Inside the crate those paths also resolve a private root `use`, so the test would still pass if the re-exports were not `pub`. The external-path oracle is the `DeclaredDomain::new` doctest (qsl_replay::...). It names only DeclaredDomain, DomainKey, FiniteBound, Integer, ProofBound, WireNodeId and EmptyFiniteBound. `FiniteBoundKind`, `IntegerInterval` and `EmptyInterval` have no test that would fail if they lost their `pub`. Fix: name all eight in the doctest, or in an integration test under qsl-replay/tests/, and tag that test FR-121-AC-15. | qsl-replay/src/call_site.rs:711-751; qsl-replay/src/identity.rs:297-311; qsl-replay/src/lib.rs:98-101 |

## Verdict

Items 1 and 2 are correct. The re-exports forge nothing that T-12 seals, and the
package bytes are the same emission's own bytes. One low test-oracle finding. The
PR can merge once item 4 is removed; FND-001 can be fixed in that same edit.

The split is clean. Items 1 and 2 use no item-4 code. Item 4 touches these places:
the `FieldName`/`FieldSite` names in the `pub use call_site::{..}` line of lib.rs,
and the module doc of call_site.rs, which mentions a field's domain key next to the
package bytes.
