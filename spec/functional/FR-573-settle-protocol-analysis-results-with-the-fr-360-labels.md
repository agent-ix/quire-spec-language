---
id: FR-573
title: "Settle protocol analysis results with the QSpec FR-360 labels"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-570
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-571
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-574
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-360
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-241
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-242
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-243
    type: depends_on
---
# FR-573: Settle protocol analysis results with the QSpec FR-360 labels

## Description

QSL SHALL settle every projection (FR-570) and realizability (FR-571)
result with one QSpec FR-360 label, through the FR-241, FR-242 and FR-243
combination FR-360 names, by the existing native-engine rule and with no
rule of its own. It implements ADR-036 PA-12, PA-14 and PA-20 (ADR-011 S6c;
ADR-018 SM-7, PC-1).

## Inputs

A projection or realizability result, or a stop, from FR-570 or FR-571.

## Outputs

One settlement per item: the label, the certification of a `proved`, the
basis, and for a `refuted` the replay result of its counterexample.

## Behavior

The settlement map is:

| Result | Label | Basis | Notes |
| --- | --- | --- | --- |
| Sound projection (witness) | `proved` | `closed-scope` | Certification `Uncertified`: a first-party engine with no core checker (ADR-018 PC-1). |
| Realizable, exploration closed, replay gate passed | `proved` | `closed-scope` | `Uncertified`. |
| Projection counterexample, or either realizability refutation form | `refuted` | `decisive-counterexample` | Settles only after the counterexample replays (below); carries the FR-574 record. |
| Search reached `max_depth` or `max_history` without a counterexample, absent role-knowledge or environment premise, a cut that decided the result, or an unforced refutation | `inconclusive` | `unsettled` | Never `proved`. |
| A construct of FR-575 | `unsupported` | `unavailable` | Names the located cause. |
| A run limit stopped the analysis before it completed its method | `failed`, execution `resource-incomplete` | `unavailable` | No witness or counterexample; states the limit and the value used (PA-14). |
| An engine failure | `failed`, execution `failed` | `unavailable` | |

- The replay arm re-walks the counterexample's stated paths or moves against
  the checked graph and the premises, checks each step is an admitted
  successor and that the walk ends in the stated divergence or violation. An
  arm that disagrees settles `inconclusive` (ADR-018 SM-7). For a projection
  it is `ReplaySource::ProtocolAnalysis` over the checked package. For a
  realizability refutation the arm has no replay source, because a replay
  source carries no protocol subject: it walks the `qsl-eval` arena of the
  request's protocol subject. Every play names the index of its initial
  state and of each step's successor in the system's canonical orders, so
  the walk is determinate; a step whose transition differs from the one at
  its index disagrees. Both forms carry the request's key and the FR-574
  record the request's root gives, and every play stays within `max_depth`.
  Every play passes only through states with no finding, and each role
  step on it is its role's only enabled action there, so no table can
  avoid it (FR-571: thread order is the environment's, a role chooses only
  among its own steps). For indistinguishable histories the arm checks that
  neither end carries a finding; that the role observes the two ends
  equally, within `max_history` at an end the role can still step from;
  that the two actions differ and each is enabled at its own end (`End` at
  an intended terminal state); that the obligation is `finish`; and that no
  action is enabled at both ends (PA-18 (i)): the environment may let the
  role move at either end, and it must then take its one committed action.
  At a forced pair no node is lost, so the enabled sets are the not-lost
  sets. For an environment defeat it checks that the play ends in a
  violated invariant of the clause or a deadlock against `finish`, as the
  counterexample names. Each engine runs its arm before it returns a
  refutation.
- For a projection, the replay arm re-walks the stated paths with the structure's successor
  relation and recomputes the actions' shapes, the role's basis at the choice
  and the diverging layer with the same structure functions the engine uses.
  It is independent of the engine's search, so it catches a counterexample
  that was edited or stale, not an error in the shared classification or merge
  rules; those are covered by their own tests (TC-1410).
- No `proved` shall carry certification `Certified` or `Trusted` while no
  independent witness checker exists.
