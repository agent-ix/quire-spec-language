---
id: SR-13931
title: 'Spec review of PR #1185: EN-4 manifest owner, mode and evidence kind (STD-251), EARS analysis'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@7f855b716; git diff origin/main...7f855b716 (PR #1185, branch std-qsl570-251): spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md; ticket STD-251'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-280
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-189
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
---

## Summary

Ticket: STD-251. PR: quire-spec-language#1185, head 7f855b716. Method: spec-review with the EARS sub-analysis, applied to the changed requirement statements. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e. The quire grammar lint in `make check-spec-validation` reports no warning on FR-189, FR-192 or FR-280 at head.

The FR-189 and FR-192 statements are ubiquitous EARS ("The EN-4 provider manifest (FR-280) SHALL advertise ..."; "EN-4's provider manifest (FR-280) SHALL advertise ..."), each with one subject and one `SHALL`. The FR-280 bullet opens with a ubiquitous `shall` ("QSL shall publish one FR-331 provider manifest for each `analyze` engine") and then adds three declarative sentences without one.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-280 manifest bullet | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:56-63 | examined |
| FR-189 EN-4 manifest statement | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:214-215 | examined |
| FR-192 EN-4 manifest statement | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:80-83 | examined |
| FR-195 EN-5 manifest statements and AC-5 | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:28,122,143 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The owner rule ("Each manifest is data of the engine's own module in `qsl-analyze`; `qsl-route` holds the registry and FR-288's conversion and no engine's manifest") and the EN-4 content ("An engine that settles `probabilistic-satisfaction` items lists the evidence kinds it settles ..., and EN-4's manifest lists ...") are declarative sentences with no `shall`. They read as description attached to the bullet's one `shall`, not as requirements of their own, though FR-280-AC-3 tests the EN-4 content. Split them into their own `shall` statements, for example "Each engine's manifest shall be data of that engine's module in `qsl-analyze`" and "EN-4's manifest shall list ...". | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:58-63 |
| FND-002 | low | FR-280 now writes a manifest entry as a (kind, mode) pair with the evidence kind stated separately: "(`probabilistic-satisfaction`, `bounded`) with the evidence kind `statistical`". FR-195 (lines 28, 122) and FR-195-AC-5 still write EN-5's as "(`probabilistic-satisfaction`, `exact`)", which in FR-280's notation reads as a pair with mode `exact`, and ADR-012 §7.1 defines no such mode. The lead default leaves EN-5's mode unstated, but FR-280-AC-1 still needs a mode for EN-5's manifest (TC-762 step 1 runs an EN-5 item). Reword FR-195 to "(`probabilistic-satisfaction`, <mode>) with the evidence kind `exact`", or state that EN-5's mode is still open. | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:28,122,143 |

## Verdict

Mergeable on the EARS axis, with two lows. The FR-189 and FR-192 statements conform. The FR-280 additions hold normative content in sentences with no `shall` (FND-001). After this change EN-5's manifest statements in FR-195 use a notation that no longer matches FR-280's (FND-002). Neither is a wrong behaviour.

## Dispositions

Disposition pass 1 at head a13a84d08 (fix commit f78db444d). The quire grammar lint in `make check-spec-validation` reports no warning on FR-189, FR-192, FR-195 or FR-280.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | after (FR-280:63-71), three bullets, each with its own `shall`: "Each engine's manifest shall be data of that engine's own module in `qsl-analyze`, and `qsl-route` shall hold the registry and FR-288's conversion and no engine's manifest."; "An engine that settles `probabilistic-satisfaction` items shall list the evidence kinds it settles (FR-057, FR-075)."; "EN-4's manifest shall list (`probabilistic-satisfaction`, `bounded`) with the evidence kind `statistical` and no other, and with the `domains` `population` and `integer` ...". |
| FND-002 | fixed | after (FR-195:28-29): "EN-5's provider manifest SHALL advertise `probabilistic-satisfaction` with the evidence kind `exact` (its mode is not yet fixed)". (FR-195:122-123): "The EN-5 provider manifest SHALL advertise `probabilistic-satisfaction` with the evidence kind `exact` only (its mode is not yet fixed), and register through FR-075." (FR-195-AC-5): "EN-5's manifest lists `probabilistic-satisfaction` with the evidence kind `exact` only." The text no longer reads `exact` as a mode, and it states that the mode is open. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | TC-630, which verifies FR-195-AC-5, still writes step 5's expected result in the old pair notation: "Step 5: exactly (`probabilistic-satisfaction`, `exact`); ...". Under FR-280's (kind, mode) notation this reads as a pair with mode `exact`. That contradicts the reworded FR-195-AC-5 ("lists `probabilistic-satisfaction` with the evidence kind `exact` only") and FR-195's "its mode is not yet fixed". Reword it to "`probabilistic-satisfaction` with the evidence kind `exact` only". | spec/test-cases/TC-630-exact-only-forms-and-exact-routing.md:35 |
