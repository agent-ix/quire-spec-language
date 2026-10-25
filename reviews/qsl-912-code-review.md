---
id: SR-17036
title: "Code review of QSL-912: refinement precedence and abstract relationships"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; FR-135-AC-7, FR-140-AC-15, FR-169-AC-11; qsl-semantics/src/check/{mod.rs,refinement.rs,refinement/protocol.rs}, qsl-semantics/tests/it/refinement_declarations.rs, qsl-replay/src/refinement/map.rs, qsl-replay/src/spine/refinement.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
---
# Code review of QSL-912

## Summary

The change adds FR-135's abstract-relationship refusal and carries the canonical
population node key into the checked refinement so FR-140 maps rows in digest
order. The requested `BadState` case and removal of `EvalFailure::Stopped` are
already present on the base commit; both existing FR-169-AC-11 tests were run.

The changed code has no correctness finding from the scoped review. Focused
tests, formatting and clippy pass. Full verification remains conditional on a
hosted rerun because the local full-suite run encountered a sandbox-only
measurement failure and the existing dependency-license gate fails.

## Verdict

**CONDITIONAL** — scoped gates pass, but full crate verification and the
repository matrix could not be established on this host.

## Coverage

- FR-135-AC-7: the integration test reaches refinement checking with a valid
  abstract relationship, then checks the refusal code, cause, alias locus,
  form, refinement and selected profile.
- FR-140-AC-15: the tagged test exercises the same population ordering helper
  used by `map_state`, with declaration order opposite to node-key digest
  order. The row key is minted by the existing canonical population-node
  preimage helper.
- FR-169-AC-11: existing checker and settlement integration tests both pass.
- `EvalFailure::Stopped` has no variant or caller in the base implementation.
- The optional semantic intent↔test↔code review was skipped. The `quoin`
  executable is installed, but `quoin gap-analysis` is not an available
  command in version 0.29.1. A full `quire matrix --scope . --format tsv` scan
  produced no output for several minutes and was stopped; the targeted
  requirement/test tags were checked directly instead.

## Checks

- `cargo fmt --check`: pass.
- `cargo check --locked --workspace --all-targets`: pass.
- `cargo clippy --locked -p qsl-semantics -p qsl-replay --all-targets --features test-support -- -D warnings`: pass.
- `cargo test --locked -p qsl-semantics --features test-support --test it an_abstract_model_relationship_refuses_the_refinement`: pass.
- `cargo test --locked -p qsl-replay population_rows_sort_by_abstract_node_key_digest`: pass.
- `cargo test --locked -p qsl-replay --test it refinement_mapping::`: 9 passed.
- Both focused qsl-analyze FR-169-AC-11 tests: pass.
- `cargo test --locked --all-features -p qsl-semantics -p qsl-replay`: 765 semantic unit tests and 1,219 semantic integration tests passed before one unrelated resource-measurement test failed because `/usr/bin/time` reported `sysctl kern.clockrate: Operation not permitted`; its child test itself passed. The command exited 101.
- `cargo deny check`: fails under unchanged `deny.toml`; the dependency graph includes upstream `AGPL-3.0-or-later` packages that are not explicitly allowed.

## Findings

| ID      | Severity | Summary                                                                                              | Refs                                                                    |
| ------- | -------- | ---------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| FND-001 | low      | Full local crate verification is incomplete; rerun the full suite and license gate in hosted CI.     | qsl-semantics/tests/it/model_identity_full_size.rs:179, deny.toml        |

## Dispositions

| ID      | Outcome                                         | Reason                                      |
| ------- | ----------------------------------------------- | ------------------------------------------- |
| FND-001 | deferred: hosted CI verification before merge  | Local resource and dependency-policy limits |
