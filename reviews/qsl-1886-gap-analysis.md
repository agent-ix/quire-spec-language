---
id: SR-2026111305294500000002
title: "Gap analysis of QSL-1886 protocol captures and on-each templates"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR 2695 head e7b13c3125a89d3045ce10dc5a0f0dc9a94ac080; FR-207, FR-208, FR-209, FR-381 against qsl-eval/src/simulation/protocol and qsl-analyze/tests/it/protocol_roles.rs"
review_set: subset
---

## Summary

Ticket: QSL-1886. Planless gap analysis of the PR diff, computed with `quire
matrix --scope <repo> --format json` (quire 0.38.3, engine 0.52.1) at the PR
head. FR-207-AC-7 and FR-209-AC-9 are tagged by tests that exercise the new
stepper paths, and FR-381-AC-1 to AC-15 are all tagged. The ticket names
FR-208-AC-6 as specced work, and it is still `untagged`. The new test
`a_retire_predicate_reads_a_protocol_capture` covers a protocol capture read by
`until` but not the AC's `present(lookup<M::Job>(M::jobs, j) absent empty)`
population half or its `missing-name` variant, and carries no trace tag.

## Verdict

**FAIL**: one ticket criterion, FR-208-AC-6, is untagged. Under the
gap-analysis verdict rule, an untagged criterion that this PR was meant to
back is a FAIL.

Pre-existing untagged rows in the same FRs (FR-207-AC-2, FR-209-AC-4,
FR-209-AC-8) are unchanged by this PR and are not counted against it. Reverse
gap: the stepper now evaluates `on origin` captures at the initial state and
maps an undecided one to `UndecidedInitialGuard`. ADR-027 FO-3 states neither
behaviour (see the spec-review integrity artifact). No stubs, placeholder
returns or `todo!()` were found in the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-208-AC-6, one of the two criteria the ticket names, is untagged at the PR head. The added `until`-reads-a-capture test omits the population `lookup`/`present` half and the `M::absent` `missing-name` refusal, and carries no `#[trace]` | qsl-analyze/tests/it/protocol_roles.rs:1921, spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md:116 |
| FND-002 | low | The untagged test's doc comment cites "FR-381 (TC-653)". TC-653's scope is FR-208, not FR-381, so the citation names a TC/FR pair that does not exist | qsl-analyze/tests/it/protocol_roles.rs:1921 |
| FND-003 | low | No test drives an undecided protocol-capture initializer: no undetermined finding under `on each`, and no `UndecidedInitialGuard` under `on origin`. The guard-following path in `start` is unexercised | qsl-eval/src/simulation/protocol/roles.rs:83, qsl-eval/src/simulation/protocol.rs:1156 |

## Coverage

- FR-207-AC-7: tagged (`a_template_reads_the_protocol_captures_and_the_activation_binder`, TC-652). It fails without the change.
- FR-209-AC-9: tagged (`protocol_captures_are_evaluated_in_source_order_when_an_instance_starts_and_frozen`, TC-654). It fails without the change, and it fails if the captures were not frozen (`fresh`).
- FR-208-AC-6: untagged (FND-001).
- FR-381-AC-1 to FR-381-AC-15: tagged.
- Semantic review: not run. The dispatch scoped this to the PR diff, and intent was checked by hand for the two new ACs only.
- Plan completion: not assessed

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The new undecided-capture test depends on S3 admitting the capture initializer `6 mod (1 - 1) >= 0`. FR-381 "Captures" says S3 SHALL refuse an initializer with an undischarged definedness obligation (`undefined_expression`/`unproved-nonzero`). The test passes, so the checker admits it: a pre-existing S3 conformance gap that this test now builds on, and the test breaks when S3 conforms | qsl-analyze/tests/it/protocol_roles.rs:2005, spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:145 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-1902 (child of QSL-1886) carries the FR-208-AC-6 population half, which needs S3 `type_named_body` to call `with_named_populations` and `evaluate_protocol_slot` to pass population reads. The PR no longer claims the criterion: the test is untagged, its doc comment says it is not a test of FR-208-AC-6, and the PR body says "Remaining work: QSL-1902". QSL-1886 must not close as Done on that criterion |
| FND-002 | fixed | e305ec25d: the doc comment no longer cites "FR-381 (TC-653)"; it now reads "Not a test of FR-208-AC-6, so it cites no criterion" |
| FND-003 | fixed | e305ec25d: `an_undecided_protocol_capture_is_a_finding_on_each_and_a_refusal_on_origin` asserts that no `activate` succeeds and that a `ProtocolExpressionUndetermined` finding names `::z` under `on each`, and that `ProtocolSystem::new` returns `Unsupported::UndecidedInitialGuard` under `on origin`; it passes |
