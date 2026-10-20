---
id: SR-13441
title: 'Gap analysis of PR #1006: FR-307-AC-7 / FR-093-AC-16 / FR-323-AC-7 coverage of direct dependency selections'
type: SpecReview
scope: 'agent-ix/quire-spec-language@d47231a7e; git diff refs/remotes/origin/main...d47231a7e (PR #1006, branch code/ir-883-direct-dependency-selections): qsl-package/src/checked.rs, qsl-package/src/checked_v2.rs, qsl-package/src/emit.rs, qsl-package/src/emit/tests.rs, qsl-replay/src/execute/tests.rs, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; measured against QSpec origin/main 240faeb (PR #283: FR-307, FR-322, FR-323, dependency-selection-vectors.json); ticket IR-883'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-093
  type: reviews
---

## Summary

Ticket: IR-883. PR: quire-spec-language#1006, head d47231a7e. Method: gap-analysis. Planner brief: one reviewer, major findings only. Evidence on head d47231a7e (scratch clone, own CARGO_TARGET_DIR): `cargo test -p qsl-package -p qsl-replay` exit 0 (qsl-package 204 passed, qsl-replay 724 passed, doc/bin suites green); root `cargo test --test it` exit 0 (975 passed, 6 ignored). QSL's `conformance_*` tests (including `conformance_dependency_selection_vectors`) pass with QSPEC_DIR pointed at an export of QSpec origin/main 240faeb (the shared /Users/peter/dev/quire-specification checkout is at ad2b37e, behind #283). The three closure_vectors' direct-dependency `package_id`s recompute by JCS SHA-256 from their base fixtures with the vectors' direct selections substituted (3/3). Mutation probes, reverted after: filtering `direct_dependency_selections` by `DependencyPath::Direct` fails `a_direct_import_reached_transitively_first_stays_in_the_lock`; writing the whole closure (`filter(|_| true)`) fails `the_lock_records_direct_imports_and_derives_the_closure`. `transitive_only_library_source_reaches_replay` asserts A's lock is `[test/b]`, which the pre-PR closure emission fails, and replay accepts the transitive C entry because rules 6/7 check the in-memory closure (FR-323-AC-7).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-307-AC-7 | QSpec spec/functional/tooling/FR-307-package-reusable-semantic-libraries.md | examined |
| FR-322-AC-35 | QSpec spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |
| FR-322-AC-76 | QSpec spec/objects/interfaces/FR-322-checked-package-artifact.md | examined |
| FR-322-AC-75 | QSpec spec/objects/interfaces/FR-322-checked-package-artifact.md | context_only |
| FR-323-AC-7 | QSpec spec/objects/interfaces/FR-323-native-runtime-envelope.md | examined |
| FR-093-AC-16 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md | examined |
| ADR-011-2.4-dependency_selections | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011-OQ-5 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| CheckedPackage::direct_dependency_selections | qsl-package/src/checked.rs | examined |
| CheckedPackage::link_with | qsl-package/src/checked.rs | examined |
| emit::dependency_selections | qsl-package/src/emit.rs | examined |
| ClosureReader::supply/admit_closure | qsl-package/src/checked_v2.rs | examined |
| execute::recompile rules 6-7 | qsl-replay/src/execute.rs | context_only |
| lowering::ImportedCall | qsl-semantics/src/check/lowering.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Every criterion the PR touches has a tagged test that fails under the corresponding mutation: FR-307-AC-7 (QSpec-TC-227) by the chain, transitive-first and recursive-admission tests; FR-093-AC-16 (TC-416) by the renamed lock test, whose assertions now require exactly one direct entry and lock == preimage; FR-323-AC-7 (QSpec-TC-277) by the transitive-only replay test. No stub, no tautology, no coverage inflation: the added `e4_refuses_a_conflicting_diamond` assertions check each mid's own direct lock. FR-322-AC-75/76 are QSpec/IR-side (reader outcomes planned, vectors recompute) and are not QSL obligations in this PR.
