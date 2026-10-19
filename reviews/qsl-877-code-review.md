---
id: SR-12169
title: "QSL-877 PR #1011 code review: manifest fairness_kinds and the requirements fairness set (FR-134)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b6c0cfdaa7e9155c8440f3d6723aa4fd4a246cc4; qsl-route/src/{lib,request}.rs, qsl-semantics/src/check/temporal_clause.rs, qsl-semantics/src/family/requirements.rs and the diff's tests (qsl-route/tests/it/{fairness_kinds,provider_origin,route_registry,routing}.rs, qsl-semantics/tests/it/temporal_fairness.rs, tests/it/spine_run.rs)"
review_set: subset
---

## Summary

Ticket: QSL-877. PR quire-spec-language#1011 at head b6c0cfdaa, diff `origin/main...HEAD` (10 files, +223/-19), branch off 2009fd339. Code review with the rust-review lane folded in. Read-only: nothing was built or run. QSpec is cited by id only. Only major bugs block.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An unknown `fairness_kinds` label refuses `invalid_capability`/`unknown-kind`, which gives that catalog cause a second meaning. QSpec FR-272's `invalid_capability` row says `unknown-kind` retains "the exact supplied kind or mode", meaning an FR-290 capability kind. QSL FR-057-AC-2 and the `RegistrationCause::UnknownKind` doc ("An advertised kind is not byte-equal to an FR-290 label") say the same. A consumer reading `unknown-kind` with bytes `Strong` or `fair` will conclude the backend advertised a capability kind with that spelling. QSpec already gives the parallel evidence-kind set its own causes (`absent-evidence-kind`, `unknown-evidence-kind`), so a separate fairness cause is the expected shape. The refusal itself is sound: nothing is accepted and the bytes are kept. The coder correctly did not invent a code and recorded the spec gap. To fix: route a QSpec ruling for a dedicated cause (e.g. `unknown-fairness-kind`). In this PR, update the `UnknownKind` variant doc so it no longer states a meaning the code now breaks. | qsl-route/src/lib.rs:208-215, qsl-route/src/lib.rs:434-436, qsl-route/src/lib.rs:172-176 |
| FND-002 | low | The manifest reading is looser than the sibling `domains` rules. A repeated label (`["weak","weak"]`) collapses silently into a set, while a repeated `domains` kind refuses `invalid-domains`. An absent member on a backend that advertises `temporal-satisfaction` reads as the empty set, although FR-331 says such a backend "carries its `fairness_kinds`", and the evidence-kind analogue has `absent-evidence-kind`. Neither case is unsound: absent/empty only narrows candidacy, and the empty-set report names the missing kind. The spec does not decide either case, so this belongs with the FND-001 ruling. | qsl-route/src/lib.rs:208-215 |

## Verdict

PASS (no major). FND-001 is a diagnostic-meaning defect waiting on a spec ruling. Its doc-comment half can be fixed in this PR. FND-002 is a spec-silent edge.

Checked and clean, with no finding:

- **Silent-filter hazard (#996).** `descriptor_from_manifest` now carries the manifest kinds into `BackendDescriptor::fairness_kinds`, so a manifest-registered backend advertising kinds is a candidate for items using them (`manifest_fairness_kinds_filter_candidates`). Present `["weak","strong"]` → both. `None` → empty. `Some(&[])` → empty, the same as `None`. Mixed case or padded spelling (`Strong`, `strong `, `""`, `fair`) refuses with the exact bytes, with no normalization (`fairness_kind_from_wire` matches bytes exactly).
- **Descriptor equality.** `fairness_kinds` is a derived `PartialEq` member of `BackendDescriptor`, so two registrations of one identity that differ only in kinds conflict through the existing FR-290-AC-10 path. A manifest that differs in kinds also differs in bytes, so its digest differs too. The existing `registrations_differing_in_fairness_kinds_conflict` covers the member.
- **Refusal order.** The fairness read runs after the pair and `domains` checks, so a doubly defective manifest keeps its earlier pinned cause.
- **Parameter shape.** `Option<&[&str]>` mirrors `domains: Option<&[&str]>`: the driver passes decoded manifest strings and QSL reads no bytes (ADR-013 C-28). `unwrap_or_default()` on `Option<&[&str]>` gives `&[]`. Every caller in the workspace is in the diff and passes `None`: 6 call sites in qsl-route tests, 1 in `tests/it/spine_run.rs`, plus the new helper. No production caller exists in this repo or in other checkouts under `~/dev`.
- **Requirements identity.** `Requirements::new` sets an empty fairness set. Every non-temporal producer (`claims.rs`, `state_clause.rs`, `state_graph_claim.rs`) is unchanged, so its record compares equal as before. `Requirements` is not serialized. `RequestWriter::item`/`bounded_item` read only `kind()`/`extent()`, and extent agreement reads only `extent()`. No existing record or identity changes.
- **Item path.** `items_from_requirements` is the only production candidate computation, and it goes through `candidates_of` → `candidates_for_fairness`. `Registry::candidates` is now the empty-set special case. A named backend lacking a kind gets an empty set with `missing_fairness_kinds`, never `UnknownBackend`.
- **Temporal claim.** `fairness.iter().map(FairnessConstraint::kind).collect()` reads the checked, resolved set the clause already holds, so weak and strong over one operation keep both kinds.
- **Layering (ADR-011 layer R, TC-390).** No new crate edge: qsl-route already depends on qsl-semantics and quire-semantic-value, and qsl-semantics already imports `quire_semantic_value::fairness`.
- **Rust idioms.** `#[must_use]` on the `with_fairness` builder. No new `unwrap`/`expect`/indexing on a production path. `BTreeSet` keeps the order deterministic. The `candidates_of` helper is private.
- **Traces.** The two new traced tests carry `TC-535` only and do not over-claim an AC. `manifest_fairness_kinds_filter_candidates` is TC-535 step 1 over manifest-built backends. `a_clauses_requirements_carry_its_fairness_kinds` checks the item-side input of the step 1 filter. The unknown-label test and the `candidates_of` unit test are untraced, which is correct: no AC states the refusal.

Interactions to watch (not findings of this PR):

- **#1010 (QSL-876)** adds a population-key argument to `TemporalClaim::requirements`. The conflict is in the `Ok(...)` body. The resolution must keep `.with_fairness(self.fairness.iter().copied())`, otherwise the hazard comes back silently: a strong item gets a weak-only candidate again.
- **#1008 (QSL-511)** lands temporal clause emission. Until it does, no temporal `RequirementRecord` reaches `items_from_requirements` (`not_emitted`). #1008 must build the record from `TemporalClaim::requirements`, not a fresh `Requirements::new`. Any #1008 fixture that registers a temporal backend through `descriptor_from_manifest(.., None)` now has no fairness kinds, so a fair item there gets an empty candidate set. Those fixtures need `Some(&["weak","strong"])`.

## Dispositions
