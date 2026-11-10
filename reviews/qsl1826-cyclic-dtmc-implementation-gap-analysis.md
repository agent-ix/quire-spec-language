---
id: SR-20261110035459820002
title: QSL-1826 cyclic DTMC implementation gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826 PR2643; 10-file child delta; FR201 AC23-26
  and FR282 AC18-19
review_set: subset
---

## Summary

Independent early blocked-draft review of frozen 5a536d91232a3d2b70a5ec3df7d5c5262bf212cf, exact reward parent cbf88f08886c0bde6fba4c57a41788965134221e. Ticket QSL-1826.

## Verdict

FAIL / BLOCKED; not mergeable until actual required bounded raw-key owner API/callback adoption is independently rechecked and affected final-head validation passes. Available math/S3/codec review found no additional concrete defect.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Cyclic caller-stop tests do not establish bounded raw ModelSystem key admission or transitive candidate/post-key callbacks; required owner API adoption remains missing. | qsl-replay/src/probability_certificate/cyclic.rs:30-33,195-198 | implementation-bug-despite-evidence |

## Coverage

Explicit EARLY BLOCKED-DRAFT review of frozen 5a536d91232a3d2b70a5ec3df7d5c5262bf212cf against exact parent cbf88f08886c0bde6fba4c57a41788965134221e, PR2643, ticket QSL-1826. All 10 changed files inspected with their semantic callers and relevant inherited helpers; source worktree remained read-only. Read reviewer, code-review, rust-review, rust-style and its API/implementation/safety/wire/testing references, repository AGENTS.md/CLAUDE.md/README, supplied handoff, PR draft, merged FR201 cyclic profile/AC23-26, FR282 AC18-19 and ADR028. No applicable AssuranceProfile found. No spec/plan, CI, dependency, lockfile or gate changes in this delta, so no spec-review subanalysis applies. No aggregate/make ci/Kani/full-matrix qualification or heavy build performed. Computed matrix attempt: quire matrix --scope <frozen worktree> --format json emitted module duplicate-declaration warnings but no result after several minutes; stopped without a matrix claim. Manual scoped trace/AC inspection is recorded separately below. Whole-repository matrix qualification remains unavailable. Plan completion: not assessed.

Math/S3: original enumeration uses stop_at_target=false, actual checked predicate/binding and owner terminal semantics. Original states retain separate semantic actions, even identical distributions and choices beyond targets; weighted_steps resolves workloads and refuses multi-post ambiguity. A target-stopped reachability pass removes downstream coverage. Iterative Kosaraju computes support SCCs and closed components; reverse BFS derives Prob0 and canonical shortest target distance. Targets are one, non-target terminals/closed components/no-target-support states zero; remaining normalized positive distributions satisfy exact harmonic equations. No producer graph, SCC partition, rank, policy or qualitative boundary is trusted. Complete values and ranking coverage, [0,1] probabilities, full FR196 binding/model keys, target/optimization/identity and threshold checks remain in the qualified checker. Mixed s:s/g/f with thirds solves x=(x+1)/3, hence 1/2; transient a->b, b->a/g/f at 1/2,1/4,1/4 also solves 1/2, with distances 2/1/0/0. Authored equal-weight stay/exit workload gives x=x/2+1/6, hence 1/3. Support recurrence without targets is forced zero instead of accepting arbitrary harmonic constants. No additional concrete math/S3 defect found.

Codec/dispatch: explicit in-memory cyclic_dtmc survives strict wire round-trip; objective profile is separate and exact. Reader rejects event/failures/reward/unit on the cyclic profile through profile-shape equivalences and deny_unknown_fields; writer rejects cyclic with reward/event and rejects failures without event. Existing profile emission branches retain their bytes. Explicit cyclic admission works on eligible acyclic subjects while default production tries old acyclic profile first. Shared reachability specialization counts grouped semantic actions and pending post-control, retains nominal identity and uses existing DTMC math. Generic multi-action can_avoid_target remains unchanged and belongs to QSL1845; this review grants no general cyclic MDP support. Admission-only native refutation probe returns no acceptance token, uses native state/edge/work/storage/scalar/cancel controls and does not gate on certificate transport max-bytes. Its temporary raw equality labels never escape into evidence. Authored weights are strictly positive and cover enabled operations, so all-action support used by that acyclicity probe agrees with admitted workload support; a tentative zero-weight objection was rejected after tracing the owner validation. Existing witness key_of coupling to document.bytes is inherited, not fixed here.

