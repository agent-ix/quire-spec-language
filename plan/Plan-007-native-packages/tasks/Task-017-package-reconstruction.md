---
id: Task-017
title: "Read packages through bounded real compiler reconstruction"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-081
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-082
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-083
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-084
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-085
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-086
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-087
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-088
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-091
    type: verifies
---
# Task-017: Read packages through bounded real compiler reconstruction

## Scope

Implement read_verified using exact byte selection, bounded Serde recognition,
closed typed decoding, explicit external authority and actual parse/link/check.
Compare every regenerated claim and preserve accepted original bytes, without
constructing a checked package from serialized status.

## Subtasks

- [x] Write successful real setup plus independently selected raw/adverse requests first; capture a genuine reader-API failure before implementation.
- [x] Implement admission/hash, generic duplicate-rejecting recognition, format selection and typed decoding in the reviewed order, with fresh per-pass counters.
- [x] Keep integer semantic decoding on original bytes; test finite numeric recognition, exact large u64 revisions, lexical fractions/exponents and all closed-record members.
- [x] Implement fixed semantic/domain/feature selection and source/authored-binding comparison. Preserve valid external binding permutations and refuse conflicting/foreign/missing bindings with specified stages.
- [x] Invoke actual native parse/link/check using independently lowered frontend limits and the complete offered model inventory. Preserve original diagnostics and only exposed usage.
- [x] Regenerate bounded manifest/canonical/artifact content and compare every member, including projection metadata excluded from canonical identity. Retain raw accepted bytes and expose only complete success.
- [x] Qualify each shape/selector/dependency/derived-field mutation with its recomputed byte selector where needed; a stale-digest or setup failure cannot count as a later semantic refusal.
- [x] Complete real Rust Draft 2020-12 schema and all reader/pass-limit/precedence/retry controls, with no network resolution or assumed passing maximum.
- [x] Complete the reader portions of shared producer cases, run local code/Rust review and relevant regressions, and reconcile only fully backed matrix criteria.

## Deliverables

The actual verified reader and its code/error/usage API, complete producer/
reader criterion mapping, precise refusal and incomplete observations, and
validated local code/Rust review with original failures and final revision.

## Notes

Serde owns grammar; private adapters own fields, budgets and typed conversion.
RawValue shortcuts must not bypass depth/duplicate accounting. Native source
and authored bindings come from the caller, not wire adoption. This task
does not add a general model decoder, B envelope or IR executable projection.
Source/semantic/interface changes reopen the completed specification reviews.

Qualification: implementation eb97a87; PR code/Rust review SR-112 and final
Plan-007 gap audit SR-113. Broader LC02/FS05 and LC04/05 acceptance remains open.
