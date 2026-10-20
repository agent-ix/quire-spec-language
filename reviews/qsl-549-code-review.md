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