FND-001: cyclic.rs:30-33 and :195-198 call system.state_key before wrapper/storage admission. ModelSystem::state_key at model.rs:947-956 builds encode::model_key(...).0 under IdentityLimits::default(), without caller working-storage reservation/cancellation. The raw identity-input default is 16 MiB, separate from document working quota. Candidate/post-sort paths still call canonical_state_key (probabilistic.rs:655,698,808 and frames.rs:1547). The new bounded stopped traversal, adjacency/SCC/distance queues and full-key reservation estimates cannot constrain these prior raw allocations. Adopt the real bounded/cancellable raw/external/internal owner API and callbacks with typed causes before claiming memory qualification. Required owner spec142 is reviewed but Luna merge pending; code143 is UNBUILT at this review's handoff. This is the same concrete prerequisite as parent reward SR90/91, not a waived or merely speculative concern.

Evidence accepted without rerun: /tmp/qsl1826-cyclic-inherited.log has 38 passing fresh-process tests (6 cyclic plus 32 inherited); -solver.log has 1 passing exact single-action nominal-MDP regression; -cancel.log has 2 passing per-poll typed cancellation/every insufficient work-budget support tests; -codec.log has 1 passing explicit profile round-trip; -clippy.log and -fmt.log show successful touched all-target lint/format. These logs precede relocation/rebase at runtime head 3b2deb56b0f79e4fe00e3cd14c87867d725a3135. Compared current cyclic integration test contents to /tmp/qsl1826-cyclic-test-pre-relocation.rs: byte-identical. Current git diff --check passes. No post-relocation build is claimed; final raw-owner adoption/rebase needs affected validation. Available tests exercise real source/S3 provision and a fresh qualified-core executable, threshold misses, exact ranks/values, complete key/binding/coverage/objective mutations, equivalent versus falsifying probabilities, residual choices even beyond targets, native cyclic refusal, bytes/storage/state/edge/scalar/work/cancel stops. Passing wrapper stops do not establish the missing raw-owner bound. Optional gap semantic campaign was not separately invoked; required code faithfulness/test-intent review was performed under this dispatch.

## Examined scope and bindings

