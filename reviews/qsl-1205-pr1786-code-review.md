---
id: SR-1786001
title: "PR #1786 (QSL-1205) code review: derived deadlock-freedom and time-lock-freedom requests, QSpec STD-547/548"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/terra-qsl-integration...75bf53dcd (qsl-route request.rs, model_request.rs, deadlock.rs; qsl-analyze analyze/items.rs, model_check/request.rs, zone_check; qsl-replay certificate.rs, zone_certificate.rs; qsl-semantics; FR-244, FR-245, TC-699)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-243
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: reviews
---

# PR #1786 code review (Rust)

## Summary

Ticket: QSL-1205. Head 75bf53dcd, based on terra-qsl-integration. Checked against QSpec 63ff21f1: FR-368 "Item kinds", "Claim and anchor", "Extent", "Position", "Negotiation" and AC-13..22; FR-331 "Certificate member" and AC-46; FR-418.

**What holds.**
- **Item order.** `ModelRequest::derived` writes the caller's items first, in caller order. The derived items follow, grouped by first-occurring subject: `deadlock-freedom` then `time-lock-freedom`. A caller's index never moves.
  - A probe with two authored items over one subject, written twice, gives one formula item and one item of each derived kind.
- **Anchor.** Each derived item is anchored at the `model_root` of the one selection its subject reads, with that node's `generated` occurrence at ordinal 0. The deadlock-freedom and time-lock-freedom items share it.
  - A subject reading two selections gets no request item, and an `UnanchoredItem` carries `unsupported_projection`/`unsupported-requested-capability` (FR-368-AC-22).
- **Derived item fields.** Each derived item is `temporal-satisfaction`, unbounded with `finite_bound_available` false, with no hybrid flag and no record requirements. It has no `max_depth`; the EN-1 adapter clears it.
- **Limits.** `DerivedLimits` admits exactly the eight exhaustive-check limits. Each of the other four refuses `invalid_runtime_input`/`unknown-member`.
- **Refinement items.** A refinement item carries `abstract_subject` (`package_id` and `initial_states`) only on that kind.
- **Certificate.** `initials` replaces `initial`, the checker refuses an empty `initials` as `malformed` at `initials`, and the part's `refinement_node` is the refinement record's occurrence node on both the emitter side and the checker side. All of this matches FR-418 and the schema's `ZoneCertificate` and `RefinementPart` property lists.
- **Certificate members.** `CertificateMember` holds FR-331's 50 names in order. The test reads the schema in place from `$QSPEC_DIR`.

**Renames without a compatibility path.** These have no shim or alias left behind:
- `initial` became `initials`.
- `RefinementPart.refinement` became `refinement_node`.
- `ModelRequest::authored`, `refinement` and `author_refinement` gained arguments.
- `opts_out`, `is_timed` and `is_dense` were folded into `SubjectReading`.

A grep finds no stale reader in the code, and FR-244, FR-245 and TC-699 are updated. This is correct for prerelease code.

**No QSpec content is copied in.** The 50-name test reads `proposals/backend-provider-v1/schema.json` from `$QSPEC_DIR`. The enum and the `ItemKind` and `DerivedLimit` spellings are implementations of FR-331 and FR-368, and the schema test pins them against the source. No vector, schema or prose is vendored.

**Judgement of the coder's declared limits.**

| Limit the coder declared | Real gap? | Detail |
| --- | --- | --- |
| Six of the eight caller limits are carried but no engine reads them | Only `max_clocks` | `max_store_buffer`, `max_messages` and `max_live_instances` bound weak-memory and `on each` protocol exploration, and no model-subject derived item has either. `max_solver_steps` and `max_solver_time` bound SMT, and neither EN-1 nor EN-6 is SMT. `max_clocks` bounds EN-6 on exactly these timed subjects, and it is silently dropped (FND-002). |
| No protocol-subject derived items (FR-429, FR-438) | Yes, against FR-368, but outside QSL-1205's model-subject scope | It is not tracked by any ticket (FND-004). |
| No domain list on `RequestItem` | Not for these ACs | QSL is the in-process producer. It has no FR-331 request-wire writer, and authored items carry no extent domain list either. The domain half of FR-368-AC-16 applies once a wire writer exists, so the trace should not claim it now (FND-005). |

**Gates** at 75bf53dcd, run in one reused target with `--all-features`, debug info off, and `QSPEC_DIR` at qspec-63ff21f1:
- All exit 0: qsl-route, qsl-replay (909), qsl-foundation, qsl-analyze (it 1037 and lib), qsl-semantics lib (816), workspace clippy `-D warnings`, fmt and `make check-spec-validation` (no errors).
- conformance_c14 (580w lane) lives in qsl-package and was not run.

## Verdict

Changes requested (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **A `time tick` subject's derived items route to EN-1 too.** `ModelRequest::derived` restricts candidates with `restrict_to_dense_time(.., reading.dense())`, so these items keep every temporal backend. A probe over a `time tick` model gives `["en-1", "en-6"]` for both the deadlock-freedom and the time-lock-freedom item. FR-243 routes every non-probabilistic timed item, both derived items included, to EN-6. EN-1 cannot run a time-lock-freedom item: `ModelCheckItem` has no such item, and `derived_model_item` refuses it. Restrict on `reading.timed()`. | FR-243 Behavior, FR-232, qsl-route/src/model_request.rs `derived` |
| FND-002 | low | **A caller's `max_clocks` on a derived item is written but never applied.** `derived_zone_item` reads only `max_states` and `max_transitions`. The `TimedSystem`'s clock bound is fixed when the caller builds the subject (the test uses `DEFAULT_MAX_CLOCKS`), so the item's `max_clocks` setting has no effect. The adapter should refuse a `TimedItem` whose clock bound differs from the item's setting, or set it. This is the only one of the six unread limits that applies to these items. | QSpec FR-368 "Negotiation" ("`max_clocks` included for a timed subject"), qsl-analyze/src/analyze/items.rs `derived_zone_item` |
| FND-003 | low | **There are two derivations of the derived items, and they disagree.** `deadlock_freedom_items` and `time_lock_freedom_items` now have only test callers. They remain what the FR-124 and FR-232 AC tests exercise, while production uses `ModelRequest::derived`. They differ: `time_lock_freedom_items` takes no refinement items, but `derived` gives a timed refinement subject its time-lock-freedom item, as FR-232 ("per distinct timed subject among the request's items") requires. Delete the helpers, or make them the one derivation `derived` calls. | FR-124, FR-232, qsl-route/src/deadlock.rs |
| FND-004 | low | **No protocol subject gets a derived request item, and no ticket tracks it.** `ItemKind::RaceFreedom` is never written. `protocol_deadlock_freedom_items` and `race_freedom_items` still have no request consumer, the same gap QSL-1205 closes for model subjects. That leaves FR-368-AC-14's protocol-clause anchor and AC-17's race-freedom position unimplemented. File a ticket. | QSpec FR-368-AC-14, AC-17, FR-429, FR-438 |
| FND-005 | low | **Trace tags claim QSpec ACs whose other half QSL does not implement.** The traces name FR-368-AC-13 (the reader refusing `time-lock`, `race` and other wrong spellings), AC-14 (the protocol anchor), AC-16 (the `infinite-trace` domain list; the test asserts the empty `domains()`, which is `extent.bounds`) and AC-20 (refusing a derived item that carries `max_depth`). Name only the producer half each test pins. | qsl-route/src/request.rs tests, qsl-analyze/tests/it/derived_request.rs |
