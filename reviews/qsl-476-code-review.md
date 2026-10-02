---
id: SR-1229
title: "Code review of quire-spec-language PR #606: drop the pinned tool identity from backend descriptors"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e4eaf61883814d3a1befd7a82b7fb231bc95a53f; PR #606 diff against origin/main: qsl-route/src/lib.rs, qsl-route/tests/it/{route_registry,routing}.rs, qsl-replay/src/identity.rs, tests/it/{lowering_registry_isolation,request_builder}.rs, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md, spec/functional/FR-288-build-the-registry-from-provider-manifests.md"
review_set: subset
---
# Code review of quire-spec-language PR #606

## Summary

Ticket: QSL-476 (slice A4t of plan v2). The PR deletes `ToolIdentity`,
`BackendDescriptor.tool` and the `tool()` accessor, and removes the `tool`
parameter from `BackendDescriptor::new` and `::admit`. Every call site moves
to the shorter signature. The docs on `BackendDescriptor` and
`qsl_replay::identity::Backend` now say the manifest digest is the only
binding to the tool. The spec changes are ADR-012's runtime-availability row
and the §7.4 evidence bullet (no mismatched-pin run), ADR-029's follow-up
list, and FR-288's References note.

Checks:
- Absence: no `ToolIdentity`, `.tool()`, "pinned tool" or "tool pin" is left
  in the Rust tree or in normative spec text. What remains is in historical
  review records (spec/reviews/**, reviews/**), which record past findings
  and are not normative. NFR-002's "pinned toolchain" is the Rust toolchain
  and is unrelated.
- ADR-012 §7.1 already declared `BackendDescriptor { id, advertises }` with
  no tool. The code now matches it.
- Registry equality: `BackendDescriptor`'s derived `PartialEq` now compares
  the candidate (id plus manifest digest) and the advertised pairs, which is
  what FR-075-AC-7 states ("same identity and advertised pairs"). No test
  relied on two descriptors that differ only in tool: every helper derived
  the tool string from the id.
- Rust lane (rust-review): pure deletion. No new panic, `unsafe` or
  conversion. No compatibility shim, deprecated alias or re-export.
- Team-leader ruling applied: the manifest digest stays. Making `BackendId`
  identity-only belongs to the BackendId slice and is not flagged here.
- Value test: the deleted field was a pin carried "unread and
  uninterpreted" (its own doc). Removing it removes ceremony and adds none.
- Gate: `cargo test -p qsl-route` at this head passed (12 unit, 32
  integration). The coder reports `make ci` exit 0 on 8a9e7049. The later
  commit e4eaf618 touches only the two ADR files (`git diff --stat
  8a9e7049 HEAD`).

## Verdict

Approved, with no findings. Mergeable as is.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
