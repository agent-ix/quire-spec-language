---
id: SR-60034
title: "spec-review of FR-290 reduction-not-preserving form spelling (quire-spec-language PR 1605)"
type: SpecReview
analysis: base
scope: "; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md; quire-semantic-value@0a365dbb spec/functional/FR-121-shared-property-and-fairness-values.md; quire-specification proposals/backend-provider-v1/schema.json"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: references
---
## Summary

Ticket: QSL-1117. The amended FR-290 row agrees with QSV FR-121 ("`Refinement { liveness }` SHALL encode as `{"refinement":{"liveness":<bool>}}`", AC-5) and with the reader's refusals: stray outer member, stray body member and wrong-kind liveness are refused as stated; empty string and other kinds are Malformed. No deferral language.

Gates. `make ci` (run once, flock-serialized): conformance, no-committed-binaries, ci-default-features, ci-all-features, test-deep-format, ci-clean-build and seam-probe pass; `string-edge` fails with 3 findings (qsl-semantics/src/check/terminal.rs:235, 236, 255, `TerminalNode::of` and `CheckedGraph::terminal_nodes`). The PR does not touch that file; origin/main (953397358) holds the identical lines, so main is red there, and `git log -S` attributes the code to e64d348e9 "QSL-983: emit checked terminal declarations as v2 terminal nodes (FR-411) (#1595)". Inherited, not fixed here (terra's lane). The remaining lanes run by hand with `make -k` (route-lint, checked-input, refinement-versioning, cargo-deny-bans, ci-docs, arch-lint-canonical-encoder, arch-lint-api-surface-qsl, arch-lint-qualified-core, build-no-std) all pass.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The QSpec provider contract disagrees: backend-provider-v1 schema.json types `reduction-not-preserving.form` as `Nonempty` (a string). FR-290 now says a provider's `form` may be the refinement object, which that schema rejects, and the row cites only QSV FR-121. The reader is merely more lenient, but the two specs state different contracts. Fix: amend QSpec FR-331 and the schema to allow the object spelling, or state in FR-290 that providers send only the four strings and the object is read for outcome-document symmetry. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:208, quire-specification proposals/backend-provider-v1/schema.json:813 | wrong-requirement |
| FND-002 | low | The row names Malformed and UnknownMember refusals but not the missing members the reader also refuses: `{}` is MissingMember at /cause/form/refinement and `{"refinement":{}}` is MissingMember at /cause/form/refinement/liveness (the latter is tested). Fix: add "a missing `refinement` or `liveness` is MissingMember" to the row. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:208 | wrong-requirement |

## Verdict

PASS with low findings.
