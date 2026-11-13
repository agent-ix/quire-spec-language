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
| A run limit stopped the analysis before it completed its method | `failed`, execution `resource-incomplete` | `unavailable` | No artifact; states the limit and the value used (PA-14). |
| An engine failure | `failed`, execution `failed` | `unavailable` | |

- The replay arm shall be `ReplaySource::ProtocolAnalysis`: it re-walks the counterexample's stated paths or moves against the checked
  graph and the premises, checks each step is an admitted successor and that
  the walk ends in the stated divergence or violation. An arm that
  disagrees settles `inconclusive` (ADR-018 SM-7).
- The replay arm re-walks the stated paths with the structure's successor
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

### Open item

QSpec FR-360's Scope does not list local-projection or realizability
results. The Scope amendment is raised with QSpec as QSL-1898 and is **not
assumed**: this requirement uses the FR-360 labels for the two kinds
exactly as ADR-036 PA-20 states, and is to be re-read against FR-360 when
QSL-1898 lands. No file of the QSpec repository is changed by this
requirement.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-573-AC-1 | A sound projection and a realizable protocol each settle `proved`, `closed-scope`, execution `completed`, truth `satisfied`, certification `Uncertified`, and the settlement carries the same certification when read back. | Test (TC-1413) |
| FR-573-AC-2 | A projection counterexample and each of the two realizability refutation forms settle `refuted`, `decisive-counterexample`, execution `completed`, truth `violated`, only after the counterexample replays through `ReplaySource::ProtocolAnalysis`. | Test (TC-1413) |
| FR-573-AC-3 | A counterexample whose replay disagrees with the checked graph (a step that is not an admitted successor, or a walk that does not end in the stated divergence) settles `inconclusive`, never `refuted`. | Test (TC-1413) |
| FR-573-AC-4 | A search that completes its method up to `max_depth` or `max_history` without a counterexample settles `inconclusive`, execution `completed`, truth `pending`, basis `unsettled`; the same holds for an absent realizability premise and for a result that depended on a cut. | Test (TC-1413) |
| FR-573-AC-5 | A run stopped by `max_states`, `max_transitions` or the evaluation meter settles `failed`, execution `resource-incomplete`, truth `unavailable`, with no artifact and a record naming the limit and the value used. | Test (TC-1413) |
| FR-573-AC-6 | A FR-575 construct settles `unsupported`, execution `unsupported`, truth `unavailable`, basis `unavailable`. | Test (TC-1413) |
| FR-573-AC-7 | No `proved` result of either kind settles `Certified` or `Trusted`. | Test (TC-1413) |

## Dependencies

- ADR-036 PA-12, PA-14, PA-20; ADR-011 S6c; ADR-018 SM-7, PC-1.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md): the settlement map this follows.
- [FR-574](FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): the record a `refuted` carries.
- QSpec FR-360, FR-241, FR-242, FR-243. Open: QSL-1898 (FR-360 Scope).

## References

- ADR-036; QSL-1889, QSL-1893, QSL-1898.
