---
id: SR-20261109172912000106
title: QSL-1826 cyclic DTMC implementation code-review
type: SpecReview
analysis: code-review
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
| FND-001 | high | Cyclic graph admission and the native acyclicity probe construct raw model keys and transitive candidate/post-sort keys before caller working-storage reservation and without raw-key cancellation controls. | qsl-replay/src/probability_certificate/cyclic.rs:30-33,195-198 | implementation-bug-despite-evidence |

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
