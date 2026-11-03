---
id: SR-2026110216374
title: "Code review — QSL-1493 timestamped finite-window evaluation"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2165 qsl-eval, qsl-replay, qsl-semantics"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: references
---
# SR-2026110216374: QSL-1493 PR #2165 code review

## Summary

Reviewed the frozen Rust diff for QSL-1493, including the temporal evaluator, spine adapter, semantic checker, tests, and FR-501/FR-330 contracts. The focused timestamped tests and formatting pass; inspection found several edge cases that can misclassify finite-window results, plus the all-features Clippy lane fails to compile new tests.

## Verdict

**CONDITIONAL** — seven medium findings remain; the normal focused timestamped tests pass, but the all-features clippy command fails while compiling new tests.

## Coverage

- Code-review with the Rust lane; reviewed all eight changed Rust files and traced the new evaluator through the checked clause and spine entry point.
- `cargo fmt --all --check`: passed.
- `cargo test -p qsl-replay timestamped_` via `~/dev/worktrees/locked-build.sh`: 8 passed, 0 failed.
- `cargo clippy -p qsl-eval -p qsl-replay -p qsl-semantics --all-targets --all-features -- -D warnings` via the locked build lane: failed compiling qsl-replay test code at three refutable `let` bindings because the `Extracted` `ClauseRunSource` variant is enabled.
- `cargo deny check`: exit 0; existing git-source warnings were emitted, and the tool reported advisories, bans, licenses and sources OK.
- No CI workflow files changed. GitHub checks at review time showed `cla / gate` and `cla-check-internal` passing; this is not evidence that the local all-features clippy command passes.
- The Quoin gap-analysis method is recorded in its separate SR artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An undefined `on each` activation guard is skipped as if false, so an unestablished guard can make the finite-window clause vacuously hold. | qsl-eval/src/temporal/observed.rs:146 |
| FND-002 | medium | A counter-cover straddle overrides a definite earlier witness, converting a true existential result to indeterminate. | qsl-replay/src/spine/clause/temporal.rs:654 |
| FND-003 | medium | Any equal-stamp pair is refused when the formula contains an order-sensitive operator, even when those positions are outside that operator's interval. | qsl-replay/src/spine/clause/temporal.rs:225 |
| FND-004 | medium | Conflicting same-stamp readings are recognized only for a root `eventually holds(...)` formula; other operators can report a definite result from conflicting evidence. | qsl-replay/src/spine/clause/temporal.rs:685 |
| FND-005 | medium | Any past operator plus a positive first stamp is treated as missing history, even when its lower boundary does not precede the trace. | qsl-replay/src/spine/clause/temporal.rs:671 |
| FND-006 | medium | The public progress constructor accepts a negative uncertainty margin, which can close a window before its actual deadline and produce a false definite result. | qsl-eval/src/temporal/timed.rs:77 |
| FND-007 | medium | Three new tests bind `ClauseRunSource::Program` with refutable `let` patterns and fail all-features test compilation when `Extracted` is enabled. | qsl-replay/src/spine/clause/tests/temporal.rs:1522,1713,1820 |

## Scope

