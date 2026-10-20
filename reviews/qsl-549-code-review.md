---
id: SR-13430
title: "Code review of quire-spec-language PR #1089: QSL-549 simulation-relation certificate and its core checker"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ba1d01ad278d124405c1de0a1c1e1b6df56cfb60; PR #1089 (git diff refs/remotes/origin/main...ba1d01ad2, 14 files, +1451/-83): qsl-replay/src/certificate/{simulation,check}.rs, qsl-replay/src/{certificate,lib,outcome,proof_result}.rs, qsl-replay/src/model_check/refinement_settle.rs, qsl-replay/src/refinement/outcome.rs, qsl-analyze/src/model_check/refine.rs, qsl-analyze/tests/it/{main,refinement_budget,refinement_certificate,refinement_replay,refinement_safety}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-149
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-556
    type: reviews
---
# SR-13430: Code review of quire-spec-language PR #1089

## Summary

Ticket: QSL-549 (E8c). One code-review pass with the rust-review lane
folded in, over `git diff refs/remotes/origin/main...ba1d01ad2`, judged
against the merged FR-149, FR-144 (AC-7), FR-141, ADR-018 PC-1 to PC-5 and
SM-4, and ADR-020 CT-1 to CT-3.

Gates run in a private clone at the reviewed head, with its own target dir
(`QSPEC_DIR=/Users/peter/dev/quire-specification CARGO_INCREMENTAL=0
CARGO_PROFILE_DEV_DEBUG=0`):

- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics`:
  3559 passed, 0 failed.
- root `cargo test --test it`: 975 passed, 0 failed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `make string-edge`, `make arch-lint-api-surface-qsl`,
  `make arch-lint-canonical-encoder`, `make arch-lint-qualified-core`: each
  exit 0.
- `make ci` not run (the lead runs the gate).

Probes (scratch-only test files, removed afterwards; the clone is clean at
the reviewed head):

- Mutation of `check_simulation_certificate`, running the `refinement_`
  tests: 13 mutants, 10 killed. Killed: skipping the abstract-digest
  comparison, the `SuccessorsDiffer` check, the initial-position match, the
  `Taken` comparison, the `max_transitions` stop, the `max_states` stop,
  the unreached-position `Malformed`, the `initial` length check, the
  concrete-digest comparison and the `successors` length check. Survived:
  removing the `BadState` rule, removing the cancellation poll, and
  removing the terminal stutter edge (FND-002).
- Terminal states with `max_depth` (FND-001).
- A valid certificate checked under a smaller evaluation meter (FND-003).

Checked and clean:

- Soundness. The checker cannot certify a false refinement. It takes
  indices and digests from the certificate and nothing else. Every concrete
  state it judges comes from `initial_states()` or from its own
  `successors()` call, and every verdict comes from its own
  `check_initial` and `check_step` calls. Every initial state is checked.
  Every listed position must be reached, or the certificate is `Malformed`.
  Every reached position's successors are recomputed and compared with the
  listed edges in canonical order. Every edge's post position and `Taken`
  must equal what `check_step` returned. So the relation is closed under
  the concrete successor relation from the initial states, and each step
  passes (ADR-020 CT-1 to CT-3). The engine's `Run`, its `known` map, its
  BFS queue and its dedupe are not used. `qsl-replay` cannot depend on
  `qsl-analyze`, and `arch-lint-qualified-core` passes. Sharing
  `open_systems`, `check_initial` and `check_step` with the engine is what
  FR-149 asks for, and they are core (`qsl-replay`) code, as PC-5 requires.
- `Stopped` is never `Rejected` for the named budgets: `max_states`,
  `max_transitions`, an `ExpansionStop` on the meter or candidates, and a
  `StepVerdict::Stopped`. AC-4 and AC-5 hold. FND-003 is the exception,
  for a meter that runs out inside a mapping row or a concrete contract.
- Determinism: canonical sort of (transition key, state key), `BTreeMap`
  only, no clock, path or environment reads. AC-4's twice-equal check
  passes.
- Engine side: `create` now returns the existing or new index, the
  certificate's `initial` keeps one entry per subject initial state
  (duplicates resolve to the coalesced node), and each node's edges are
  listed in the order they are judged.
- Deletion: `SimulationCertificateNotBuilt` and
  `simulation-certificate-not-built` appear nowhere in code, spec, schemas
  or docs at the head, or in quire-specification. They remain only in the
  historical `reviews/qsl-544-code-review.md`. The interim test that
  asserted the cause is deleted. No shim, alias or re-export is left.
- Settlement: `Accepted` gives `Proved{Exhaustive, Certified}`,
  `closed-scope`. `Rejected` gives `Inconclusive(CertificateRejected{rule,
  at})`, `unsettled`. `Stopped` gives V-7 through `stopped_run`. `Stale`
  gives `ReplayRefused(stale_dependency)`.
- Comment idiom: module docs state the behaviour and a "What this does not
  do yet" paragraph, matching the existing files. No build-log prose.

## Verdict

FIX. FND-001 is blocking. With `max_depth` set, the engine can return
`Holds` and write a certificate that its own core checker rejects. A true
refinement then settles `inconclusive` with `CertificateRejected` instead
of `proved`. This was reproduced. FND-002 is a test gap that should be
closed in this PR: the soundness-relevant `BadState` guard and the checker's
SM-4 stutter edge can each be deleted with every test still green. FND-003
is a spec follow-up (FR-149 maps every non-`Passes` verdict to `StepFails`)
with a code change once the spec decides. FND-004 to FND-007 cover the
coder's four listed spec gaps plus one more, and are spec follow-ups only;
the code is right to wait for them. FND-008 to FND-010 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The engine writes an invalid certificate when `max_depth` is set and a terminal state sits at the horizon. `expand` returns at `depth >= horizon` before it builds the edge list. A terminal node there sets no `horizon_cut`, so the run returns `Holds`, but that node's `successors` entry is empty. The checker always computes one SM-4 stutter edge for a terminal position, so it rejects `SuccessorsDiffer`, and FR-144 settles a true refinement `inconclusive`/`CertificateRejected`. Reproduced on `CasRefinesCounter` without `ensure`, with `pre PeekNever ... peek { false }` and `pre BeginBNever ... beginB { false }` (a 6-state chain ending at a terminal state at depth 5). `max_depth: Some(5)` gives `Holds`, position 5 has empty edges, and the check is `Rejected{SuccessorsDiffer, SimulationStep{position: 5, edge: None}}`. `None` and `Some(6)` give `Accepted`. Fix: at the horizon, record (and judge) the stutter edge of a terminal node as the non-horizon path does, so the certificate has an edge list for every node. Add a TC-556 test with a horizon at a terminal depth. | qsl-analyze/src/model_check/refine.rs:343-349; qsl-replay/src/certificate/simulation.rs:448-463 |
| FND-002 | medium | Three checker rules have no test: mutants that remove each one pass every test. (a) `BadState` (simulation.rs:434-440). This guard matters for soundness: `successors()` drops a transition whose precondition was refused (`frames.rs:164-178`, `continue`), so without the guard a certificate that leaves out that successor would be accepted. (b) The SM-4 terminal stutter edge (simulation.rs:448-452). No TC-556 fixture has a terminal state, because `peek` is always enabled. (c) The cancellation poll (simulation.rs:289-299). Add tests: a position whose expansion records `ContractUndetermined` rejects `BadState`; a fixture with a terminal state is accepted, and with its stutter edge removed is rejected `SuccessorsDiffer`; a pre-cancelled `Cancel` gives `Stopped(Cancelled{..}, None)`. | qsl-replay/src/certificate/simulation.rs:289-299; qsl-replay/src/certificate/simulation.rs:434-452; qsl-analyze/tests/it/refinement_certificate.rs |
| FND-003 | medium | A valid certificate checked under a smaller evaluation meter is `Rejected`, not `Stopped`, when the meter runs out in a mapping row or a concrete contract. Then FR-144 settles V-6 `CertificateRejected`, which PC-2 reserves for a certificate the checker found wrong. Reproduced on the light `RingIsQueue` certificate built at `UNLIMITED`. Work 1 and 5 give `Rejected{InitialMismatch, position 0}` (`check_initial` was `Undetermined`, and its verdict is dropped). Work 50 and 100 give `Rejected{StepFails, verdict: Undetermined(Mapping{cause: Evaluation(Incomplete{WorkUnits, ..})})}`. Work 20 gives `Stopped(EvaluationMeter, 20)`. By reading, a concrete contract whose meter runs out records `ContractUndetermined` with `ClauseOutcome::Incomplete`, and that rejects `BadState`. This follows FR-149's text, which sends every non-`Passes` verdict to `StepFails`, but it goes against AC-5's "Stopped, not Rejected" and PC-2's meaning. Spec follow-up: FR-149 should say that a verdict, or a `BadState`, whose only cause is an `Incomplete` (meter) evaluation stops V-7 naming `EvaluationMeter`. Then change `unsound`/`position` to match. | qsl-replay/src/certificate/simulation.rs:394-396; qsl-replay/src/certificate/simulation.rs:416-422; qsl-replay/src/certificate/simulation.rs:434-440; qsl-replay/src/certificate/simulation.rs:504-510 |
| FND-004 | low | Spec gap (coder-listed): FR-149's `CertifiedEdge.transition` is a plain `ModelTransition`, but the certificate must also list ADR-018 SM-4's terminal stutter edge, which the code writes as `transition: None` (the same encoding `ConcreteStep.transition` already uses). SM-4 says the stutter step "has its own transition identity", so the spec should name it, either as `Option<ModelTransition>` with `None` as the stutter or as a stutter identity. The code is right for now. Spec follow-up only. | qsl-replay/src/certificate/simulation.rs:65-74 |
| FND-005 | low | Spec gap (coder-listed): FR-149's `Stopped(IncompleteCause, ReachedLimit)` cannot express a cancellation, which names no limit. The code uses `Option<ReachedLimit>`. A third shape also exists: the other core checkers return `TemporalCertificateCheck::Stopped(IncompleteCause)`, with the limit inside `IncompleteCause::LimitReached{limit, value}` (check.rs:113-129), and that already covers both cases. Spec follow-up: choose one shape for all certificate checks. Preferably the existing one, and then this checker adopts it. | qsl-replay/src/certificate/simulation.rs:96-106 |
| FND-006 | low | Spec gap (coder-listed): FR-149's locus carries `verdict: Option<StepVerdict>`. The code carries `Option<Box<FailedStep>>`, which is better typed because it excludes `Passes`, `Stopped` and `Fault`. But the wire form writes only `FailedStep::kind()` (for example `"abstract-step-rejected"`), so AC-3's `transition: inc(c)` and `cause: Postcondition` reach the terminal record only in memory. No spec gives the wire form. Spec follow-up: FR-149 or QSpec FR-379 to define the serialized verdict (kind plus failure fields), and rename the type in FR-149. | qsl-replay/src/certificate.rs:139-150; qsl-replay/src/certificate/simulation.rs:109-139 |
| FND-007 | low | Spec gap, partly coder-listed: FR-149 names four rules, but the checker also needs `Malformed` and `BadState`, and neither appears in FR-149. The reused `Malformed` is documented as "found before any recomputation" (certificate.rs:57-59), but the unreached-position case is found during the scan (simulation.rs:427-429). FR-149 also says `SuccessorsDiffer` fires unless the successors are "exactly the listed edges' transitions and post-states". The code compares transitions only and reports a wrong post as `PostDiffers` after `check_step`, which is what AC-2 expects for a post index at the initial position. The two spec texts conflict. Spec follow-up: list `Malformed` (both forms) and `BadState` in FR-149, and say that `SuccessorsDiffer` compares transitions and `PostDiffers` compares posts. | qsl-replay/src/certificate.rs:57-59; qsl-replay/src/certificate/simulation.rs:427-469 |
| FND-008 | low | The checker re-implements plumbing that check.rs exists to share. `poll`, `expansion_stop`, `stopped` and the `Ended`/finish pair duplicate `check::poll`, `expansion_halt`, `stopped` and `Halt`/`finish`. check.rs's own doc says sharing them is what makes the checkers "unable to drift". The edited doc now claims only "the bound pre-check and the fault vocabulary" for this checker. The difference comes from `SimulationCheck::Stopped`'s different payload (FND-005), so aligning that shape lets this checker reuse `check::poll` and `expansion_halt`. | qsl-replay/src/certificate/simulation.rs:227-324; qsl-replay/src/certificate/check.rs:1-9 |
| FND-009 | low | Tags overstate FR-144-AC-7. `a_stale_or_stopped_check_settles_without_a_proof` is tagged `FR-144-AC-7`, but AC-7 says nothing about stale or stopped settlement: that is FR-149 Settlement and Identity. AC-7's `ensure` row (`proved`, `Uncertified`) has no test, so three tags claim an AC that is only partly covered. Separately, `a_malformed_certificate_is_rejected` is tagged `FR-149-AC-2`, which does not name `Malformed`. Retag the stale/stopped test to FR-149 (for example `TC-556` alone, or AC-4 for the stale half), and keep the AC-7 tags only on the `Certified` and `CertificateRejected` tests, with the `Uncertified` row recorded as remaining work. | qsl-analyze/tests/it/refinement_certificate.rs:608-653; qsl-analyze/tests/it/refinement_certificate.rs:322-352 |
| FND-010 | low | A checker refusal of `NoInitialState` settles `failed`, through `Err(SimulationRefusal::Subject(_)) => Row::failed()`. The closure path's `Row::of_check` settles the same refusal `Inconclusive(NoInitialState)`. It is not reachable from a `Holds` run over the same request, but the request is the caller's, so the two settlement maps disagree. Map `Subject(ClosureRefusal::NoInitialState)` as `of_check` does. | qsl-replay/src/model_check/refinement_settle.rs:62-64; qsl-replay/src/model_check/settle.rs:405-408 |

## New findings (disposition pass 1)

Round 1 reviewed the fix commit 45e3ab64a and re-ran the probes and the
surviving mutants at that head. Mutants that now fail tests: removing
`BadState`, removing the cancellation poll, removing the terminal stutter
edge, removing the meter stop on a step verdict, removing the meter stop
on `check_initial`, and reverting the engine's horizon fix. Two new
mutants still pass every test: FND-011 and FND-013 below.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | The concrete-contract branch of the meter fix has no test. Changing `if matches!(evaluation, ClauseOutcome::Incomplete(_))` to `if false && ...` passes every test, and that change would bring back the FND-003 case (a true certificate `Rejected` `BadState` when a concrete contract runs out of meter). `a_true_certificate_under_a_small_meter_never_rejects`'s doc says the sweep covers "a mapping row, a step-row argument or a concrete contract". But on the light ring every stop in the swept range comes from a mapping row or a step verdict, so the contract branch is never reached. QSL #1095's FR-149-AC-5 names the concrete-contract case explicitly. Add a fixture whose concrete precondition costs more than the checked meter, while its mapping rows fit (for example a `heavy` variant on a ring precondition), and assert `Stopped(LimitReached{EvaluationMeter, B})`. | qsl-replay/src/certificate/simulation.rs:407-413; qsl-analyze/tests/it/refinement_budget.rs:238-269 |
| FND-012 | low | Whether a position stops or rejects depends on the order of its findings. Only the first `ContractUndetermined` finding decides: if it is `Incomplete` the check stops on the meter, even when a later finding is a refusal for another reason. QSL #1095's FR-149-AC-7 says that a position at which a concrete contract is undecided "for a reason other than the meter" rejects `BadState`. Either outcome is safe, since neither accepts, but the classification is order-dependent. Reject `BadState` when any finding is not `Incomplete`, and stop only when every finding is `Incomplete`. | qsl-replay/src/certificate/simulation.rs:400-414 |
| FND-013 | low | The FND-010 fix (a `Subject` refusal settled through `Row::of_check`) has no test. Changing the refusal passed to `of_check` from `refusal` to `ClosureRefusal::Encoding` passes every test. Add one settlement test with a concrete subject that has no initial state, and assert `Inconclusive(NoInitialState)`. | qsl-replay/src/model_check/refinement_settle.rs:55-57 |

## New findings (disposition pass 2)

Round 2 reviewed f8f40d586 (on top of the merge of origin/main at acd9c1fde) against the merged FR-149 (#1095). Mutants: switching off the static unreached-position rule, the `NoInitialState` settlement and the decimal-string `position` each fail a test. Counting back-edges in `reached()` is an equivalent mutant: a mark on a position the scan has already passed has no effect. Turning the `MeterStop` arm into `BadState` passes every integration test (FND-014).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-014 | medium | The contract meter-stop path is dead code and should be deleted. A `ContractUndetermined` finding comes only from `Conjunction::Undecided` (contracts.rs:196). That is built only for `ClauseSetOutcome::Undecided`, and `combine` (clause_set.rs:102-112) returns `Incomplete` whenever any clause is `Incomplete`. `conjunction` (contracts.rs:164) then turns that into `ExpansionStop(EvaluationMeter)`, so no finding ever carries `ClauseOutcome::Incomplete`. Therefore `UndecidedState::MeterStop`, its arm in `position` and the `Incomplete` arm of `of_clause` for contract findings can never run. Mutating the arm to `BadState` passes every integration test. The unit test `a_refusal_among_meter_stops_is_a_bad_state` (tagged `FR-149-AC-7`) feeds `undecided_state` synthetic inputs that production cannot produce. FR-149-AC-5 (c) is met by the expansion stop, which `a_concrete_contract_over_the_budget_stops_the_check` covers. Fix: reject `BadState` on any `ContractUndetermined` finding, and delete `UndecidedState`, `undecided_state` and the unit test. The "or a `BadState` finding is" clause of FR-149 "Meter stops" is vacuous under FR-120; a one-line spec note to the spec lane is optional. | qsl-replay/src/certificate/simulation.rs:596-612; qsl-replay/src/certificate/simulation.rs:709-760; qsl-eval/src/simulation/model/contracts.rs:164; qsl-eval/src/value/expression/clause_set.rs:102-112 |
| FND-015 | low | `FailedStep::Undetermined`'s public `cause` and `evaluation` fields can be paired in ways the checker never produces, and the AC-8 test asserts the wire form of one such pair. `EvaluationOutcome::of_undetermined` always gives `Refused` for `UndecidedSuccessor`, and a meter-only outcome always stops rather than rejecting, so `{UndecidedSuccessor, Incomplete(EvaluationMeter)}` never reaches a locus. Test a reachable pair instead (for example `Mapping` with `Incomplete(Counter(IntegerBits))`), and build the variant only through `FailedStep::undetermined`. | qsl-replay/src/certificate/simulation.rs:178-184; qsl-analyze/tests/it/refinement_certificate.rs:980-1000 |
| FND-016 | medium | The `UndefinedEvaluation` verdict's wire form is not the one FR-149 specifies. FR-149's table says `where` and `cause` use "FR-127's `UndefinedEvaluation` encoding" (FR-127-AC-6: `cause: division-by-zero`). The code writes `where.locus` as a bare kind (`"mapping"`, `"contract"` and so on, without the row, clause or atom) and `cause` as `"kernel"`, `"family"` or `"insufficient-data"`, dropping the undefined reason. No test asserts this kind (AC-8's "each other kind" omits it). Either encode FR-127's form, with the locus identity and the undefined reason, and test it, or record it explicitly as remaining work on QSL-549. | qsl-replay/src/certificate/simulation.rs:229-237; qsl-replay/src/certificate/simulation.rs:272-292 |
| FND-017 | low | The module doc's "What this does not do yet" paragraph is stale against the merged FR-149. FR-149 now names the `None` stutter edge (`CertifiedEdge.transition: Option<ModelTransition>`) and moves package and obligation identity to FR-144 ("the checker reads the refinement node only"). The doc still says FR-149's `CertifiedEdge` does not name the stutter edge, and presents the missing identity fields as this checker's gap. | qsl-replay/src/certificate/simulation.rs:28-34 |

## New findings (disposition pass 3)

Round 3 reviewed 85274c320 (on top of the merge of origin/main at d9e3560a2). Mutants: removing the `BadState` rule fails `an_undetermined_contract_is_a_bad_state`, and forcing `UndeterminedStep`'s evaluation to `Refused` fails the AC-8 wire test.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-018 | low | Spec follow-up only. FR-149's wire table writes an `Undetermined` incomplete evaluation's `limit` as "<FR-126 limit spelling>". That is kebab case (`evaluation-meter`), and only the meter is an FR-126 limit. Another counter is written with quire-exact's `LimitKind::as_str` spelling, which is snake case (`integer_bits`, as the AC-8 test asserts), so one field mixes two vocabularies. The meter itself can never reach the wire, because a meter-only outcome always stops. So every `limit` this field ever carries is a non-FR-126 counter, which the spec does not name. Spec follow-up: say which spelling a non-meter counter uses. The code is fine as written. | qsl-replay/src/certificate/simulation.rs:118-123; qsl-analyze/tests/it/refinement_certificate.rs:1002 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (45e3ab64a): the horizon return now applies only when the node has real successors, so a terminal node at the horizon lists and judges its stutter edge. `a_horizon_at_a_terminal_state_gives_an_accepted_certificate` sweeps `max_depth` 0..=8 over the terminal chain, asserts `Accepted` at every depth that holds, and checks that depth 5 is one of them. Reverting the fix fails that test. |
| FND-002 | fixed | Round 1: `an_undetermined_contract_is_a_bad_state`, `a_terminal_state_lists_its_stutter_step` and `a_cancelled_check_stops` were added. The three surviving mutants (no `BadState`, no stutter edge, no poll) now each fail a test. |
| FND-003 | fixed | Round 1: `meter_only` and `meter_stop` turn a mapping-row or step-row `Incomplete` (from `check_initial` or `check_step`), and a concrete contract `Incomplete`, into `Stopped(LimitReached{EvaluationMeter, value})`. The spec text is in QSL #1095 ("Meter stops"). Removing the step or initial meter branch fails `a_true_certificate_under_a_small_meter_never_rejects`. The contract branch is untested: see FND-011. |
| FND-004 | deferred | Spec follow-up in QSL #1095 (spec lane, separate reviewer): FR-149 names the `None` stutter edge. No code change needed. |
| FND-005 | fixed | Round 1: `SimulationCheck` is deleted. The checker returns `TemporalCertificateCheck`, whose `Stopped(IncompleteCause::LimitReached{..} or Cancelled{..})` is the shape QSL #1095 adopts for FR-149. No `SimulationCheck` remains in the tree. |
| FND-006 | deferred | Spec follow-up in QSL #1095: FR-149 defines the StepFails verdict wire form. No code change in this round. |
| FND-007 | deferred | Spec follow-up in QSL #1095: FR-149 names `Malformed` and `BadState`. No code change needed. |
| FND-008 | fixed | Round 1: the checker now uses `check.rs`'s `poll`, `expansion_halt`, `stopped`, `Halt`, `Step`, `finish` and `Budgets`. Its own copies are deleted. |
| FND-009 | fixed | Round 1: `a_stale_or_stopped_check_settles_without_a_proof` and `a_malformed_certificate_is_rejected` are tagged `TC-556` only. The `FR-144-AC-7` tags remain only on the `Certified` and `CertificateRejected` tests. The `Uncertified` row is tracked as SR-13431 FND-001. |
| FND-010 | fixed | Round 1: `Err(SimulationRefusal::Subject(refusal))` settles through `Row::of_check(Err(refusal), ..)`, so `NoInitialState` and `NoInstance` settle as they do on the closure path. It is untested: see FND-013. |
| FND-011 | rejected | Round 2 (f8f40d586): the finding's premise was wrong. The branch it asked to test cannot run, because FR-120 turns an `Incomplete` contract into an expansion stop before any finding exists (contracts.rs:164, clause_set.rs:102-112), so no test can reach it. The FR-149-AC-5 (c) behaviour it was about is now tested through the live path, `a_concrete_contract_over_the_budget_stops_the_check`. The dead branch that remains is FND-014. |
| FND-012 | rejected | Round 2: same premise as FND-011. A position never has a meter-only contract finding, so there is no order-dependence to fix. The `undecided_state` helper added for it is dead code (FND-014). |
| FND-013 | fixed | Round 2: `a_subject_with_no_initial_state_settles_no_initial_state` asserts `Inconclusive(NoInitialState)`. Swapping the refusal passed to `of_check` now fails it. |
| FND-014 | fixed | Round 3 (85274c320): `UndecidedState`, `undecided_state` and its unit test are deleted. Any `ContractUndetermined` finding now rejects `BadState`, and a comment states why (FR-120 stops an incomplete clause set). Removing the rule fails `an_undetermined_contract_is_a_bad_state`. |
| FND-015 | fixed | Round 3: `FailedStep::Undetermined(UndeterminedStep)` has private fields and only `UndeterminedStep::new(cause)`, which derives the evaluation, so only producible pairs exist. The AC-8 test now uses `Mapping` with an `IntegerBits` `Incomplete`, and `UndecidedSuccessor` with `Refused`. |
| FND-016 | deferred | Round 3: recorded as remaining work on QSL-549 in the module doc ("FR-127's full encoding of them is remaining work on QSL-549"). The PR body was not checked, under the Linear-only rule. The wire form stays kind-only until then. QSL-549 stays open. |
| FND-017 | fixed | Round 3: the module doc now says package and obligation identity is FR-144's, that the checker reads the refinement node only, and that `HistoryValues` and FR-127's encoding are not carried. The stale stutter-edge sentence is gone. |
