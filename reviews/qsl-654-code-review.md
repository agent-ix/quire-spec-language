---
id: SR-1383
title: "Code review of quire-spec-language PR #664 (QSL-654): ProofBound domain kind and admit domains"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6bcd0259772fe70eabc0274ce916bc0ebc5e1aa3; PR #664 diff against merge base 1372b151 (origin/main bfeb258c); qsl-foundation/src/bound.rs, qsl-route/src/lib.rs, qsl-route/src/request.rs, qsl-route/tests/it/{routing,provider_origin,route_registry}.rs, qsl-semantics/src/family/requirements.rs, qsl-replay (identity, lib, witness, parity_identity, composite_parity, declared_domain_facade), tests/it/request_builder.rs, spec/decisions/ADR-013 C-28; against QSpec ada3f9eb FR-290 Advertised mode and AC-13, FR-331 extent and manifest domains"
review_set: subset
---
# Code review of quire-spec-language PR #664

## Summary

Ticket: QSL-654. The PR moves `DomainKind` to `qsl-foundation` with its seven
FR-331 wire spellings, adds `ProofBound.kind: Option<DomainKind>` which the
request writer fills, and makes `BackendDescriptor::admit` take and check the
manifest's `domains`, keeping them on the descriptor. The Rust lane
(`rust-review`) is folded into this file.

Examined:
- qsl-foundation/src/bound.rs `DomainKind`, `to_wire`, `from_wire`, `ProofBound.kind` (examined)
- qsl-route/src/request.rs `RequestWriter::bounded_item` (examined)
- qsl-route/src/lib.rs `BackendDescriptor::new`, `admit`, `domains`, `admit_domains`, `DomainsDefect`, `RegistrationCause::InvalidDomains` (examined)
- qsl-route/tests/it/routing.rs new domains tests (examined)
- qsl-replay ProofBound construction sites and `composite_domain::harness` (examined)
- qsl-semantics/src/family/requirements.rs re-export (examined)
- xtask/src/import_graph.rs `LAYER_PERMITTED_MODULES`, tools/arch-lint api-surface and qualified-core scope (context_only)
- QSpec FR-290 lines 148-164 and FR-290-AC-13; FR-331 lines 148-191 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `ProofBound.kind`'s pairing rule is documented but not enforced. The fields are `pub`, and `DeclaredDomain::new`, `harness_bounds` and replay's `harness` check accept `kind: None` on a cardinality, range or depth bound, or a kind that does not pair with the bound (`Some(Integer)` with `depth`). `RequestWriter` itself is sound, because it fills `kind` from the classification and refuses a mismatch first. Any other producer can still hand a CG writer a bound that FR-331 refuses ("a `ProofBound` without `kind` ... refuses before consumption"). | qsl-foundation/src/bound.rs:268-278; qsl-replay/src/identity.rs:302; qsl-replay/src/execute/composite_domain.rs:466-473 |
| FND-002 | medium | `BackendDescriptor::new` takes `domains` unchecked, so it builds descriptors that `admit` refuses: a `bounded` pair with `None`, `Some(empty)`, or `Some({Quantity})`. `domains()`'s doc then says something false ("None ... which only a registration advertising no `bounded` pair may do"). The test helpers already build such descriptors, for example provider_origin.rs `kani()` with `(OperationContract, Bounded)` and `None`. | qsl-route/src/lib.rs:227-241; qsl-route/src/lib.rs:319-324; qsl-route/tests/it/provider_origin.rs:17-24 |
| FND-003 | medium | The six new tests trace `#[trace("TC-271", "FR-290-AC-13")]`, but those are QSpec ids. In QSL's namespace, FR-290 is "Settle plugin results as typed terminal records", with AC-1 to AC-4 only, and spec/tests.md has no TC-271. `quire matrix` binds none of these tests: it has no FR-290-AC-13 row. | qsl-route/tests/it/routing.rs:316; qsl-route/tests/it/routing.rs:334; qsl-route/tests/it/routing.rs:343; qsl-route/tests/it/routing.rs:354; qsl-route/tests/it/routing.rs:371; qsl-foundation/src/bound.rs:352 |
| FND-004 | low | `tc_438_each_proof_bound_carries_the_kind_its_bound_variant_pairs_with` traces FR-097-AC-4, which covers the writer's refusal order. The test asserts what is emitted, not a refusal. FR-097-AC-3 is the nearest AC, although neither AC states `kind`. | qsl-route/src/request.rs:570 |
| FND-005 | low | After the move, the type is still reachable at its old path through `pub use qsl_foundation::bound::DomainKind` in qsl-semantics, and qsl-route imports it by both paths (lib.rs from `qsl_foundation::bound`, request.rs from `qsl_semantics::family`). That gives one type two public homes, so the old path works like a compatibility re-export. | qsl-semantics/src/family/requirements.rs:49; qsl-route/src/lib.rs:53; qsl-route/src/request.rs:48 |
| FND-006 | low | The admit doc says `domains` "is checked after every pair", but no test has both a pair defect and a `domains` defect. The absent-mode test passes valid `domains`, so if the order were reversed (an unknown mode with empty `domains` refusing `invalid-domains`), no test would fail. | qsl-route/src/lib.rs:270-276; qsl-route/tests/it/routing.rs:128-140 |

## Verdict

**Changes requested (medium).** The core behaviour matches QSpec FR-290 at ada3f9eb:

- **Registration paragraph.** A present but malformed `domains` (empty, holding a non-boundable or unknown kind, or repeating a kind) refuses `invalid_capability`/`invalid-domains` on every registration. An absent `domains` refuses only when a `bounded` pair is advertised.
- **Tests.** They cover every defect on both a bounded registration and an unbounded-only one. They are behavioural and assert the typed cause and the catalog code and cause.
- **Wire spellings.** `DomainKind::to_wire` matches FR-331's seven kinds exactly, and `from_wire` does not normalize.
- **Pairing.** `finite_kind` follows FR-331: cardinality for collection and population, integer-range for integer, depth for recursive.
- **Request writer.** `RequestWriter` never writes `None` and never writes a mispaired kind.
- **Layering.** The move respects layering. `qsl_foundation` is on `LAYER_PERMITTED_MODULES`, no Cargo edge changed, and the arch-lint api-surface and qualified-core rules do not cover `bound`.

FND-001 and FND-002 say the same thing about two types: the invariant lives in prose, and a public constructor can break it. FND-003 means none of the new tests count toward QSL's matrix.
