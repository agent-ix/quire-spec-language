---
id: SR-12234
title: 'Gap analysis of PR #1114 (QSL-883) against FR-255, FR-096-AC-2, FR-185, FR-186, FR-393, NFR-011, TC-427, TC-620 and TC-621'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@b71362855e2e88200dfa6999f51fadd84e390227; git diff origin/main...HEAD (PR #1114): spec/functional/FR-096-*.md (limit kinds, AC-2), spec/functional/FR-185-*.md (Exact values, AC-2, AC-6), spec/functional/FR-186-*.md (exact values, AC-4), spec/functional/FR-255-*.md (setting table), spec/functional/FR-393-*.md, spec/non-functional/NFR-011-*.md, spec/test-cases/TC-427-*.md, TC-620-*.md, TC-621-*.md; tests in qsl-semantics/tests/it/probabilistic_declarations.rs, qsl-semantics/src/check/refusal.rs, qsl-foundation/src/diagnostic/stage.rs, qsl-replay/src/limits.rs; QSpec native-diagnostics catalog and FR-272 (read by id); ticket QSL-883'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-185
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-096
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-255
  type: reviews
- target: ix://agent-ix/quire-spec-language/NFR-011
  type: reviews
---

## Summary

Ticket: QSL-883. Planless. Plan completion: not assessed. The traceability below comes from the trace tags in the diff and the tests they name. No `quoin matrix` run was made, because the brief says no gates. `make check-spec-validation` (QSPEC_DIR=qspec-main-now) exits 0, with no warning on any file this PR touches.

How the tests bind to the criteria:

- **FR-255 setting table (`s3.rational_bits` row): `each_limits_field_maps_to_one_distinct_setting_and_sets_only_itself` (TC-720, FR-255-AC-3).** The binding is correct. The test reads FR-255's table from the spec file. It checks the name, the stage `S3` and the kind `rational bits` against `Setting`, and that setting the bound changes only that field. The replay `limits.rs` builder test sets `S3RationalBits` to 37 and compares it with `with_rational_bits(37)`.
- **FR-096-AC-2 (eight kinds): `limit_exceeded_reports_stage_limit_exceeded_per_setting` (TC-427).** The binding is correct. It iterates `Setting::ALL`, so `s3.rational_bits` gives `stage_limit_exceeded`/`rational-bits-exceeded`. `refusal.rs` tests cover the `CheckingLimitKind` cause and the setting round-trip.
- **FR-185 Exact values / FR-185-AC-2 (default ceiling): `exact_values_are_bounded_as_written` (TC-620).** Correct. It now asserts `rational-bits-exceeded`, limit 65,536 and actual above it, along with the admitted boundary values.
- **FR-185-AC-6 (weight sum): `a_distribution_total_is_bounded_after_each_addition` and `a_weight_sum_past_the_ceiling_refuses_promptly`.** Correct. The cause is `rational-bits-exceeded` at `d`, with limit 65,536 and actual 65,537.
- **FR-185 Behavior, configured ceiling: `a_configured_rational_bits_ceiling_is_named_by_the_refusal` (new, TC-620).** It asserts that 255 is admitted and 256 refused under `with_rational_bits(8)`, with `("s3.rational_bits", 8, 9)`. The behaviour is correct, but no AC states it (FND-004).
- **FR-186-AC-4 / TC-621 step 4 (limit arm):** no test produces it (FND-005).
- **FR-393 quantity literal past the ceiling:** no test (FND-004).

The PR re-lands spec text that hotfix PR #1007 removed. Measured against #1007's diff, the FR-255 row, the FR-096 kind list and AC-2, and the FR-185 Behavior and Notes are restored. So are the FR-186 and FR-393 inline cites, TC-427, TC-620 step 2 and TC-621 step 4, and the NFR-011 settings list, metric row, counter definition and default derivation. Not restored: FR-185-AC-2, FR-186-AC-4, NFR-011 Scope and NFR-011 Verification (FND-001, FND-003).

## Verdict

**No major findings, so the PR is mergeable.** The code matches FR-255, FR-096 and FR-185 Behavior, and the tests back them. The spec restoration is incomplete in two AC rows and in NFR-011, and those are mediums. The QSpec catalog gap is known and QSpec-owned. It is recorded as FND-002 and needs a QSpec ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-185-AC-2 and FR-186-AC-4 still say "refused with a limit refusal (`LimitExceeded`) pending QSL-883". This PR is the one that delivers QSL-883. FR-185 Behavior, TC-620 step 2 and TC-621 step 4 now name `rational-bits-exceeded`/`s3.rational_bits`, but the AC rows do not, so the ACs are weaker than their TCs and still call the work pending. Fix: restore the AC text to name `stage_limit_exceeded`/`rational-bits-exceeded` and setting `s3.rational_bits` (as #1001 had), and drop `pending QSL-883`. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:222; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:185 |
| FND-002 | medium | Known gap, owned by QSpec. QSpec's native-diagnostics `stage_limit_exceeded` row and FR-272 list seven causes and do not include `rational-bits-exceeded`. FR-096 (ADR-030 D-1) and FR-185 now present it as a catalog cause, with no mark that it is QSL-proposed. #1001's text cited STD-188 for this request, but STD-188 is about probabilistic fixtures. No Linear ticket was found for adding the cause to QSpec. Deferred: open a QSpec ticket for the eighth cause, and mark the cause QSL-proposed in FR-096 until that ticket lands. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:151-159; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:95-99 |
| FND-003 | medium | NFR-011 is only partly restored. Scope still says `PackageDeclarations::check` "applies its node, byte and work ceilings", with no rational-bits ceiling. It also says a standalone expression check applies only the node ceiling, although a quantity literal in one now applies `s3.rational_bits` (`typing.rs`). Verification has no rational-bits line; #1007 removed "covered by TC-620 (FR-185-AC-2) ...", and the ticket asks for verification to be restored. The metric row says "as written", but the weight sum (FR-185-AC-6) is bounded too. Dependencies do not cite FR-185. | spec/non-functional/NFR-011-bound-value-checking-work.md:30-34,56,149-167 |
| FND-004 | low | Two branches are uncovered at the AC level. First, the configured-ceiling test `a_configured_rational_bits_ceiling_is_named_by_the_refusal` is tagged `TC-620` with no `FR-185-AC-n`, although TC-620 says to tag the AC, and no FR-185 AC states that a configured `s3.rational_bits` replaces the default. Second, no test drives a quantity literal (FR-393, `typing.rs` `quantity_magnitude`) past the ceiling, default or configured, and none drives the weight-sum path under a configured ceiling. Add the configured case to FR-185-AC-2 or AC-6, tag the test with it, and add one quantity-literal case. | qsl-semantics/tests/it/probabilistic_declarations.rs:619-640; qsl-semantics/src/check/check/typing.rs:869-873 |
| FND-005 | low | This predates the PR, but the PR's spec edit restates it. FR-186-AC-4 and TC-621 step 4 expect a `rational-bits-exceeded` refusal for `alpha decimal(1, 4294967295)`. The claim path never bounds confidence or threshold values: `magnitude`/`ratio` read them without the ceiling, and the claim refuses `not-yet-implemented` (`a_huge_written_confidence_value_is_read_without_expansion`). That arm has no backing test until claim checking lands. Deferred to the FR-186 implementation ticket. | qsl-semantics/src/check/probabilistic.rs:593-720; qsl-semantics/tests/it/probabilistic_declarations.rs:1135-1145 |
| FND-006 | low | TC-427 step 1 reads "`diagnostic-count-exceeded` and `work-budget-exceeded` and `rational-bits-exceeded` for the eight kinds": a doubled "and" left by the list edit. Write it as "`diagnostic-count-exceeded`, `work-budget-exceeded` and `rational-bits-exceeded`". | spec/test-cases/TC-427-a-stage-limit-names-its-kind-bound-counter-and-locus.md:36-39 |

## Dispositions

Round 1, reviewed at 5117af0a9 (fix commit f81aad06e). `make check-spec-validation` (QSPEC_DIR=qspec-main-now) exits 0, and no `QSL-883`/`pending` marker remains under `spec/`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f81aad06e: FR-185-AC-2 and FR-186-AC-4 name the `rational-bits-exceeded` limit refusal and `s3.rational_bits`; `pending QSL-883` removed. |
| FND-002 | deferred | QSpec owns the native-diagnostics catalog and FR-272; QSpec is read-only for this PR. Needs a QSpec ticket that adds `rational-bits-exceeded` as the eighth `stage_limit_exceeded` cause. |
| FND-003 | fixed | f81aad06e: NFR-011 Scope names the rational-bit ceiling, including for a standalone expression's quantity literals; the metric row covers the running weight sum; Verification cites TC-620; Dependencies cite FR-185. |
| FND-004 | fixed | f81aad06e: `a_quantity_literal_is_bounded_by_the_configured_rational_bits` (255 ms admitted past S3, 256 ms refused naming `s3.rational_bits`, 8, 9) and `a_distribution_total_is_bounded_by_the_configured_rational_bits` (tagged FR-185-AC-6; 1/3 + 1/5 = 8/15 refused at ceiling 3, actual 4, admitted at 7). |
| FND-005 | deferred | Predates this PR: claim checking refuses `not-yet-implemented`, so FR-186-AC-4's limit arm waits on the FR-186 claim-checking implementation. |
| FND-006 | fixed | f81aad06e: TC-427 step 1 reads "`diagnostic-count-exceeded`, `work-budget-exceeded` and `rational-bits-exceeded` for the eight kinds". |
