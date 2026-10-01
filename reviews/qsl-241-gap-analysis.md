---
id: SR-927
title: "QSL-241 gap analysis of PR 547: FR-087-AC-7, the tests.md trace convention, and trace tags that resolve to nothing"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@35c74a26273504a59ff21cd54be903b727544189; spec/functional/FR-087-typestate-and-cross-package-node-key.md (AC-7); spec/tests.md (Requirements Traceability convention paragraph; exact-backing list TC-213..TC-242); qsl-semantics/src/library/bundle_tests.rs and bundle.rs (FR-087-AC-7 carriers); quire-exact/src/** (TC-300..TC-356 tags); qsl-semantics/tests/it/model_operations.rs:1538 (SR-766); tests/it/complete_cst.rs, tests/it/complete_editor.rs, qsl-semantics/tests/it/complete_value_lock.rs (Task-047/048); QSpec ids from agent-ix/quire-specification origin/main 4634f5fd."
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
---
## Summary

Ticket: QSL-241. PR: quire-spec-language#547 at 35c74a26.

- **FR-087-AC-7** (amended to spell the kept tags `QSpec-FR-131-AC-n` /
  `QSpec-FR-339-AC-n`) is met. All ten `library::bundle_tests` scenarios and
  `bundle::tests::resolved_graph_identity_matches_its_golden_vector` carry
  `TC-491`, their `FR-111-AC-n` and `QSpec-FR-131-AC-1..3` /
  `QSpec-FR-339-AC-3`. No bare `FR-131`/`FR-339` tag remains anywhere in the
  workspace.
- **tests.md convention paragraph** ("a bare id names a QSL artifact") is
  false for 61 tags in the workspace today. They name no artifact in either
  repository (FND-001, FND-002). Nine QSL TCs that tests.md marks passed and
  backs with named tests have no test carrying their tag anywhere in the
  workspace (FND-004).

**Orphan provenance (FND-001).** The 60 `TC-3NN` tags in quire-exact (55
distinct ids, TC-300..TC-356, in 19 files; 56 of the 60 tests carry nothing
else) were never QSpec ids. QSpec's highest TC is TC-282 on `origin/main` and
TC-279 at the local checkout. `git log --all` finds no `TC-3NN` file in QSpec
history. They were never QSL ids either: QSL's TC files jump from TC-297 to
TC-376, and `git log --all -S'id: TC-300'` finds no TC row ever. They were
minted in the crate's founding PR, #254 (`0b88fa03`). Its commit message reads
"44 unit tests (TC-300 through TC-343)", and later PRs extended the run to
TC-356. These are local ids that never got TC rows, not QSpec ids that were
renumbered. Under the ruling there are no local TC rows to add. The crate's
own lib.rs doc already says QSpec owns this behaviour, and that a test earns a
QSpec AC tag only where its assertions distinguish the AC.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | 60 `#[trace("TC-3NN")]` tags in quire-exact (TC-300..TC-356) name a TC that exists in neither repository. They were invented in #254 and never had rows | quire-exact/src/node.rs:127; quire-exact/src/division.rs:230; quire-exact/src/value.rs:815-1004; quire-exact/src/identity.rs:331-467 (and 15 more files) |
| FND-002 | low | `#[trace("SR-766", "FR-113")]` names a review finding id; no SR-766 artifact exists in the repo | qsl-semantics/tests/it/model_operations.rs:1538 |
| FND-003 | low | 13 `Task-047` tags and 1 `Task-048` tag trace tests to plan tasks, not to a requirement or TC they verify | tests/it/complete_editor.rs:52,155,206,249,301,346,380,495; tests/it/complete_cst.rs:220,260,318,367,404,453; qsl-semantics/tests/it/complete_value_lock.rs:23 |
| FND-004 | medium | tests.md marks QSL TC-213, TC-215, TC-226, TC-228, TC-229, TC-235, TC-237, TC-238 and TC-242 passed and names exact backing tests, but zero `#[trace]` tags in the workspace carry any of these ids (those tests carry only `QSpec-` tags). TC-213's named test `n01_normalizes_f1_to_the_exact_ground_truth_identities` no longer exists | spec/tests.md:423-440; qsl-semantics/tests/it/model_normalization.rs:673,813,1283; qsl-semantics/tests/it/model_systems.rs:373; qsl-semantics/tests/it/model_population.rs:371,615,1070 |

## Verdict

FR-087-AC-7 is traced and its spelling amendment is honoured.

Fixes:

- **FND-001:** delete the 60 `TC-3NN` tags. Add no TC rows. Where a test's
  assertions really distinguish a QSpec AC, tag it `QSpec-TC-NNN` /
  `QSpec-FR-NNN-AC-n`, as `tc_323_division_by_zero_is_undefined` already is;
  otherwise leave it untagged. The `tc_3NN_` function-name prefixes and the
  "TC-300 (...)" doc-comment openers cite the same fake ids. Renaming them is
  a churn rename, so ask first.
- **FND-002:** replace `SR-766` with the QSL TC that verifies binder
  no-shadowing, `TC-512`, plus `FR-113-AC-5` (the pairing
  `protocol_clause.rs:1852` already uses), or drop it and keep `FR-113`.
- **FND-003:** the `Task-NNN` ids resolve to real QSL plan tasks, so they are
  not mis-namespaced. But a plan task is not something a test verifies.
  Task-047 itself `verifies` QSpec TC-180/184/222, so retag each test to the
  QSpec TC/AC it checks. `complete_editor.rs:96` shows the form:
  `QSpec-TC-184`, `QSpec-FR-134-AC-1`. Then delete the Task tag.
- **FND-004:** add the bare QSL `TC-NNN` and `FR-NNN-AC-n` pair that
  tests.md names to each test it names. For example: `TC-237`, `FR-081-AC-6`
  on n06; `TC-238`, `FR-081-AC-7` on n07; `TC-235`, `FR-086-AC-3` on y02;
  `TC-242`, `FR-084-AC-6` on l01. Do the same for TC-215, TC-226, TC-228 and
  TC-229. Point TC-213's line at the renamed
  `n01_normalizes_f1_to_stable_deterministic_identities`. Also add
  `TC-219` to `model_normalization.rs::r01_a_closing_generalization_cycle_*`,
  the second test TC-219's line names.

FND-001 to FND-004 predate this PR, but this PR is the trace-namespace sweep
and they are the same class of defect, so they belong in it.
