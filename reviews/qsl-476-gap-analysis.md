---
id: SR-1230
title: "QSL-476 gap analysis of PR #606 (FR-057, FR-075-AC-7)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e4eaf61883814d3a1befd7a82b7fb231bc95a53f; PR #606 diff against origin/main; spec/functional/FR-057-admit-shared-capability-kinds.md; spec/functional/FR-075-*.md; spec/functional/FR-288-build-the-registry-from-provider-manifests.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
---
## Summary

Ticket: QSL-476. PR: quire-spec-language#606. Manual check of each AC
against its tests.

- FR-075-AC-7 ("an equal descriptor (same identity and advertised pairs) …
  is one registration and is not refused"): TC-448
  `identical_repeat_registration_is_idempotent_and_not_refused`
  (qsl-route/src/lib.rs:674), and TC-447 at route_registry.rs:177, :654 and
  :686. Their doc comments now name id, manifest digest and advertised
  pairs, which matches the AC and the derived equality. Correct bindings.
- FR-057 (registration from identity and advertised labels alone): the type
  has no tool member, so it holds at compile time. TC-447
  `db_07_a_malformed_registration_never_reaches_the_registry` (FR-057-AC-8)
  still exercises `admit` with the two-argument form. Correct binding.
- The ticket's "Deletes in the same PR" list (`ToolIdentity`,
  `BackendDescriptor.tool`): both are gone from the tree.
- Spec text: ADR-012's runtime-availability row and §7.4 evidence agree with
  FR-057's tool-absence row. The probe checks presence only, and the result
  names backend, tool and claim. FR-288's note now records the descriptor as
  already tool-free, and spec.md's FR-288 row ("no tool identity in the
  descriptor") agrees.
- No production code lacks an owning requirement: the diff deletes code
  only.

## Verdict

Clean: every AC in scope is backed and nothing is underspecified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
