---
id: SR-202610310155
title: "Code review — QSL-563 protocol refinement"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/model_check/refine.rs; qsl-replay/src/model_check/refinement_replay.rs; qsl-replay/src/refinement/failure.rs; qsl-replay/src/refinement/protocol_system.rs; qsl-replay/src/refinement/step.rs; qsl-analyze/tests/it/refinement_protocol.rs; FR-148-AC-12; FR-142-AC-7"
review_set: subset
---

## Summary

Reviewed the Rust implementation diff for QSL-563 PR #2034, including the protocol assumption-relation and replay paths. One high-severity FR-148 gap remains: protocol-side undetermined fairness is returned as a generic undecided-successor result rather than the required typed relation witness.

## Verdict

**FAIL** — FR-148-AC-12's protocol-side `ContractUndetermined` case loses its exact fairness identities and paired finite witness, so it cannot be replayed as specified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Protocol-side undetermined fairness is collapsed to generic `UndecidedSuccessor`, omitting the typed relation witness and replay data required by FR-148-AC-12 | qsl-analyze/src/model_check/refine.rs:682-689 |

## Review detail

The code and Rust lanes covered the checked protocol adapter, candidate-specific protocol product, refusal and terminal-success relation, assumption-relation search, replay validation, and the AC-linked integration fixtures. No applicable AssuranceProfile covers these changes; AD-001 concerns the shared evaluation-core interface and is outside this diff. The feature does not alter CI workflow gates.

`protocol_divergence` returns `SafetyUndecided::Undetermined(UndecidedSuccessor)` whenever a protocol successor expansion contains `ContractUndetermined` (lines 682-689). `protocol_assumption_strengthening` makes the same conversion (lines 877-884). The specification requires the protocol-class case, like the concrete-class case, to return `ProtocolAssumptionNotEstablished{reason: relation-undetermined}` with exact identities and both finite prefixes, and for replay to recompute the predicate. The sole AC-12 test exercises a concrete assumption predicate. A protocol-side fairness predicate therefore never reaches the typed witness/replay path.

## Coverage

- Code review and Rust review: ran together in this artifact.
- Static checks independently run: `cargo fmt --all -- --check` and `git diff --check` passed.
- Coder-reported evidence: `refinement_protocol::` 23/23, required-clause subset 2/2, and `cargo check --no-default-features -p qsl-analyze -p qsl-replay` passed. I did not run those builds independently.
- Aggregate `make ci` was not run in this review pass; the code-lead's gate order runs it after review findings are fixed.
- No CI workflow files changed. No applicable AssuranceProfile was found.

## Findings data