```yaml
scope:
- id: FR-201-AC-23
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A DTMC s with probabilities 1/3 each to s, target g and non-target absorbing
    f has exact eventual probability 1/2; values (s,g,f)=(1/2,1,0), ranks (1,0,0)
    accept minimum AtLeast and maximum AtMost at 1/2. Target g absorbs despite its
    original outgoing cycle. An initial target has exact value 1 and accepts minimum
    AtLeast 1/2; a non-target terminal has exact value 0 under infinite stutter and
    accepts maximum AtMost 1/2. Thresholds 3/5 AtLeast and 2/5 AtMost reject ThresholdMissed
    for the mixed case. A target-free two-state closed cycle has exact values/ranks
    0 and accepts maximum AtMost 1/2; as
- id: FR-201-AC-24
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: In a transient SCC, a goes to b with probability 1, b to a with 1/2, target
    g with 1/4 and closed failure f with 1/4. Values a=b=1/2, g=1, f=0 and ranks a=2,b=1,g=f=0
    accept; both SCC members having condensation rank 1 rejects RankBroken. A fully
    authored workload at s choosing equally between deterministic return-to-s and
    a distribution g:1/3,f:2/3 resolves to s:1/2,g:1/6,f:1/3 and accepts exact value
    1/3, rank 1 on either workload bound at equality; 2/5 AtLeast and 1/4 AtMost reject.
- id: FR-201-AC-25
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-23/24 documents round-trip canonically; wrong values, distances, missing/extra
    coverage, foreign binding, altered complete model key, model-only key, changed
    derived targets/optimization or mismatched identity prevent acceptance. Changed
    source probabilities that falsify equations reject; equivalent source changes
    may accept. A live state with stay and exit scheduler choices refuses as unsupported
    even if a producer supplies a policy or equal values, including when that state
    lies beyond an early target. Unresolved workload, fairness, clocks, rewards, other
    formulas, multi-post and multiple
- id: FR-201-AC-26
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-23/24 independently check with no solver dependency. Source/document/key
    byte, state/edge, graph work/storage and scalar/rational budgets are exercised
    at exhaustion, including SCC, reverse reachability and distance construction;
    actual typed limit and consumption are retained. Pre-cancellation and cancellation
    during each pass or exact sum stop without acceptance or truth values; no fabricated
    upstream key-budget API or cyclic refutation is claimed.
- id: FR-282-AC-18
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: Analyze writes real exact cyclic DTMC eventual documents for FR-201-AC-23/24;
    a fresh qualified-core-only executable accepts their canonical bytes from the
    bounded source provision and actual S3 subject/claim binding, including random/workload
    packages S4 refuses. It independently reconstructs original/stopped graphs, predicate/terminal
    boundaries, support SCCs, Prob0, distances and harmonic checks without analyzer/provider
    linkage, reused solver graph, emitted-package bytes or fabricated replay request.
    Wrong values/ranks/full keys/binding, changed objective/identity or source probabilities
    f
- id: FR-282-AC-19
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: AC-18 preserves typed source/hash admission failures and actual named byte/state/edge/work/storage/scalar/rational
    limits and consumption, pre-cancellation and cancellation during independent SCC/distance/equation
    passes without acceptance or truth values. Existing profiles preserve canonical
    bytes and verdicts. Raw-model-key owner budget admission and shared FR-413 witness
    wire remain open prerequisites/gaps, not newly supplied APIs or completed coverage.
bindings:
- test_id: self_loop_and_transient_scc_accept_exact_values_and_shortest_distances
  ac_id: FR-201-AC-23
  trace: correct
- test_id: self_loop_and_transient_scc_accept_exact_values_and_shortest_distances
  ac_id: FR-201-AC-24
  trace: correct
- test_id: self_loop_and_transient_scc_accept_exact_values_and_shortest_distances
  ac_id: FR-282-AC-18
  trace: correct
- test_id: target_free_recurrence_forces_zero_and_early_target_cannot_hide_scheduler_choice
  ac_id: FR-201-AC-23
  trace: correct
- test_id: target_free_recurrence_forces_zero_and_early_target_cannot_hide_scheduler_choice
  ac_id: FR-201-AC-25
  trace: correct
- test_id: target_free_recurrence_forces_zero_and_early_target_cannot_hide_scheduler_choice
  ac_id: FR-282-AC-18
  trace: correct
- test_id: explicit_cyclic_profile_admits_acyclic_dtmc_and_target_precedence
  ac_id: FR-201-AC-23
  trace: correct
- test_id: explicit_cyclic_profile_admits_acyclic_dtmc_and_target_precedence
  ac_id: FR-201-AC-25
  trace: correct
- test_id: explicit_cyclic_profile_admits_acyclic_dtmc_and_target_precedence
  ac_id: FR-282-AC-19
  trace: correct
- test_id: authored_workload_checks_its_resolved_rational_distribution
  ac_id: FR-201-AC-24
  trace: correct
- test_id: authored_workload_checks_its_resolved_rational_distribution
  ac_id: FR-201-AC-25
  trace: correct
- test_id: authored_workload_checks_its_resolved_rational_distribution
  ac_id: FR-282-AC-18
  trace: correct
- test_id: cyclic_evidence_mutations_and_caller_controls_prevent_acceptance
  ac_id: FR-201-AC-25
  trace: correct
- test_id: cyclic_evidence_mutations_and_caller_controls_prevent_acceptance
  ac_id: FR-201-AC-26
  trace: correct
- test_id: cyclic_evidence_mutations_and_caller_controls_prevent_acceptance
  ac_id: FR-282-AC-18
  trace: correct
- test_id: cyclic_evidence_mutations_and_caller_controls_prevent_acceptance
  ac_id: FR-282-AC-19
  trace: correct
- test_id: native_acyclicity_probe_uses_native_counts_and_working_storage_without_transport_bytes
  ac_id: FR-201-AC-26
  trace: correct
- test_id: native_acyclicity_probe_uses_native_counts_and_working_storage_without_transport_bytes
  ac_id: FR-282-AC-19
  trace: correct
- test_id: cyclic_profile_roundtrip_retains_its_explicit_selection
  ac_id: FR-201-AC-25
  trace: correct
- test_id: cyclic_profile_roundtrip_retains_its_explicit_selection
  ac_id: FR-282-AC-19
  trace: correct
- test_id: every_support_scc_and_distance_poll_preserves_typed_cancellation
  ac_id: FR-201-AC-26
  trace: correct
- test_id: every_support_scc_and_distance_poll_preserves_typed_cancellation
  ac_id: FR-282-AC-19
  trace: correct
- test_id: support_passes_preserve_each_exhausted_work_budget
  ac_id: FR-201-AC-26
  trace: correct
- test_id: support_passes_preserve_each_exhausted_work_budget
  ac_id: FR-282-AC-19
  trace: correct
- test_id: nominal_mdp_with_one_action_uses_exact_chain_boundaries_for_both_extrema
  ac_id: FR-201-AC-23
  trace: correct
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: actual bounded owner API and transitive key callbacks now share caller meter, cumulative storage and original cancellation in cyclic/native admission and producer candidate paths. |

## Disposition round 1 verification

Resumed the original review on final frozen c5baf8ca2cbb4aac6bd8c060f1160595cbf48a04 against actual base bfebfd678f405a8778150843dd3bfaaefadc0f5b. Reviewed all 15 changed paths (including unchanged original finding custody files) and the complete new runtime/adoption delta, relevant inherited owner/candidate helpers and both new real-owner integration tests. No new concrete finding. Original Findings, Summary, Verdict and examined scope above are retained as historical review text; this round supersedes their current blocked status. Latest verdict: PASS for the scoped cyclic DTMC implementation and original raw-key owner adoption finding; mergeable from this review's scope, subject to ordinary artifact custody/final merge handling. No whole-repository qualification or full FR-201-AC-30 claim.

FND-001 fixed: enumerate_graph now borrows one real caller Meter and cumulative storage counter through ProbabilityKeyAdmission, with original Cancel. Initial/current raw state bytes use state_key_bytes_with_budget; complete FR196 keys use for_model_state_with_budget; semantic distributions use the shared model_distributions_with_budget and actual actions_with_budget/weighted_steps_with_budget. The already merged owner routes carry the same scope through current/Undecided/candidate/post-sort keys. Wrapper construction never resets those counters. Raw identity byte policy stays distinct from document working storage and probability scalar_bits. Reservation and model integer representation/work denials preserve ProbabilityDocumentError causes; native preflight now returns the typed document envelope and document_failure maps its real cause. Native scratch uses raw equality bytes only, no certificate bytes gate or acceptance token. Native graph state/edge/scalar/storage/cancellation limits remain enforced. Shared producer CandidateBudget now applies to eventual forms, preserving one budget through build_retaining_monitor_with_budget, actual owner expansion, full keys, value slots and retained candidate rows; no nominal product identity rewrite. The wider external transition/private-reader/interpreter allocation boundary is outside the accepted narrow RAWSTATE owner scope; it is not falsely characterized as fully covered or introduced by this delta.

The cyclic mathematics, independent original admission, stopped coverage, SCC/BSCC/Prob0, normalized harmonic equality, shortest target-support distances and exact threshold checks remain as previously examined. Strict reader/writer cyclic profile dispatch and old profile branches remain intact. Original residual MDP choices stay rejected even with equal distributions/values or beyond targets. Generic multi-action can_avoid_target is unchanged/outside this PR; no general cyclic MDP or cyclic native refutation capability is claimed.

Evidence inspected, not rerun: /tmp/qsl1826-cyclic-adoption-regressions.log: 43 fresh-process tests passed, including exhaustive every-observed-poll Requested/Deadline typed cancellation (430.57 s). The roundtrip assertion now requires observed count >= selected target poll instead of exact equality, while retaining the loop over every observed poll and exact typed cancellation result: the owner legitimately polls during unwind; the assertion is not a relaxation of cancellation outcome or poll reachability. /tmp/qsl1826-cyclic-adoption-owner.log: 2 new owner-path tests pass actual raw/post/full identity ceilings, separate model integer_bits versus probability scalar_bits, zero transport-byte independence, cumulative storage refusal, work exhaustion and mid-operation native/cyclic cancellation. /tmp/qsl1826-cyclic-adoption-core.log: 5 core tests pass support-pass work/cancel and explicit profile codec controls. Touched-package all-target Clippy with warnings denied and formatting pass in -clippy.log/-fmt.log. Independent git diff --check on actual base/head passes. No heavy builds repeated because no concrete unresolved concern justified them. The owner/long-run/reward prerequisites are now merged in this base; no post-test rebase is claimed. Current main's additional weighted-long-run specification-only change does not alter this runtime delta.

Computed full matrix remains unavailable as described in the initial review; scoped acceptance text and actual trace-tagged tests were inspected directly. Additional bindings examined: cyclic_key_admission::cyclic_and_native_preserve_raw_post_and_full_key_identity_causes and cyclic_key_admission::real_owner_route_preserves_cumulative_stops_and_mid_operation_cancellation, both trace FR-201-AC-26 and FR-282-AC-19, correctly asserting their scoped owner/admission obligations. Optional repository semantic campaign not invoked. Plan completion: not assessed.

Fixed source excerpt (cyclic.rs:128-138):

```rust
            let bytes =
                system.state_key_bytes_with_budget(state, identity, &mut admission, cancel)?;
            if full_key {
                ProbabilityProductKey::for_model_state_with_budget(
                    binding,
                    &bytes,
                    identity,
                    &mut admission,
                    cancel,
                )
```
