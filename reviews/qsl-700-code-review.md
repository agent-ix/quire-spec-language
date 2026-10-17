---
id: SR-4955
title: "QSL-700 code review of PR #718: preserve requirement occurrence path conditions"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9304c4cf90423813450bc89a9a797ec14bf62440; PR #718 diff from acad2f9bf2f700194b3225ad6d9a67428526ea28: qsl-route/src/request.rs, tests/it/request_builder.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
---
# QSL-700 code review of PR #718

## Summary

Ticket: QSL-700. The change carries each `RequirementRecord`'s canonical
`PathGuard` sequence into `RequirementItem::path_condition()` and adds TC-984
coverage for same-node occurrences under opposite branch outcomes and an
unguarded record. I reviewed the code and Rust lanes, the surrounding
`RequirementItem` API, and the test oracle against FR-075-AC-9 and TC-984.

The full `make ci` log for this exact head records `CI_EXIT=0`; I read the
recorded log and did not run a build. `git diff --check` is clean. No assurance
profile specifically applies to this route-item change.

## Verdict

PASS. The implementation transfers the canonical guard values without
translation or deduplication, retains the source ordering and empty case, and
documents the public accessor. TC-984 exercises the checked-package request
builder and asserts the two distinct occurrences share one guard occurrence
while carrying opposite `holds` values. Its source-record comparison catches
per-item substitution, and the explicit true/false assertions catch a shared
wrong value. The unguarded fixture requires exactly one item and an empty
sequence. I found no code, Rust, integrity, or test-oracle defect in the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope

- `FR-075-AC-9` — examined: each same-node occurrence returns its own ordered canonical path-condition sequence; the then/else entries share the guard occurrence and differ in `holds`; unguarded records are empty.
- `TC-984` — examined: the required checked `if p then x + 1 else x + 1` and unguarded fixtures and expected per-record path conditions.
- `FR-075-AC-8` — context only: the adjacent request-builder contract already distinguishes same-node items by occurrence key and preserves result bounds.
- `qsl-route/src/request.rs` — examined: `RequirementItem` storage/accessor and `items_from_requirements` source mapping.
- `tests/it/request_builder.rs` — examined: TC-984 assertions and fail-closed fixture selection.

## Review methods

- `code-review` and Rust review: read the diff, crate conventions, public API
documentation, ownership, panic/unsafe surface, and test traceability/oracle.
- No runtime checks were run by this reviewer. The reported CI evidence is the
  exact-head log named above.
