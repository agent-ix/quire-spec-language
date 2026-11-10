---
id: SR-20261109172912000078
title: QSL-1826 native state refutation gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2628; ten changed Rust files; FR-202
  AC17-22
review_set: subset
---

## Summary

Independent read-only review of frozen 233cd90916d5dde7a396e551f2f7922be97e54b7 against fd123d0e2. Ticket QSL-1826.

## Verdict

FAIL; do not merge until the resource/cancellation findings are resolved and rechecked.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | AC-22 coverage misses cancellation inside semantic expansion/predicate calls and storage exhaustion before induced-action retention. | qsl-replay/src/probability_certificate/witness.rs:282-286,334-368,474-488 | implementation-bug-despite-evidence |

## Coverage

Reviewed all ten changed Rust files against fd123d0e2, the checked source admission, full FR-196/binding keys, original-cycle admission before any boundary shortcut, exact min/max and workload-side strict mass, actual semantic draw/transition/post-state reconstruction, first-decision outcomes, infinite terminal stutter, empty cylinders, full-state deterministic scheduler consistency and prefix-free actual draw cylinders. No semantic soundness defect was found in these controls. Until gives B precedence and rejects neither-A-nor-B or unresolved terminal; always rejects not-P prefixes and accepts genuine terminal P. The opaque reproduced token remains the only native refuted settlement authority. No emitted package/wire or analyzer graph trust is introduced. Existing proof/document outcome logic is shared. Existing old graph-enumerator wrapper is preserved.

FND-001 detail: original graph enumeration reaches model_distributions (probability_certificate.rs:684-735), which calls weighted_steps/actions without the caller Cancel. Scheduler validation and each independent path step also call those old APIs (witness.rs:282-286,334-368), and shared state_predicate::outcome calls state_predicates (state_predicate.rs:132). Those semantic owners create evaluator/expansion meters without this Cancel; polling before or after a whole call cannot observe cancellation inside an expensive expansion or predicate. The newly admitted lane promises this behavior in AC-22. Adopt the narrow owner with_cancel APIs for each actual semantic call, including analyzer witness conversion exact.rs:1052, and preserve typed cancellation. Root has coordinated the shared owner handoff; no waiver granted.

FND-002 detail: induce calls control once per popped state, then action_steps materializes and clones every selected outgoing ProducedStep (including action/draw/reward data), queue.extend retains an entry per successor, and induced.insert retains the vector. The supplied native control reserves one sizeof(ProducedStep) per call (exact.rs:916-925), independently of selected outdegree. A wide action therefore grows its retained vector and queue beyond remaining storage before reaching any subsequent control in reaching. Charge/preflight selected steps and queue/map/cloned payload growth before action_steps/queue growth, using a resource-aware helper while preserving the existing public wrapper. Later callbacks or eventual rejection cannot satisfy reserve-before-retain. The existing tiny storage=1 control stops earlier and cannot catch this allocation ordering.

Accepted final evidence: /tmp/qsl1826-state-final-replay.log (40 native/legacy), /tmp/qsl1826-state-final-adapter.log (13 adapter), /tmp/qsl1826-state-final-producer.log (2 producer), /tmp/qsl1826-state-final-core.log (24 unchanged controls), /tmp/qsl1826-state-final-routing-control.log (updated obsolete unsupported expectation plus independently replayed token), /tmp/qsl1826-state-final-clippy.log and final formatting logs. No heavy probe required for statically traced defects. No cargo, aggregate, make ci, full matrix, Kani, qualification or dependency gate rerun. CI/workflow/lock/manifest diff is empty; no gate weakening found.

Read repository README, AGENTS.md and CLAUDE.md, supplied handoff, reviewer/code-review/rust-review/rust-style and API/implementation/safety/boundary/testing guidance. No applicable AssuranceProfile found. This is a scoped engineering review, not whole-repository qualification. Plan completion: not assessed. Computed full matrix excluded by dispatch: manual scope uses real trace attributes; all six new ACs have nontrivial tests, but current AC-22 tests do not demonstrate cancellation inside these semantic calls or reserve-before-retain at a wide induced action. No spec/plan changes, so spec-review methods do not apply.

## Examined scope and bindings

