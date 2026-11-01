---
id: SR-202610310151
title: "Code and Rust review of QSL-1443 PR #2040"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b202293619b944fee512bcc6384f2e61ec4667ad; PR #2040 changed qsl-package/src/checked_v2/tests.rs, qsl-package/src/checked_v2/timed_model.rs, qsl-package/src/emit.rs, qsl-package/src/emit/timed_order.rs; QSL-1443 and QSpec FR-322 timed-clause binder vectors CL-57..62"
review_set: subset
---
## Summary

Ticket: QSL-1443. The Rust implementation projects and emits guard and invariant binders, and the two CL vector consumers replay the intended QSpec #564 vectors without skipping the binder cases. The main review gap is that neither admitted case asserts that the binder references survive the typed projection and typed writer. A regression that dropped them in either layer could pass the new `Read::Verified` assertion. This review is not mergeable until that end-to-end evidence is added.

Rust gates: `cargo fmt --all -- --check`, `cargo check --locked -p qsl-package --no-default-features`, and `git diff --check origin/main...HEAD` passed. The focused timed clause vector consumers pass with QSpec #564 plus IR #785. `cargo deny check` fails on both clean base and PR because existing AGPL-3.0-or-later dependencies are rejected by the repository's license policy; this is a clean-base baseline. Current QSpec main's two consumers fail identically on clean base and PR at CL-01 with `UnknownMember /temporal_bindings`, blocked by IR-1230. This finding is excluded from the PR delta. No CI workflow changes were present. No applicable AssuranceProfile was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The admitted CL-57/CL-59 cases do not prove typed binder preservation through projection and writing. | qsl-package/src/checked_v2/tests.rs:2608-2610; qsl-package/src/checked_v2/timed_model.rs:1390-1440 |

## Verdict

Not mergeable: FND-001 is a medium evidence gap in the added admitted binder behavior. In `conformance_timed_clause_vectors_keep_their_published_outcomes`, the admitted branch asserts only `Read::Verified`; the pre-existing `clause_intake_is_total_and_round_trips_the_clause_fixture` round-trips an unquantified fixture. The implementation adds binder fields to the typed structs, reads them in `project_guard`/`project_invariant`, and writes them in `write_guard`/`write_invariant`, but the added conformance path never exercises those typed conversions on CL-57/CL-59.

The test should apply CL-57 and CL-59 to the positive fixture, project the admitted nodes with `CheckedTimedModel::from_admitted` (or the equivalent admitted-node API), assert the expected ordered `CheckedNodeId` binders on the guard/invariant, call `write_nodes`, assert the ordered `binders` references are emitted, and round-trip the typed model or written nodes to equality. This assertion must be non-vacuous by checking the expected non-empty binder list and its exact order.

## Finding details

### FND-001 — medium

| Field | Value |
| --- | --- |
| AC | QSpec FR-322 clause-node acceptance for admitted quantified guard/invariant binder vectors CL-57 and CL-59; QSL-1443 requires both consumers to replay these admitted cases. |
| Failure scenario | A regression in `project_guard`, `project_invariant`, `write_guard`, or `write_invariant` drops or reorders a non-empty binder list while leaving the I2 read outcome admitted. The new outcome assertion still passes, and the existing typed round-trip fixture contains no binders. |
| Evidence | `qsl-package/src/checked_v2/tests.rs:2608-2610` only matches `Read::Verified`; `qsl-package/src/checked_v2/timed_model.rs:1390-1440` round-trips the unquantified positive fixture. |
| Required assertion | For CL-57 and CL-59, assert exact ordered binder `CheckedNodeId`s after typed projection, exact ordered `binders` references after `write_nodes`, and round-trip equality. |

## Scope examined

- QSL-1443 task outcome and blocked-by IR-1230 context.
- QSpec #564 timed-clause vector consumers in `checked_v2/tests.rs` and `emit/timed_order.rs`.
- Binder projection and writer in `checked_v2/timed_model.rs`.
- Public Rust API and call sites for the added binder fields.
- Repo conventions and applicable review gates.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit d8dab338f7a6d69896036e573236ddb831912883 asserts the exact nonempty ordered binder IDs after `from_admitted`, asserts matching ordered binder references after `write_nodes`, and compares the `from_nodes` typed round-trip for CL-57 and CL-59. Both focused vector consumers pass with QSpec #564 and IR #785. |

After excerpt:

```rust
assert_eq!(
    projected_binders, expected_binders,
    "{id} projected binder order"
);
assert_eq!(
    binder_groups(&written_clause.body),
    vec![expected_binders.clone()],
    "{id} writer preserves the exact ordered binders"
);
assert_eq!(
    round_tripped,
    vec![(*model).clone()],
    "{id} typed model round-trips"
);
```
