---
id: SR-20261104212207
title: "Code review of PR #2297: bounded FR-201 model graph adapter and FR-498 claim binder, code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@bb72749c3; git diff origin/main...bb72749c3 (PR #2297, branch peter/qsl-1498-en5-provider): qsl-replay/src/lib.rs, qsl-replay/src/probability_certificate.rs, qsl-replay/tests/it/main.rs, qsl-replay/tests/it/probability_certificate.rs; ticket QSL-1498"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-498
    type: reviews
date: '2026-11-04'
---

## Summary

Ticket: QSL-1498. PR: quire-spec-language#2297, head bb72749c3, a draft slice of FR-201. Method: code-review with the rust-review lane. The diff adds `enumerate_model_graph`, which builds a bounded acyclic graph from `qsl_eval::ModelSystem` with FR-196 v1 product keys, and two new public functions. `check_model_exact_reachability` runs the existing Bellman core over that graph. `check_claim_exact_reachability` binds an `ObligationIdentity` to the checked claim whose `occurrence().node()` equals it and then checks it. The module's `#![allow(dead_code)]` is removed.

Binding uses only `NodeKey` equality, never name, span or source order. Zero matches, several matches and a claim with no occurrence each return `ClaimNotBound`. A non-`Reach{Eventually}` form returns `UnsupportedClaimForm`. The bound and the AtLeast/AtMost side come from the bound claim. The objective does not; it is a caller parameter (FND-001).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| enumerate_model_graph | qsl-replay/src/probability_certificate.rs:148-245 | examined |
| product_key | qsl-replay/src/probability_certificate.rs:247-263 | examined |
| check_model_exact_reachability | qsl-replay/src/probability_certificate.rs:271-283 | examined |
| check_claim_exact_reachability | qsl-replay/src/probability_certificate.rs:290-326 | examined |
| ExactProbabilityGraphError new variants | qsl-replay/src/probability_certificate.rs:127-138 | examined |
| crate re-exports | qsl-replay/src/lib.rs:319-324 | examined |
| check_exact_reachability (existing core) | qsl-replay/src/probability_certificate.rs:359-460 | context_only |
| exact_checker_reconstructs_bounded_graph_from_checked_model_system | qsl-replay/tests/it/probability_certificate.rs:23-88 | examined |
| claim_binding_uses_the_checked_node_not_name_or_source_order | qsl-replay/tests/it/probability_certificate.rs:134-214 | examined |
| bounded_model_product_key_uses_the_complete_fr196_v1_tuple | qsl-replay/src/probability_certificate.rs:638-655 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `check_claim_exact_reachability` binds the claim but takes the scheduler `objective` from the caller. The claim fixes it: `ProbScheduler::Every` needs `Minimum` for `>=` and `Maximum` for `<=`, and a workload needs `Workload`. If a caller passes `Maximum` for an `under every scheduler ... >= θ` claim, the checker accepts a certificate for the best scheduler, so it accepts a false claim. Fix: derive the objective from `claim.scheduler` and the bound inside the binder, and remove the parameter. | qsl-replay/src/probability_certificate.rs:295 |
| FND-002 | medium | `check_model_exact_reachability` is `pub` and re-exported from the crate root. It accepts any predicate, bound and objective, so it skips the FR-498 claim binding and is a public half-wired entry point a consumer could misuse. Fix: make it `pub(crate)`, remove the re-export, and have the first integration test go through the binder or move it to a unit test. | qsl-replay/src/lib.rs:320; qsl-replay/src/probability_certificate.rs:271 |
| FND-003 | medium | The binder's error paths are only partly tested. No test reaches `UnsupportedClaimForm` (a non-`eventually` reach claim in the same fixture would) or the unidentified-claim branch. The zero-match case uses an empty slice (`&claims[..0]`) rather than a foreign node against the full claim list. Mutation M2, which deletes the several-matches guard at :307-309, passes both tests. | qsl-replay/src/probability_certificate.rs:304-316; qsl-replay/tests/it/probability_certificate.rs:202-212 |
| FND-004 | low | The negative case asserts only `!matches!(.., Accepted{..})`, so any `Err` (for example `UnsupportedClaimForm` or `LimitReached`) also passes. Assert the exact rejection rule and locus at the root key. | qsl-replay/tests/it/probability_certificate.rs:195-198 |
| FND-005 | low | A model with more than one initial state is reported as `UnsupportedDraw`. It is not a draw, so the cause misleads. Add a variant such as `UnsupportedInitial`, or document that `UnsupportedDraw` covers this case. | qsl-replay/src/probability_certificate.rs:155-157 |

## Verdict

Conditional. The slice is coherent apart from FND-001: the binder is a public API that can return a wrong acceptance, and fixing it is a few lines. After FND-001 and FND-002 are fixed, the slice can merge on its own. It is not FR-201 completion, and the PR body says so.

Evidence, run on head bb72749c3 with one target dir through locked-build.sh, -j4 and CARGO_PROFILE_DEV_DEBUG=0:

- `cargo test -p qsl-replay --test it -- probability_certificate`: 2 passed. The lib `probability_certificate` tests: 3 passed.
- `cargo clippy -p qsl-replay --all-targets -- -D warnings`, with default features and with `--all-features`: clean. `cargo fmt --check`: clean.
- Trial merge with origin/main 42ef33506: `cargo check --locked --workspace --all-targets` is clean, and the focused tests pass (2/2).
- The diff touches no `Cargo.lock`. It adds no compatibility layer and weakens no existing test. The removed `#![allow(dead_code)]` leaves clippy clean.

Mutations:

- M1 binds the first identified claim in source order, ignoring the node and the uniqueness guard. `claim_binding_uses_the_checked_node_not_name_or_source_order` fails at :195. The test is value-asserting and catches source-order or first-match selection.
- M2 deletes only the several-matches guard. It survives (FND-003). `CheckedProbabilisticClaim` is not `Clone` and FR-498 gives every claim a distinct node, so this guard is defensive and only reachable through a slice the caller assembles.

## Dispositions

- FND-001: fixed. The binder takes no objective; `claim_objective` derives Minimum (every, `>=`), Maximum (every, `<=`) or Workload from the claim. A unit test pins every combination and a mutation of the `<=` arm fails it; a false `<=` claim is rejected with `ThresholdMissed` at the root.
- FND-002: fixed. `check_model_exact_reachability` is `pub(crate)` with no re-export; every integration test goes through the binder.
- FND-003: fixed in part. `UnsupportedClaimForm` (an `always` claim) and a foreign node against the full claim list are tested. The claim-with-no-identity branch is not constructible from tests: `occurrence` is `pub(crate)` and the checker sets it on every checked claim. The several-matches guard stays defensive (equal nodes cannot occur under FR-498).
- FND-004: fixed. Negative cases assert the exact rule (`Malformed`, `ThresholdMissed`) and the locus.
- FND-005: fixed. A model without exactly one initial state returns the new `UnsupportedInitial`.
