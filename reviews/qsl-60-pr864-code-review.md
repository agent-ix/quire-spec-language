---
id: SR-11255
title: "Code review — QSL-60 qualified systems-name binder (PR 864)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@880285f273754140fe4d777a52c152d78b5b07cd; qsl-semantics/src/model/qualified.rs, qsl-semantics/src/model/refusal.rs, qsl-semantics/src/model/accounting.rs, qsl-semantics/src/model/admitted.rs, qsl-semantics/src/model/normalize.rs, qsl-semantics/src/model/systems.rs, qsl-semantics/tests/it/model_systems_resolve.rs, qsl-semantics/tests/it/model_systems.rs"
review_set: subset
---

## Summary

Ticket: QSL-60. Code review with the rust-review lane folded in, over the PR 864 diff against origin/main. Oracles: QSpec FR-152 (Systems-model roles, binder request, Navigation), FR-152-AC-1/3/5, TC-197 Y01/Y02/Y05/Y06, value-accounting `systems.resolve` row.

## Verdict

**Two blocking findings.** What is right: the alias is matched against the unit's aliases only and compared with `admitted.selection()` before any charge; segments resolve only through `AdmittedPackage::declaration` (the admitted package's own artifact-id map), never by display name or another package; `systems.resolve` is charged once per segment after the alias, before the segment is resolved, one work unit and no size counter (matches the value-accounting row); a segment past the artifact id is refused, so `M::Sys::pump_alloc` and `M::Sys::sys_pump` name nothing; the effective id comes from a visible `(owner, original)` entry in the supplied view with no fallback; the Interface resolves to its effective type. No unwrap/expect/panic in new non-test code (`split().next().unwrap_or_default()` cannot panic). `ChargePoint::ALL` (now 18) has no other consumer, catalogue or golden list in the repo; the only stale count is the ticket's own "15". The two new causes are mapped by name in `as_str`, `code`, `origin`-style arms, the fixtures sample list and the mirror tests; no wildcard swallows them, and the CLI/replay matchers only see admission refusals, which these causes cannot reach. The new test module is gated on `test-support` like `model_systems.rs`; `make ci`'s `ci-all-features` lane runs `xtask test-suites all-features-workspace --workspace --all-features`, and the root crate's dev-dependency turns `qsl-semantics/test-support` on for any workspace build, so the tests run in the gate.

Recorded, not blocking: the wrong-export refusal from `resolve_kind` is the unit cause `WrongExport` with required/actual kind only in `detail`, short of what FR-152's binder-request text names (pre-existing in systems.rs).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `resolve_systems` exposes a Connection or Allocation that its admission rule refuses. It calls `resolve_kind` only, and kind mapping gives every allocation `Kind::Allocation` and every connection with Port ends `Kind::Connection`; the connection conditions and the allocation rule are never consulted. So `resolve(Connection, M::pipe)` on the TC-197 Y05 or Y03(a) package (FR-152-AC-4 refusals) returns a resolved Connection, and `resolve(Allocation, M::pump_alloc)` with the target replaced by `pump_out` (Y02) returns a resolved Allocation. Fix: refuse (or require an admitted-connections/allocations input) when the construct failed `check_connection`/`check_allocation`, and test Y05 through the binder. | qsl-semantics/src/model/qualified.rs:140-141; qsl-semantics/src/model/systems.rs:384-391 |
| FND-002 | high | The owner of a Connection/Allocation is "the first element of `record_member_owners`", which is the source end-owner only when the source end has one. For an Allocation `record_member_owners` is `element_owner(source).chain(element_owner(target))`; when the source has no owner (a Port whose `owner` is not a Part, or a source that is not a Part, Port or operation, none of which kind mapping refuses for an allocation) the first element is the TARGET end-owner, and the binder silently returns the target-owned member (e.g. `(Sys, alloc)`) as if it were the source end-owner's. FR-152 and TC-197 Y01 require the source end-owner's member. Fix: compute the source end-owner explicitly (`element_owner` of the source end) and refuse when it is absent; add a test. | qsl-semantics/src/model/qualified.rs:165-190; qsl-semantics/src/model/normalize.rs:2571-2576 |

## Dispositions

Round 1, reviewed at agent-ix/quire-spec-language@49bb6fbdd37f2836e5b4936b39f547e32a838bf4.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 99faa8b21: `admit_construct` runs `check_connection` / `check_allocation` before the effective lookup; Y02, Y03(a) and Y05 now refuse through the binder (tests y02_*, y03a_*, y05_*). |
| FND-002 | fixed | 99faa8b21 (+41a08f513 for the refusal name): `effective_owner` computes the source end's owner on its own and refuses `wrong-export` when absent; test an_allocation_without_a_source_owner_never_takes_the_target_owner (source `Count`, view holds only the target `Sys` entry). |

Recorded, not blocking (round 1): the binder now charges the three `systems.connection-condition` (or one `systems.allocation`) on every resolve of a Connection/Allocation. No production path runs the admission rules today, so nothing is charged twice yet; once admission applies them per the value-accounting schedule, resolution should reuse that outcome instead of re-charging.
