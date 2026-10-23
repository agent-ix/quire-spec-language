---
id: SR-16357
title: "Gap analysis of QSL PR #1401: StaticAuthority digest acceptance and refusal evidence"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@cd12160fc37dae5bde056d92dbd9caeafb129d09; FR-049-AC-10/11, FR-064-AC-5; tests/it/composed_state_evaluation.rs and xtask/src/string_edge.rs"
review_set: subset
---
# Gap analysis of QSL PR #1401

## Summary

Ticket: QSL-270. FR-049-AC-10 and FR-049-AC-11 are both tagged on `static_authority_jcs_vectors_and_refusal_order`. Its assertions exercise the public evaluator, all four published digest vectors, independent RFC 8785 preimages, wrong-domain and malformed causes, and two-member precedence. FR-064-AC-5 is also tagged on `real_adr010_sites_are_refused_as_allow_list_entries`, but the PR removes one of the production sites that test demands.

## Verdict

**FAIL.** The FR-064-AC-5 trace binding remains present but its test no longer passes on this PR head. The FR-049-AC-10/11 scoped bindings are meaningful; this review does not claim a full-repository matrix result.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-064-AC-5's trace-tagged real-site test fails because it still selects the deleted `CanonicalizationDomain::from_str` site. This leaves the affected branch-dispatch coverage red on the PR head. Update the real-site vector for the new digest decision and preserve the branch-gating assertion. | xtask/src/string_edge.rs:361-405; src/state/input.rs:46-53 |

## Coverage

Plan completion: not assessed. The computed `quoin matrix --repo . --json` and `quire matrix --scope . --format json` were attempted but did not return in this review window and were stopped; no full repository matrix status is claimed. Scoped criterion and test bindings above were checked directly from the spec, trace tags, and executable assertions. `cargo test -p xtask real_adr010_sites_are_refused_as_allow_list_entries -- --nocapture` returned exit 101 with one failed test. Optional semantic review was not invoked as a separate method; the specified FR-049 contract was inspected in the code-review lane.
