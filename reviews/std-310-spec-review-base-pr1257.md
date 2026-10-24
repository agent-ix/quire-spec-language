---
id: SR-16050
title: "Spec review — FR-230 reads package clock fields and package time units"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md, spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md at 97ac55a"
review_set: subset
evaluated_revision: "97ac55a912d03bc2c73deb735f6cbc3f051eb604"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-685
    type: reviews
---

## Summary

Ticket: STD-310 (with STD-309, QSL part). PR quire-spec-language#1257, head 97ac55a. Reviewed FR-230 'The time dimension and its units', the dimension-conflict bullet, 'Clock fields and the domain package', the Timed facet gate edit, AC-1/2/4/5 and the AC-6 removal, and TC-685 fixtures, scope and step 6, against filament-core-data FR-146, QSpec #374/#376 and FR-415. No remaining reference to FR-230-AC-6 or STD-192 outside historical reviews.

## Verdict

PASS WITH MINOR FINDINGS, but merge it after QSpec #376: FR-230 relies on #376's package-unit naming (`time dense unit M::ms`), so #376 FND-001 (M::ms resolution against acme/rpc/ms) must be settled first and may change the spelling here. The vectors check by hand, including the source `unit ms : Time = 1/1000 * M::s;` case, which correctly avoids a second root of the package Time. `QSPEC_DIR=<#376 head> make check-spec-validation` exits 0 with no FR-230 or TC-685 diagnostic. Ids unchanged (AC-6 removed, no new ids). No code, fixture or vector changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Step 6 checks 'a source that declares dimension Rate; beside a package Rate', but the only package fixture is acme/rpc with dimensions [Time] and units [s, ms]; no fixture package declares Rate. Add a Rate dimension (or a Rate variant of acme/rpc) to the fixtures. | spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md:19,26 |
| FND-002 | low | FR-230 states the two-package case ('the same holds for two admitted domain packages, at the later package's selection row'), but AC-5 and TC-685 step 6 only cover source-versus-package. Add the two-package vector, or cite QSpec TC-187 U34 as its evidence. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:147-152,281 |
| FND-003 | low | The head does not contain current origin/main (39823fe7, 423ac6aa and four other commits); it merges cleanly. Merge origin/main into the branch before landing, as the lane convention requires. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:- |

## Scope

| ID | Path | Role |
| --- | --- | --- |
| FR-230 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-1 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-2 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-4 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-5 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| TC-685 | spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md | examined |
| TC-685 | spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md | examined |

## Dispositions

Round 1 at 1032ae3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

Round 1 verdict: MERGEABLE once QSpec #376 lands (or with it): all three round-0 findings fixed at 1032ae3. TC-685 has the Rate variant and acme/flow, AC-5 and step 6 cover the two-package Time conflict at /lock/model_selections/1, and FR-230's package-unit wording matches #376's declaration-less nodes reached by M::name. #376 FND-005 (preimage schema) does not change any FR-230 text. Gate: QSPEC_DIR=<#376 head aea3033> make check-spec-validation exits 0 with no FR-230 or TC-685 diagnostic. Main bc6c446 is 3 commits ahead of the head and merges cleanly.
