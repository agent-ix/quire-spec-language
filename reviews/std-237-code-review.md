---
id: SR-15410
title: 'Code review of PR #1207: conformance harness supplies QSpec dependency packages, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@5d4be17f7; git diff origin/main...5d4be17f7 (PR #1207, branch qsl-conformance-fix): qsl-package/src/checked_v2/tests.rs; ticket STD-237'
review_set: subset
date: '2026-10-22'
---

## Summary

Ticket: STD-237. PR: quire-spec-language#1207, head 5d4be17f7. Method: code-review with the rust-review lane. The diff adds one test helper, `supply_qspec_dependency_packages`, to `qsl-package/src/checked_v2/tests.rs` and calls it from `read_fixture_wire`. No production code, build file or existing assertion changed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| supply_qspec_dependency_packages | qsl-package/src/checked_v2/tests.rs:1307-1367 | examined |
| read_fixture_wire supply call | qsl-package/src/checked_v2/tests.rs:1379 | examined |
| conformance_dependency_selection_vectors | qsl-package/src/checked_v2/tests.rs:1741 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The helper inlines the package-id digest as `format!("{:x}", Sha256::digest(&jcs(..)))` although the file already has `package_id_digest` for exactly this. | qsl-package/src/checked_v2/tests.rs:1335-1338 |
| FND-002 | low | When the dependency library is refused, `unwrap_or_else(\|_\| panic!(..))` throws the refusal away, so the panic message does not show the code or locus. | qsl-package/src/checked_v2/tests.rs:1360 |

## Verdict

Mergeable. Both findings are minor. Evidence:

- `cargo test -p qsl-package --lib checked_v2` with QSPEC_DIR=quire-specification@8ecde34: 42 passed, 0 failed.
- `cargo fmt --check` passes, and `cargo clippy -p qsl-package --all-targets -D warnings` passes.

Mutation checks show the test still exercises the production reader:

- Without the supply step, positive-dependency-call is refused `missing_import`, and both conformance tests fail.
- With the package supplied under the wrong identity, the read is refused `missing_import`/`missing-selection` at /lock/dependency_selections/0, and the tests fail.
- With a different admitted package supplied under the right identity, the read is refused `stale_dependency`/`content_mismatch` at /lock/dependency_selections/0/package_id/digest, and the tests fail.

`conformance_dependency_selection_vectors` still expects `missing_import` and passes, because the placeholder ids match no published fixture. Recursion always ends: a package id is a hash over a preimage that includes the package's own dependency_selections, so a fixture cannot select itself or form a cycle.
