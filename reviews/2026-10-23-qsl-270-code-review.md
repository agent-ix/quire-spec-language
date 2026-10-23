---
id: SR-16356
title: "Code review of QSL PR #1401: StaticAuthority sha256-jcs digests"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@cd12160fc37dae5bde056d92dbd9caeafb129d09; PR #1401 against main; Cargo.toml, Cargo.lock, src/state/{evaluation,input,mod}.rs, tests/it/{compiled_protocol_v2,composed_state_evaluation}.rs, tests/support/{mod,static_authority}.rs; FR-049-AC-10/11 and FR-064-AC-5"
review_set: subset
---
# Code review of QSL PR #1401

## Summary

Ticket: QSL-270. The Rust review examined the exact PR diff, the FR-049 digest and refusal contract, the FR-064 string-edge policy, and the affected state and xtask tests. The four vector digests are checked independently with RFC 8785 canonical JSON and SHA-256; the new refusal order and cause checks match FR-049-AC-11. No applicable AssuranceProfile was found.

Examined requirement units: FR-049-AC-10 (vector acceptance and JCS preimage), FR-049-AC-11 (wrong-domain and malformed refusal order), FR-064-AC-5 (real branch-gating sites cannot be allow-listed). FR-049's StaticAuthority digest rule and FR-064's string-edge behavior were context for those units. `tests/it/composed_state_evaluation.rs::static_authority_jcs_vectors_and_refusal_order` binds both FR-049 criteria and exercises the evaluator through the public API.

Focused checks at this head: `cargo test -p xtask real_adr010_sites_are_refused_as_allow_list_entries -- --nocapture` failed (1 test, exit 101); `cargo run --package xtask -- string-edge` failed (exit 1). Both used the required locked-build wrapper. No aggregate gate was run.

## Verdict

**FAIL.** The new branch-gating digest dispatch is unmarked, and the existing real-site regression test still names the API this PR removes. Both required checks fail on the PR head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The `sha256-jcs:` prefix decision is a new branch-gating string edge with no `#[ix_trace_rs::string_edge]` mark. `xtask string-edge` exits 1 and reports `static_authority_digest_refusal` at line 2483 as unmarked, so the required CI target cannot pass. Mark the typed boundary and verify the scanner accepts it. | src/state/evaluation.rs:2472-2486 |
| FND-002 | high | The FR-064 real-site regression still scans `CanonicalizationDomain::from_str` in `src/state/input.rs`, but this PR deletes that item. Its focused test fails at line 405 when the expected site is absent. Update the test's real-site entry to the new digest dispatch and preserve its allow-list rejection oracle. | xtask/src/string_edge.rs:361-405; src/state/input.rs:46-53 |
