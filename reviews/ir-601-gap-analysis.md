---
id: SR-1317
title: "IR-601 gap analysis of PR #639 (QSpec FR-147 member-only division, FR-096-AC-13, FR-100 key table, TC-428, TC-452)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1d27057837c187c1106d7445e7e806faa9aa669b; PR #639 diff against origin/main (merge base bcca43356); qsl-semantics/tests/it/integer_division.rs; qsl-foundation/tests/kernel_refusal_record.rs; qsl-eval/tests/it/kernel_refusal_payloads.rs; qsl-replay/src/spine/call/tests.rs; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-428-a-refusal-record-carries-code-category-locus-and-fields.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; spec/test-cases/TC-202-evaluation-functions-unchanged-after-removal.md; QSpec spec/functional/expressions/FR-147-evaluate-integer-division-domains.md at quire-specification 2b2dd28"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---
## Summary

Ticket: IR-601. PR: quire-spec-language#639. This is a manual check of each AC
against its tests, with no plan bundle.

Trace, per unit:

- **FR-096-AC-13 / TC-428 step 5.**
  `a_division_cause_follows_the_exposed_member` and
  `division_refusals_name_the_consumer_domain_and_exposed_member` (both tagged
  TC-428, FR-096-AC-13) build `DivisionOutOfDomain` records for both members.
  They assert `expected` `Int[0, 9]` (or `Int[0, 1]` through the real
  `divide`) and the literal cause strings. The bindings are correct.
- **FR-100 key table / TC-452 step 4.** `tc_452_step_4_outcome_mapping_covers_every_category`
  maps both member variants to `division_out_of_domain` with the literal
  causes. The binding is correct.
- **QSpec FR-147-AC-1/AC-4 (identity and profile table).** The signed table, the
  generated oracle and `assert_law` cover all three profiles independently.
  The binding is correct.
- **QSpec FR-147-AC-2/AC-6 (zero divisor, accounting).** DIV-01, DIV-11 and the
  generated test check undefined for both members. DIV-08 checks the four
  renamed charges and `result_units` 1 for `div`.
- **Operator-level claims.** QSL's qsl-eval has no `div`/`rem` evaluator arm.
  FR-091-OQ-5 refuses `div`/`rem` as an unrepresented construct, so tests
  that call `value::divide` directly are the right seam. No FR-096 or FR-100
  AC claims operator-level behaviour. FR-096-AC-13 and the FR-100 table are
  record-level, and FR-099-AC-8's `10 div x` is a check-time
  `unproved-nonzero` refusal, not evaluation. Nothing built is
  over-claimed.

## Verdict

Changes requested on traces only. The code is fully exercised, but FND-001
(medium) leaves the two new QSpec ACs that this PR exists to implement
(FR-147-AC-9 and AC-10) with no QSL trace, and tags their test to AC-1/AC-4
instead. FND-002 (medium) is a refactor-equivalence TC that this PR's behaviour
change makes false. FND-003 (low) is an untested half of FR-147-AC-6.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No QSL test is traced to QSpec FR-147-AC-9 (each profile: `div` with q outside refuses `division_out_of_domain`/`quotient-outside-domain`, `rem` with r outside refuses `remainder-outside-domain`, `mod` refuses `modulo_out_of_domain`) or FR-147-AC-10 (the unexposed member never refuses: `10 div 5` over `Int[1,10]` gives 2, `-10 rem -1` over `Int[-10,5]` gives 0). Both were added by QSpec #188 and are what IR-601 implements. `only_the_exposed_member_must_lie_in_the_consumer_domain` tests exactly the AC-10 vectors and the AC-9 causes, but is tagged `QSpec-FR-147-AC-1`, `QSpec-FR-147-AC-4`, which are the identity and profile-table ACs. The member checks in `div_04_div_06_...` and in the generated test are also untagged for AC-9/AC-10. Fix: tag the new test `QSpec-FR-147-AC-9`, `QSpec-FR-147-AC-10`, and add AC-9 to `div_04_div_06_mathematical_and_signed_64_domains` and `generated_members_match_the_law_oracle_domains_and_every_denial`. | qsl-semantics/tests/it/integer_division.rs:728 |
| FND-002 | medium | TC-202 / FR-078-AC-3 claim that the tests in `integer_division.rs` "passed unchanged before and after the negotiate_* removal" and that evaluation is "unchanged". This PR changes `divide`'s signature, its result and its bounded-domain behaviour, and rewrites those tests. Every test in the file still carries the `TC-202`/`FR-078-AC-3` tags, so the TC now asserts a before/after equivalence that the tagged tests no longer have. This is a refactor-equivalence criterion (it tests a refactor, not a behaviour), and this change shows it is stale. Fix: retire FR-078-AC-3/TC-202 the way FR-078-AC-2 was retired, and drop the `TC-202`/`FR-078-AC-3` tags from the division tests. The QSpec FR-147 tags already carry the behaviour. | spec/test-cases/TC-202-evaluation-functions-unchanged-after-removal.md:22-29 |
| FND-003 | low | QSpec FR-147-AC-6 requires exact-bound accounting "of `div` and of `rem`". DIV-08 (`div_08_exact_bound_succeeds_and_each_named_denial_is_atomic`, tagged AC-6) runs only `DivisionMember::Quotient`. `rem` is covered for injected denials in the generated test, but no test checks its exact-bound success, admitted charge order or `result_units` 1. Fix: run DIV-08's exact-bound and named-denial assertions for both members (for example, a loop over `[Quotient, Remainder]` with the expected 2 or 1). | qsl-semantics/tests/it/integer_division.rs:323-336 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The FND-002 retirement left the TC-202 status rows unchanged. spec/tests.md:68 still lists TC-202 as "✅ Passed locally; `tests/ieee_profiles.rs` + `tests/integer_division.rs`", but this PR removed every `TC-202` tag from those files, and TC-202 itself now says "no test carries the `TC-202` tag". spec/spec.md:994 still reads "Implemented — see TC-201, TC-202". Fix: mark the tests.md row retired (for example, "Retired with FR-078-AC-3; no test") and drop TC-202 from the spec.md status cell, or mark it retired there. | spec/tests.md:68 |

## Dispositions

Round 1, reviewed at 2156c32cedfe0b8da4a07cc0202d5746589fa298 (fix commit
2156c32ce on 1d2705783; `git diff 1d2705783 2156c32ce`). No builds were run in
this round. The coder reports `make ci` exit 0 (qsl-601-make-ci2.log).

- FND-001: `only_the_exposed_member_must_lie_in_the_consumer_domain` is now
  tagged QSpec-FR-147-AC-9 and AC-10. `div_04_div_06_mathematical_and_signed_64_domains`
  and `generated_members_match_the_law_oracle_domains_and_every_denial` now
  carry AC-9.
- FND-002: FR-078-AC-3 is marked RETIRED, and TC-202's description says it is
  retired. Every `TC-202`/`FR-078-AC-3` tag is gone from integer_division.rs
  and ieee_profiles.rs (a `git grep` at the head finds them only in spec
  prose). The leftover status rows are FND-004.
- FND-003: DIV-08 now loops over Quotient (2) and Remainder (1). Each member
  asserts the exact-bound success, the four charges in order, consumption with
  `result_units` 1, both named denials, and an injected denial at each point.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 2156c32ce |
| FND-002 | fixed | 2156c32ce |
| FND-003 | fixed | 2156c32ce |
| FND-004 | still-open | New this round: spec/tests.md:68 and spec/spec.md:994 still report TC-202 as passing or implemented through tests that no longer carry the tag. |