- QSL shall apply the FR-360 labels to these two kinds exactly as FR-360
  names them; QSL shall add no label, axis member or `NegotiationCause`
  variant.

### Serving

- One provider manifest, backend `protocol-analysis`, advertises
  `local-projection` and `realizability`, each in the `unbounded` mode: a
  `bounded-static` protocol is decided without a bound for any domain, and
  `max_depth` is a key member whose cut settles `inconclusive`. It names no
  `domains` and no evidence kind, which FR-290 assigns to
  `probabilistic-satisfaction` alone. The linked engine set holds it.
- An `analyze` item of either kind settles one FR-331 terminal record by
  this map, and carries the result: its FR-572 key and the witness or the
  counterexample with its FR-574 record. A refusal before analysis (an
  absent key member or root, an unknown profile or role, a premise holding a
  fact) is `declined` `invalid-input` and carries no result; a run limit or
  the evaluation meter that stops the analysis is the result's own `failed`
  outcome, `resource-incomplete`, naming the limit or counter and its value;
  any other engine failure is `failed` and carries no result.
- The FR-331 terminal value of each row: `proved` with basis `exhaustive`,
  `uncertified`; `refuted`; `inconclusive` with cause `bound-reached` and
  the `max_depth` for the horizon, and otherwise `premise-not-stated`,
  `replay-disagreed`, `unforced-refutation`, `strategy-replay-failed`,
  `cut-count` or `undecided-evaluation`; `unsupported` with the FR-575 slug
  under `unsupported_projection`; `incomplete` `limit-reached` naming the
  limit and its value.

### FR-360 Scope

QSpec FR-360's Scope lists neither local-projection nor realizability
results. QSL applies the FR-360 labels to both kinds as this requirement
states, and reads FR-360 as covering them once QSpec's Scope says: "It also
covers local projection and realizability results (FR-174, FR-175)."
QSL-1898 carries that QSpec change; no file of the QSpec repository is
changed by this requirement.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-573-AC-1 | A sound projection and a realizable protocol each settle `proved`, `closed-scope`, execution `completed`, truth `satisfied`, certification `Uncertified`, and the settlement carries the same certification when read back. | Test (TC-1413) |
| FR-573-AC-2 | A projection counterexample and each of the two realizability refutation forms settle `refuted`, `decisive-counterexample`, execution `completed`, truth `violated`, only after the counterexample replays through its arm: `ReplaySource::ProtocolAnalysis` for a projection, the arena walk for realizability. | Test (TC-1413) |
| FR-573-AC-3 | A counterexample whose replay disagrees with the checked graph (a step that is not an admitted successor, or a walk that does not end in the stated divergence) settles `inconclusive`, never `refuted`. | Test (TC-1413) |
| FR-573-AC-4 | A search that completes its method up to `max_depth` or `max_history` without a counterexample settles `inconclusive`, execution `completed`, truth `pending`, basis `unsettled`; the same holds for an absent realizability premise and for a result that depended on a cut. | Test (TC-1413) |
| FR-573-AC-5 | A run stopped by `max_states`, `max_transitions` or the evaluation meter settles `failed`, execution `resource-incomplete`, truth `unavailable`, with no witness or counterexample and a record naming the limit and the value used. | Test (TC-1413) |
| FR-573-AC-6 | A FR-575 construct settles `unsupported`, execution `unsupported`, truth `unavailable`, basis `unavailable`. | Test (TC-1413) |
| FR-573-AC-7 | No `proved` result of either kind settles `Certified` or `Trusted`. | Test (TC-1413) |

## Dependencies

- ADR-036 PA-12, PA-14, PA-20; ADR-011 S6c; ADR-018 SM-7, PC-1.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md): the settlement map this follows.
- [FR-574](FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): the record a `refuted` carries.
- QSpec FR-360, FR-241, FR-242, FR-243. Open: QSL-1898 (FR-360 Scope).

## References

- ADR-036; QSL-1889, QSL-1893, QSL-1898.