```yaml
scope:
- id: FR-202-AC-17
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Until fixture: initial A-and-not-B has actions a and b reaching B-and-not-A
    with probabilities 1/4 and 3/4, otherwise neither-A-nor-B followed by terminal
    B. At threshold 1/2, a''s rejecting cylinder of mass 3/4 refutes minimum AtLeast,
    and b''s accepting cylinder of mass 3/4 refutes maximum AtMost. Equal-weight resolved
    workload has accepting and rejecting mass 1/2 each: its rejecting cylinders refute
    AtLeast 3/5 and accepting cylinders refute AtMost 2/5. All four produce only their
    matching reproduced token and retain semantic evidence; repeat replay gives equal
    outcomes. The failure prefix st'
- id: FR-202-AC-18
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Always fixture: initial nonterminal P has actions a and b reaching terminal
    P with probabilities 1/4 and 3/4, otherwise not-P followed by terminal P. At threshold
    1/2, a''s not-P cylinder of mass 3/4 refutes minimum AtLeast and b''s terminal-P
    cylinder of mass 3/4 refutes maximum AtMost. Equal-weight resolved workload refutes
    AtLeast 3/5 with rejecting mass 1/2 and AtMost 2/5 with accepting mass 1/2. Later
    P cannot repair the not-P prefix; the initial live P alone cannot be accepted.'
- id: FR-202-AC-19
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: With admitted threshold 1/2, empty paths of mass 1 refute AtMost for initial
    B-and-not-A until and initial terminal-P always; empty paths refute AtLeast for
    initial neither-A-nor-B until, terminal A-and-not-B until, and initial not-P always
    even with a later terminal P successor. Terminal A-and-not-B has until probability
    0 and terminal P has always probability 1 without a fabricated stutter step. Source
    threshold endpoints remain refused; probabilities 0 and 1 are admitted.
- id: FR-202-AC-20
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: In AC-17/18, changed source/package, foreign claim occurrence or S3 binding
    refuses identity mismatch; a changed formula that changes the represented verdict,
    changed threshold to equality, changed actual action/draw/post-state/full key,
    fabricated binding node, missing binding component or a formula-monitor key prevents
    a token. Formula spelling alone need not fail if the actual bound claim has identical
    semantics and identity checks pass. A wrong-verdict path, live undecided prefix,
    path extended beyond first success/failure, duplicate cylinder or prefix overlap
    yields no reproduced token; i
- id: FR-202-AC-21
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-17/18 two evidence paths demanding different actions at one complete
    choice key fail policy consistency; a policy under a resolved workload fails replay.
    Distinct supported draws reaching the same post-state contribute their separate
    exact masses once, while repeating a draw cannot increase mass. A raw reachable
    cycle beyond initial B for until or initial not-P for always is unsupported despite
    immediate absorption; acyclic counterparts remain admitted. Fairness, timed/reward/nested/bounded
    forms, multiple initials/bindings, unresolved workload, subsystem and randomized-policy
    evidence rema
- id: FR-202-AC-22
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-17/18 pre-cancellation, cancellation at final acceptance, or exhaustion
    of each caller expansion/work/storage/arithmetic limit during original admission
    or replay preserves its typed incomplete outcome with no truth, token or accepted
    evidence in a refuted record. Refused predicates preserve refusal. Bounds, PrecisionBudget,
    certificate acceptance alone and unreplayed exact engine verdicts cannot settle
    refuted. Native replay needs no emitted package or witness wire codec and imports
    no analyzer graph types.
bindings:
- test_id: direct_until_and_always_refute_both_extrema_and_resolved_workloads
  ac_id: FR-202-AC-17
  trace: correct
- test_id: direct_until_and_always_refute_both_extrema_and_resolved_workloads
  ac_id: FR-202-AC-18
  trace: correct
- test_id: initial_state_verdicts_use_empty_cylinders
  ac_id: FR-202-AC-19
  trace: correct
- test_id: altered_semantics_prefixes_policy_and_duplicate_cylinders_have_no_token
  ac_id: FR-202-AC-20
  trace: correct
- test_id: altered_semantics_prefixes_policy_and_duplicate_cylinders_have_no_token
  ac_id: FR-202-AC-21
  trace: correct
- test_id: cancellation_and_caller_storage_arithmetic_limits_return_no_token
  ac_id: FR-202-AC-22
  trace: correct
- test_id: terminal_stutter_is_evaluated_without_fabricating_a_model_step
  ac_id: FR-202-AC-19
  trace: correct
- test_id: terminal_stutter_is_evaluated_without_fabricating_a_model_step
  ac_id: FR-202-AC-20
  trace: correct
- test_id: equality_source_identity_and_opposite_verdicts_do_not_reproduce
  ac_id: FR-202-AC-20
  trace: correct
- test_id: original_cycles_are_rejected_even_beyond_initial_state_boundaries
  ac_id: FR-202-AC-21
  trace: correct
- test_id: cancellation_at_the_final_acceptance_poll_cannot_release_a_token
  ac_id: FR-202-AC-22
  trace: correct
- test_id: disjoint_actual_draws_to_one_post_state_count_once_each
  ac_id: FR-202-AC-20
  trace: correct
- test_id: disjoint_actual_draws_to_one_post_state_count_once_each
  ac_id: FR-202-AC-21
  trace: correct
- test_id: choices_at_one_complete_key_and_workload_policies_must_be_consistent
  ac_id: FR-202-AC-21
  trace: correct
- test_id: adapter_budget_exhaustion_keeps_incomplete_without_evidence
  ac_id: FR-202-AC-22
  trace: correct
- test_id: actual_s3_over_parameter_and_single_admitted_instance_drive_replay
  ac_id: FR-202-AC-20
  trace: correct
- test_id: maximum_mass_equal_to_the_authored_threshold_yields_parity
  ac_id: FR-202-AC-20
  trace: correct
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: independently inspected owner API propagation, actual native callers and pre-retention producer controls at 8d584ebf46a714bb5c93891d1962401a45ab85c7; meaningful positive and counterfactual controls pass. |

## Disposition round 1

**CONDITIONAL; mergeable for this implementation scope.** All original findings are fixed. The broader raw-key working-memory prerequisite remains open as stated below.

Inspected frozen 8d584ebf46a714bb5c93891d1962401a45ab85c7 and all changed owner/caller files. FND-001 is fixed: original graph expansion, scheduler action validation, independent workload/action steps, shared state-event outcomes, ordinary eventual/bounded observations, analyzer product building, conversion and system admission adopt the caller-aware owner APIs. Owner expansion carries Cancel into candidate budget, invariant, conjunction/pre/post and computed-write meters; reward admission/evaluation and state predicates use cancellable qualified meters. Recorded tripped cancellation is checked before an additional observer poll so its original cause survives. Invariant iteration stops on cancellation. Old entry points delegate with no token and preserve compatibility.

FND-002 is fixed: the stopped native lane first borrows its selected action; native_steps visits selected edges and admits action/draw/reward/rational payload and step/queue/map slots before allocating/cloning the first step. Queue growth, reverse predecessor maps/queues, accepted path copies, frontier links and scheduler entries have pre-retention controls. The legacy public wrapper remains. MaterializedStep is a test-support observation after the clone, not the reservation point. The one-byte-short 128-edge control requires zero materialized steps; the exact-budget positive control requires all 128, preventing a test that merely rejects everything.

Accepted evidence: 41 replay tests /tmp/qsl1826-state-fix1-replay2.log; eight state-proof controls /tmp/qsl1826-state-fix1-core-state2.log; owner cancellation /tmp/qsl1826-state-fix1-owner-cancel3.log; 13 adapters /tmp/qsl1826-state-fix1-adapter.log; three producers /tmp/qsl1826-state-fix1-producer.log; all-target clippy /tmp/qsl1826-state-fix1-clippy3.log. Counterfactual logs /tmp/qsl1826-state-fix1-red-{expansion,predicate,workload,wide}.log fail their intended assertions when the actual fixed paths are reverted, including 128 materialized steps instead of zero. No redundant cargo run or source edit by reviewer. The worktree is clean.

Residual scope is explicit: ModelSystem::state_key still constructs model Enc/canonical raw bytes under its pre-existing fixed identity_limits (1 GiB) before ProbabilityProductKey applies caller document limits. The retained-product quota does not cover that upstream raw-key scratch. This existing owner allocation gap needs the separately coordinated budgeted raw-key owner work and its public budgeted decimal primitive (upstream quire-exact#142); it is not fixed by these dispositions and is not a full working-memory qualification claim. The root keeps that prerequisite/partial scope on open QSL-1826. No upstream merge or exception is authorized by this review. Original FND-001/002 concern semantic cancellation and induced-vector retention, both now fixed. No new regression found within the reviewed implementation diff. Full computed matrix remains excluded by instruction; Plan completion: not assessed.

Fixed excerpts:

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: fn cancellation_inside_qualified_expansion_step_and_predicate_returns_no_token()
    {
```