- {id: "qsl-eval/src/temporal/mod.rs", path: "qsl-eval/src/temporal/mod.rs", role: examined, excerpt: "New finite-window evaluator and progress APIs are publicly re-exported."}
- {id: "qsl-eval/src/temporal/observed.rs", path: "qsl-eval/src/temporal/observed.rs", role: examined, excerpt: "Finite-window activation selection, observed atom readings, outcome aggregation and meter accounting."}
- {id: "qsl-eval/src/temporal/timed.rs", path: "qsl-eval/src/temporal/timed.rs", role: examined, excerpt: "Finite-window closure, evaluator state, temporal visitation and future-reach/deadline computation."}
- {id: "qsl-replay/src/spine.rs", path: "qsl-replay/src/spine.rs", role: examined, excerpt: "New timestamped finite-window spine API exports."}
- {id: "qsl-replay/src/spine/clause.rs", path: "qsl-replay/src/spine/clause.rs", role: examined, excerpt: "Temporal verdict categories, exit codes and report usage fields."}
- {id: "qsl-replay/src/spine/clause/temporal.rs", path: "qsl-replay/src/spine/clause/temporal.rs", role: examined, excerpt: "Timestamped trace admission/order, progress mapping, verdict conversion and uncertainty/conflict/history special cases."}
- {id: "qsl-replay/src/spine/clause/tests/temporal.rs", path: "qsl-replay/src/spine/clause/tests/temporal.rs", role: examined, excerpt: "New traced timestamped-profile tests for AC-1 through AC-4 and AC-7 through AC-12."}
- {id: "qsl-semantics/src/check/temporal_clause.rs", path: "qsl-semantics/src/check/temporal_clause.rs", role: examined, excerpt: "The finite-window profile reads bounded step intervals as timestamp intervals and refuses timed intervals."}
- {id: "FR-501-AC-1", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Sparse timestamp windows use eligible instants and the closed window adds no synthetic false tail."}
- {id: "FR-501-AC-2", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Closed finite windows settle empty future intervals without false extension."}
- {id: "FR-501-AC-3", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Open windows remain pending until the deadline; decisive witnesses and counterexamples settle early."}
- {id: "FR-501-AC-4", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Missing and conflicting atom evidence is indeterminate unless another witness decides the result."}
- {id: "FR-501-AC-5", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Incomplete input yields indeterminate; a complete trace with the same valuation yields a definite result."}
- {id: "FR-501-AC-6", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Inclusive window deadlines participate; positions outside the interval do not, and an unproven half-open watermark is not inclusive coverage."}
- {id: "FR-501-AC-7", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Equal stamps require comparable order keys only when both positions participate in an order-sensitive operator."}
- {id: "FR-501-AC-8", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Past windows before a history cutoff are indeterminate unless represented evidence decides them."}
- {id: "FR-501-AC-9", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Exact and counter stamp covers distinguish definite bounds from a straddling bound."}
- {id: "FR-501-AC-10", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Each activation is judged independently; a definite earlier false result outranks a later pending activation."}
- {id: "FR-501-AC-11", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "The spine reports success, violation, pending and indeterminate categories/exits and refuses mismatched trace shapes."}
- {id: "FR-501-AC-12", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Finite-window temporal visits are charged exactly and denied work returns incomplete."}
- {id: "FR-330-AC-9", path: "spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md", role: examined, excerpt: "The finite-window profile accepts timestamped input and refuses a position-only or mismatched profile input."}
- {id: "FR-044", path: "spec/functional/FR-044-activate-temporal-obligations.md", role: examined, excerpt: "If a guard cannot be established from inputs, report an incomplete or refused activation; do not treat it as false."}
- {id: "FR-285-AC-1", path: "spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md", role: context_only, excerpt: "The category table maps the ten categories, including pending and indeterminate, to their exit behavior."}

## Findings Detail

### FND-001

For `on each ... when (...)`, `letters.activation` can return `AtomReading::Undefined` when the guard evaluation cannot be established. The new finite-window path handles that value in the same branch as `False`, omitting the activation. If every guard is undefined, the activation list is empty and the aggregation defaults to `Holds`. FR-044 explicitly requires an incomplete or refused activation and says an unestablished guard must not be treated as false.

### FND-002

`counter_straddles_bound` returns an index for any true consequent reading whose counter cover straddles the upper bound. `finite_window_verdict` then unconditionally replaces the already-computed verdict with `Indeterminate`. For `eventually[0,3000] ack`, an ack definitely within the bound (for example tick 2998 under `Counter { uncertainty: 0 }`) already makes the result true; a second ack at tick 3000 must not erase that decisive witness.

### FND-003

`same_stamp_order_refusal` checks only whether the formula contains any order-sensitive operator and whether the input has any equal-stamp group. It never checks whether that group is eligible at an activation for the operator's interval. For example, an `until[3,5]` anchored at the first tick cannot read duplicated positions at tick 0, but the whole request is still refused for missing order keys. FR-501 requires refusal only when both equal-stamp positions participate.

### FND-004

`conflicting_timestamped_valuation` recognizes only a root `eventually` whose operand is one `holds` node. The existing special case makes AC-4 pass for that formula, but conflicting readings are not lifted as possibilities for other operators. With `always[0,5] holds(p)` and true/false readings at the same stamp, the timed evaluator can return a violation from the false copy instead of `Indeterminate { ConflictingValuation }`.

### FND-005

The history conversion checks only `!authoritative_origin`, a positive first stamp and the presence of any past operator. It does not compare the operator's actual lower boundary with the first represented stamp. A trace starting at tick 3 with `once[0,0] holds(p)` and `p` false has all required history; under `HistoryCutoff` it should be violated, but this branch changes it to `MissingHistory` indeterminate.

### FND-006

`FiniteWindowProgress::new` stores any `Rational` uncertainty margin. A caller can pass a negative margin, making the computed deadline earlier than `t0 + Hf`; if the watermark and completeness cover that shortened deadline, a formula with an unresolved future interval can be reported definite. The finite-window contract permits only a non-negative margin derived from the counter uncertainty.

### FND-007

With `--all-features`, `ClauseRunSource` includes `Extracted`, so these `let` bindings are refutable and Rust rejects the test target with E0005. The three new sites are the source-rewrite helpers in `timestamped_counter_cover_uses_half_open_boundary_rules`, `order_sensitive_equal_stamps_require_one_authority_and_sort_by_key`, and `timestamped_guarded_on_each_activations_are_evaluated`; use `let ... else` or an exhaustive match.

## Bindings

- {test_id: "timestamped_windows_use_ticks_and_close_without_false_extension", ac_id: "FR-501-AC-1", trace: correct}
- {test_id: "timestamped_windows_use_ticks_and_close_without_false_extension", ac_id: "FR-501-AC-2", trace: correct}
- {test_id: "timestamped_windows_pending_and_decisive_witnesses", ac_id: "FR-501-AC-3", trace: correct}
- {test_id: "timestamped_missing_and_conflicting_atom_readings_are_indeterminate", ac_id: "FR-501-AC-4", trace: correct}
- {test_id: "timestamped_counter_cover_uses_half_open_boundary_rules", ac_id: "FR-501-AC-9", trace: correct}
- {test_id: "finite_window_report_preserves_pending_and_indeterminate_exit_classes", ac_id: "FR-501-AC-11", trace: correct}
- {test_id: "timestamped_window_charges_exact_visits_and_stops_on_the_denied_visit", ac_id: "FR-501-AC-12", trace: correct}
- {test_id: "timestamped_profile_and_trace_shape_must_match", ac_id: "FR-330-AC-9", trace: correct}
- {test_id: "order_sensitive_equal_stamps_require_one_authority_and_sort_by_key", ac_id: "FR-501-AC-7", trace: correct}
- {test_id: "timestamped_past_windows_respect_history_cutoff", ac_id: "FR-501-AC-8", trace: correct}
- {test_id: "timestamped_guarded_on_each_activations_are_evaluated", ac_id: "FR-501-AC-10", trace: correct}

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | The guard now returns Undefined, which the spine maps to a violation; FR-044 requires an incomplete or refused activation naming the guard. |
| FND-002 | fixed | A definite in-bound consequent witness now suppresses the later counter-cover straddle result. |
| FND-003 | fixed | Order refusal now traverses only positions participating in each interval; a regression test covers duplicates outside the interval. |
| FND-004 | fixed | Conflicting readings at an equal stamp are lifted to unknown for general temporal formulas, with an `always` regression test. |
| FND-005 | fixed | Missing-history conversion now checks the actual lower boundary of reachable past operators; zero-width `once` has a regression test. |
| FND-006 | fixed | The progress constructor rejects negative uncertainty margins. |
| FND-007 | fixed | The test helper uses a `let-else` binding that compiles when `Extracted` is enabled; focused all-features tests passed. |

## Disposition pass 1 summary

FND-002 through FND-007 are fixed. FND-001 remains open: the new undefined-guard path returns `TemporalOutcome::Undefined`, which the spine maps to `TemporalVerdict::Undefined` and the `Violation` category. FR-044-AC-7 requires an incomplete or refused activation naming the guard. The regression test asserts the `Undefined` result, so it does not establish the required behavior. The PR is not mergeable until this is corrected.