```yaml
clean: false
scope:
- id: FR-148-AC-12
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: 'When an identified concrete or protocol fairness class has ContractUndetermined
    enabledness at a reachable paired state and no violating paired lasso is available,
    the product returns ProtocolAssumptionNotEstablished{reason: relation-undetermined}
    with the exact checked identities and both finite prefixes to the undecided state;
    FR-145 replay reproduces the same undecided predicate.'
- id: FR-142-AC-7
  path: spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md
  role: context_only
  excerpt: CasRefinesCounter with a true poll returns Stopped{Cancelled, None}.
- id: protocol-assumption-relation
  path: qsl-analyze/src/model_check/refine.rs
  role: examined
  excerpt: Protocol-side ContractUndetermined findings currently return generic UndecidedSuccessor
    from protocol_divergence and protocol_assumption_strengthening.
- id: protocol-assumption-witness
  path: qsl-replay/src/refinement/failure.rs
  role: examined
  excerpt: ProtocolAssumptionWitness carries a typed reason, exact source assumptions,
    paired path, and optional undecided predicate locus.
- id: protocol-assumption-replay
  path: qsl-replay/src/model_check/refinement_replay.rs
  role: examined
  excerpt: replay_protocol_assumption_witness recomputes the predicate and source
    identities for relation-undetermined witnesses.
- id: TC-553-AC-12-test
  path: qsl-analyze/tests/it/refinement_protocol.rs
  role: examined
  excerpt: an_undetermined_assumption_predicate_has_a_typed_finite_witness covers
    the concrete fairness class case.
- id: FR-379-AC-8
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: Over a protocol abstract subject, a stable concrete state that refuses
    a label every stable member of the candidate set enables fails `RefusalNotPermitted{position,
    label}` with `position` the prefix's step count, and the reported `label` is the
    least qualifying label by canonical encoding.
- id: FR-379-AC-9
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: '`RefusalNotPermitted` is not reported when some stable member of the candidate
    set refuses the label, when the candidate set has no stable member, when the concrete
    state enables a `stutter` or `any` step, or when a concrete step visible with
    the label is enabled.'
- id: FR-379-AC-10
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: 'A terminal-success concrete state with no terminal-success member of the
    candidate set fails `TerminalSuccessMismatch{position, direction: concrete-only}`,
    and a terminal non-intended concrete state with every member terminal success
    fails it with `abstract-only`.'
- id: FR-379-AC-11
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: '`TerminalSuccessMismatch` is not reported when some member of the candidate
    set is terminal success (`concrete-only`), when some member is not (`abstract-only`),
    or when the concrete state enables a step.'
- id: FR-379-AC-12
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: A failing step rule on the incoming step is reported instead of a protocol
    relation failure at its post-state, and where both relation kinds fail at one
    state `RefusalNotPermitted` is reported; a model abstract subject never yields
    either kind.
- id: FR-379-AC-13
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: Replay of `RefusalNotPermitted` or `TerminalSuccessMismatch` recomputes
    the candidate set and the classifications at the prefix's last state and settles
    `reproduced-with-evaluated-witness` and the item `refuted` when the recorded failure
    holds; a recorded `position` that differs from the prefix's step count, a label
    no stable member enables or the concrete state does not refuse, a `direction`
    that does not hold, or no recomputed failure settles `inconclusive`, cause `replay-parity`;
    a loop on these kinds is refused `invalid_runtime_input`/`inva
- id: TC-318
  path: quire-specification/spec/test-cases/TC-318-protocol-refinement-step-rules.md
  role: context_only
  excerpt: Case 4 refusal, case 5 permitted refusal, cases 6-7 terminal mismatch,
    case 9 replay parity; each settles FR-379-AC-8 to AC-13.
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: code-review
  check: soundness
  artifact_id: FR-148-AC-12
  related_ids:
  - FR-145
  - FR-148
  path: qsl-analyze/src/model_check/refine.rs
  lines: 682-689
  excerpt: 'When an identified concrete or protocol fairness class has ContractUndetermined
    enabledness at a reachable paired state and no violating paired lasso is available,
    the product returns ProtocolAssumptionNotEstablished{reason: relation-undetermined}
    with the exact checked identities and both finite prefixes to the undecided state;
    FR-145 replay reproduces the same undecided predicate.'
  finding: Both protocol_divergence and protocol_assumption_strengthening convert
    a protocol successor's ContractUndetermined finding to the generic UndecidedSuccessor
    result. This omits the exact fairness identity and paired finite prefixes required
    for a typed relation-undetermined witness, so FR-145 cannot reproduce this protocol-side
    AC-12 case. The only AC-12 fixture currently covers the concrete-side predicate.
bindings: []
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Rechecked at d4f1c44dcafda8546eef4c0a074f5820be82111e. |

### FND-001 after excerpt

```text
if let Some(witness) = self.protocol_contract_undetermined_witness(
    node_index, candidate, protocol, &transition, &clause, None, None,
)? {
    return Err(End::Undecided(SafetyUndecided::ProtocolAssumptionNotEstablished(
        Box::new(witness),
    )));
}
// an_undetermined_protocol_assumption_has_a_typed_finite_witness asserts
// RelationUndetermined, selected RequiredTemporal identity, and replay Reproduced.
```
