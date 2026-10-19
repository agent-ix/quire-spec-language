---
id: SR-12916
title: 'Base review of STD-191 (PR #1004): FR-393 S2 decimal scale refusal, FR-230 time dimension and units'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@b9c479125; spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md; spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md; spec/test-cases/TC-620-s3-admits-random-parameters-workloads-and-rewards.md; spec/test-cases/TC-621-s3-checks-probabilistic-claim-forms.md; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md; context: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-257-build-forms-at-any-depth.md, spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md, spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; agent-ix/quire-specification@ad2b37e spec/functional/type-model/FR-142; agent-ix/filament-core-data@e777d8d spec/functional/FR-027'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-393
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-230
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-185
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-186
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-620
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-621
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-685
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-1078
  type: reviews
---
# SR-12916: base review of STD-191 (PR #1004)

## Summary

Ticket: STD-191 (STD-192 is the filament-core-data follow-up). Scope is `git diff origin/main...HEAD` at b9c479125: FR-393's new S2 decimal-scale refusal and AC-8, the FR-185/FR-186 vector renumbering and TC-620/TC-621/TC-1078, and FR-230's new time-dimension, unit-lookup and Clock-field sections with AC-5 and TC-685 step 5.

What holds: the FR-185 renumbering composes textually with the merged as-written bound. `decimal(1, 19728)` is 65,535 bits and `decimal(1, 19729)` is 65,539 bits (checked with Python `(10**s).bit_length()`), so the 65,536-bit inclusive boundary stays where FR-185 puts it. A unit padded to 19,729 bytes or more admits both scales at S2. `decimal(1, 4294967295)` moves to S2 consistently in FR-185-AC-2, FR-186-AC-4, TC-620 and TC-621, and FR-186 gains a `rational(n, n)` vector so the rational-bits refusal stays covered. The `1/1000` edge direction for `ms` matches FR-142's `target_value = scale x source_value + offset`. TC-1078's procedure and expected results agree step for step. The diff allocates no artifact ids, only FR-393-AC-8 and FR-230-AC-5, and neither is used in the open PRs (#1000 and #1003 touch no spec file; #995, #989, #983, #971, #970, #967, #723 and #706 touch none of these eight files). All eight files pass `quire validate` (quire 0.36.2).

What does not hold: the S2 refusal is reported as a stage limit, but QSL's limit rules say S2 has no limit and that every limit names a setting that raises it. And FR-230 says its time-source vectors run now, although the admitted domain package cannot declare a dimension or a unit graph today.

Units examined (each listed in the Linear comment's `scope` with its text): FR-185, FR-185-AC-2, FR-186-AC-4, FR-230, FR-230-AC-1, FR-230-AC-2, FR-230-AC-5, FR-393, FR-393-AC-8, TC-1078, TC-620, TC-621, TC-685. Context only: FR-091, FR-096, FR-255, FR-257, FR-277.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-393's new S2 refusal is reported as `stage_limit_exceeded`, which contradicts four unamended requirements. FR-255 says 'S2 has no limit of its own (FR-257)', and every `stage_limit_exceeded` must name a setting that lets the caller run the input again under a larger limit. FR-257 Behavior 4 says S2 takes no limit set and refuses by content through FR-091/FR-102/FR-112 causes. FR-096 requires a `LimitExceeded` to carry one of seven `LimitKind`s and its setting. FR-277 requires every bound to be a caller limits field. The bound here is the unit's own byte length, not a configured value. Raising `s1.input_bytes` does not change the outcome: the same source refuses again under any setting, so 'the setting that raises it is `s1.input_bytes`' is false. The bound is also redundant. FR-393 holds `decimal(c, s)` as written ('S2 neither ... raises ten to a scale'), and FR-185 already measures `⌊s·log₂10⌋+1` exactly and refuses past `s3.rational_bits` without expanding `10^s`. So S2 has no work to bound. Fix: either drop the S2 rule and restore `decimal(1, 4294967295)` as an S3 rational-bits refusal in FR-185-AC-2 and FR-186-AC-4, or make it a content refusal with an FR-091 catalog row (for example `ill_typed`/`type-mismatch`, as `IntegerOutsideI128` does) that names no setting, and amend FR-255 and FR-257 to match. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:156-170; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:102; spec/functional/FR-257-build-forms-at-any-depth.md:41 |
| FND-002 | high | FR-230 says the time-source vectors 'run now' and only Clock-field vectors wait on STD-192. That does not hold for filament-core-data on main (e777d8d, 2026-10-16). Its domain package declares no dimension and no unit graph: a field's `unit` is a bare UCUM symbol string (FCD FR-027, `UnitSymbol.json` is `type: string`), with no dimension identity and no scale edge to a root. STD-192 itself asks for both the Clock field type and 'the base dimension time with a zero-offset root unit s and scaled units such as ms'. So the lookup FR-230 now specifies (find `u` among the package's units, read its dimension identity, compose edges to the root) has nothing to read. FR-230-AC-5's first four vectors, AC-2's `time dense unit m` and AC-3's `unit ms` tick model wait on STD-192 too, or on another unit source. Fix: list those vectors as waiting on STD-192 as well, or name the unit source that exists today. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:128-137,199; filament-core-data spec/functional/FR-027-declare-field-multiplicity-and-units.md |
| FND-003 | medium | 'QSL declares no dimension and no unit of its own' contradicts FR-091, where a QSL unit declares dimensions and units in its own source (Dimension form, Unit form, the assembler's unit graph). It also contradicts FR-094-AC-8, which declares base dimensions `Length` and `Time` and the root unit `second` in source. FR-230 then looks `u` up only 'among the admitted package's units', so whether a source-declared `unit` (FR-091) is visible to `time dense unit u` is decided by omission. Fix: state which unit graph the lookup reads (package, unit source, or both), and why. Reword the first sentence to say that QSL has no built-in dimensions or units. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:98-101; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:248-262 |
| FND-004 | medium | AC-5's last vector, a `Clock` field refusing `unsupported_construct`/`not-yet-implemented`, contradicts AC-1, AC-2's clock vectors and AC-4, which require Clock fields to check. At no point can all of FR-230's ACs pass: before STD-192, AC-1 fails, and after it AC-5 fails. The AC table rows do not say they wait. The marking is only in the prose section and in a trailing sentence of TC-685's Step 5 expected result. 'The clock refusals of FR-230-AC-2' is also not a list: `time dense unit m`, the duplicate `time` member and `urgent when P` need no clock, and the rest do. Fix: mark AC-1, the named AC-2 vectors and AC-4 as 'Remaining work (STD-192)' in their rows, and move the interim Clock refusal to its own AC that says it is replaced when STD-192 lands. Alternatively, keep it out of the ACs (the precedent in SR-12913 FND-002: do not write an implementation gap into a vector). | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:132-137,195-199; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md:35 |
| FND-005 | medium | The S2 rule sits in FR-393, which covers time forms, and AC-8 tests it only 'as a quantity magnitude'. FR-185-AC-2 (a workload weight) and FR-186-AC-4 (`alpha`) now expect it at S2 in FR-311's forms, but FR-311 is not amended and no S2 AC covers those positions. The diff also does not say whether the rule applies to the `decimal(c, s)` exact numbers of FR-091's unit declarations, whose assembler bound is `s3.decimal_scale`. Fix: state the rule once, on `ExactValue` in every position where S2 builds one (or name the positions), and give FR-311 an AC for the weight and confidence positions. If FND-001 drops the rule, this finding goes with it. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:107-115,186; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:158 |
| FND-006 | low | TC-1078 puts the new AC-8 step at 7 and moves AC-7's Clock step to 8. The steps are now out of AC order. PR #1003's tests already say 'FR-393-AC-7 (TC-1078 step 7)' (qsl-forms/tests/it/time_forms.rs), and those comments go stale. Fix: keep the Clock units as step 7 and add the decimal-scale vectors as step 8. | spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md:27-28,40-41 |

## Verdict

Not mergeable. FND-001 and FND-002 are high. FND-001: FR-393's refusal contradicts FR-096, FR-255, FR-257 and FR-277, which are not amended. It also names a setting (`s1.input_bytes`) that cannot raise the bound, and it is not needed: FR-393 holds `(c, s)` as written and FR-185 already bounds the written denominator at S3 without expanding `10^s`. The planner default in STD-191 asked for this refusal, so the lead may need to rule. The two consistent options are a content refusal with no setting (FR-091 table and FR-257 Behavior 4), or no S2 rule, with `decimal(1, 4294967295)` refused at S3 by `s3.rational_bits` as before. FND-002: FR-230's 'run now' claim does not hold for filament-core-data as it is. Out of diff scope, for the lead: PR #1003's `scaled()` expands `10^s` at S2 (`divided_by_power_of_ten`), against FR-393's 'no `10^s` expanded'. Its `FormsCause::DecimalScaleBeyondSource` carries no counter or bound, and its tests' doc comments say 'FR-393-AC-7 (TC-1078 step 7)', which this renumbering makes step 8.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | 'Units of two such dimensions convert to one another by FR-142' contradicts QSpec FR-142. FR-142 admits a direct declared-to-declared conversion 'only when both units have the same dimension node identity; distinct dimension nodes with equal base-dimension maps, such as torque and energy, do not admit it.' A derived `Duration = Time` is a separate dimension node with its own root, so `time dense unit d` with `d` of `Duration` and a guard `x >= 3 ms` (`ms` of `Time`) cannot convert, and FR-230 promises that it can. Fix: make the time dimension the one dimension node whose identity is the base `Time` (a derived alias has its own units and is not the time dimension). Alternatively, require every time quantity in a model to share `u`'s dimension node and refuse otherwise. Drop the `Duration = Time` vector from AC-5, or make it a refusal. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:109-113,220 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed at b3a8e6c42. |
| FND-002 | fixed | Verified against FR-091 and filament-core-data: the assembler admits a unit's own `dimension`/`unit` forms into the checked package's `units` (FR-091 assembler steps 1-6, AC-32; FR-094-AC-8 declares base `Time` and root `second` in source), so source-declared time units are available now without STD-192; FCD main still has no unit graph, and FR-230 now waits only the domain-package vectors. |
| FND-003 | fixed | Fixed at b3a8e6c42. |
| FND-004 | fixed | Fixed at b3a8e6c42. |
| FND-005 | fixed | Moot: the S2 rule is dropped, so no position needs it. |
| FND-006 | fixed | TC-1078 is back to main: step 7 is the Clock units again. |
