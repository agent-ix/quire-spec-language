---
id: SR-13930
title: 'Spec review of PR #1185: EN-4 manifest owner, mode and evidence kind (STD-251), spec-integrity analysis'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@7f855b716; git diff origin/main...7f855b716 (PR #1185, branch std-qsl570-251, merge base 48b47ab99): spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md, spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md, spec/test-cases/TC-627-statistical-results-settle-as-measured.md, spec/test-cases/TC-761-qsl-engine-manifests-register-and-the-provider-entry-returns-analyze.md; ticket STD-251'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-280
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-189
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-761
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-627
  type: reviews
---

## Summary

Ticket: STD-251. PR: quire-spec-language#1185, head 7f855b716, based on main (not stacked). Method: spec-review with the spec-integrity sub-analysis. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

The PR adds to FR-280's manifest bullet that each engine manifest is data of the engine's module in `qsl-analyze`, that `qsl-route` holds the registry and FR-288's conversion and no engine manifest, and that EN-4 lists (`probabilistic-satisfaction`, `bounded`) with evidence `statistical` only. It adds an AC for that, numbered FR-280-AC-3, extends TC-761 with step 3, and aligns FR-189, FR-192 (statement and AC-4) and TC-627 step 4 with the pair and mode.

Gates at head: `make check-spec-validation` exit 0, with no warning on any touched file; `tools/check-index-completeness.sh` exit 0. The diff touches nothing under `docs/` and no test-parsed vector table. The PR is QSL-only; there is no QSpec half, so the QSpec gates do not apply. QSpec FR-331 already lists "for `probabilistic-satisfaction`, its advertised evidence kinds (FR-290)" as a manifest member, so the QSL text agrees with QSpec.

PR body claims checked. FR-057 admits twelve kinds including `probabilistic-satisfaction` (FR-057 table; FR-075 Description "the twelve labels FR-290 fixes and FR-057 admits"). FR-075 gives the descriptor an `evidence_kinds` set with `statistical` and `exact` (FR-075 Inputs, AC-10 to AC-14). The code half is as stated: `qsl_semantics::check::Capability` (qsl-semantics/src/check/capability.rs) walks "its ten members", and `qsl_route::BackendDescriptor` (qsl-route/src/lib.rs:162-170) has no evidence-kind member. ADR-013 C-28 already has the driver pass `evidence_kinds` to `BackendDescriptor::admit`, so FR-280-AC-3's conversion path is specified. No AC states a result the code already produces, so no probe was run.

Lead defaults: `qsl-analyze` owning the manifests agrees with FR-281 (the engines' crate) and with C-28 (the driver reads manifests, route admits). EN-4 mode `bounded` is consistent with ADR-012 §7.1's two modes. It leaves one gap: a `bounded` pair requires `domains` (FND-002). EN-5's mode stays unstated, as the lead default says.

Other open PRs: only #1186 (std-qsl570-252) touches FR-189, FR-192 and TC-627, in different hunks (quantile field, symmetry, TC-627 scope line). It does not edit the manifest statements, FR-192-AC-4 or TC-627 step 4. No open PR touches FR-280, TC-761 or TC-767.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-280 manifest bullet (owner sentence, EN-4 content) | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:56-63 | examined |
| FR-280-AC-3 (new, EN-4 manifest) | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:79 | examined |
| FR-280-AC-3 (existing, cargo tree) | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:80 | examined |
| FR-280 relationships and Dependencies | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:4-22,82-92 | examined |
| FR-189 EN-4 manifest statement | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:214-215 | examined |
| FR-192 EN-4 manifest statement | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:80-83 | examined |
| FR-192-AC-4 | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:140 | examined |
| TC-761 scope, step 3, expected result 3 | spec/test-cases/TC-761-qsl-engine-manifests-register-and-the-provider-entry-returns-analyze.md:15,21,29 | examined |
| TC-627 step 4 expected result | spec/test-cases/TC-627-statistical-results-settle-as-measured.md:33 | examined |
| FR-057 kinds and `domains` rule | spec/functional/FR-057-admit-shared-capability-kinds.md:57,249-282 | context_only |
| FR-075 Inputs and evidence-kind ACs | spec/functional/FR-075-compute-candidates-from-registered-backends.md:20-46,208-224,299-303 | context_only |
| FR-288 conversion | spec/functional/FR-288-build-the-registry-from-provider-manifests.md | context_only |
| ADR-013 C-28 | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1080 | context_only |
| FR-195 EN-5 manifest statements and AC-5 | spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:28,122,143 | context_only |
| QSpec FR-331 manifest members, `domains` rule, AC-22 | quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:63,220-223,402 | context_only |
| Capability and BackendDescriptor code | qsl-semantics/src/check/capability.rs:72-90; qsl-route/src/lib.rs:162-170 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new AC reuses the id FR-280-AC-3, which the existing cargo-tree AC (verified by TC-767) already holds, so FR-280's table has two rows with id FR-280-AC-3. TC-767 ("Scope: FR-284-AC-1, FR-280-AC-3") and the arch-lint tests tagged `#[trace("TC-767", "FR-280-AC-3")]` (tools/arch-lint/qualified_core.rs:1081,1111) now point at an ambiguous id. `make check-spec-validation` does not catch duplicate AC ids inside one table. Number the new AC FR-280-AC-4 and update TC-761's scope and the tag it tells tests to use. | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:79-80 |
| FND-002 | medium | EN-4's manifest advertises a `bounded` pair, but neither FR-280 nor FR-189 says which `domains` it carries. FR-057 refuses `invalid_capability`/`invalid-domains` when `domains` is absent and a `bounded` pair is advertised (FR-057:273-276; QSpec FR-331-AC-22 makes the manifest schema refuse it too). So FR-280-AC-3's "a `BackendDescriptor` that `BackendDescriptor::admit` accepts" depends on a domain set the spec does not state, and two implementers would pick different ones (for example `{population}` or `{population, integer}`). State EN-4's `domains` in the FR-280 bullet and the AC, or record it as a lead default. | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:60-63,79 |
| FND-003 | low | The new FR-280 text cites FR-057 and FR-075, but FR-280 has no relationship edge to either and its `## Dependencies` names neither. FR-288 is listed. Add `references` edges and Dependencies entries for FR-057 and FR-075. | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:4-22,82-92 |
| FND-004 | low | EN-4's manifest content is now stated normatively in three requirements (FR-189 statement, FR-192 statement and AC-4, FR-280 bullet and AC-3), with two ACs (FR-192-AC-4 via TC-627, FR-280-AC-3 via TC-761) asserting the same listing. FR-280 now owns the manifest. A future change to EN-4's pairs or domains has to be made in three places. FR-189 and FR-192 could refer to FR-280 for the content rather than restate it. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:80-81,140; spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:214-215 |

## Verdict

Not mergeable as written. FND-001 is high: the new AC duplicates an existing AC id that tests already carry. FND-002 is medium: the `bounded` default leaves EN-4's `domains` unstated, and admission depends on them. The rest is sound. The owner sentence agrees with FR-281 and ADR-013 C-28. The FR-057 and FR-075 claims in the PR body match the text, and the stated code half matches the code. The FR-189, FR-192 and TC-627 edits agree with FR-280. QSpec FR-331 already carries evidence kinds on the manifest. The wording is behaviour-only, and the PR body contains no review-finding paraphrase. Apart from FND-001's TC-767 reference, no statement in `spec/` outside `reviews/` contradicts the new text: FR-195 and ADR-028 SP-3 cover EN-5 only, and FR-195's notation is noted in SR-13931.
