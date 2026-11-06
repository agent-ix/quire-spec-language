---
id: FR-052
title: "Preserve temporal evaluation ownership on the checked-package route"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-044, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-051, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-390, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-449, type: references }
---
# FR-052: Preserve temporal evaluation ownership on the checked-package route

## Description

When a checked temporal protocol obligation is evaluated through the Layer-4
checked-package route, the QSL evaluator SHALL remain the sole owner of its
temporal truth, settlement and decision support, with outcomes and failures
consistent with FR-043 and activation inputs consistent with FR-044.

## Behavior

The replacement route SHALL derive temporal truth, settlement and decision
support from the selected checked obligation and admitted evaluation evidence.
It SHALL NOT accept caller-authored truth, settlement or support as evaluation
inputs or treat a leaf valuation as the formula result.

The route SHALL preserve the selected obligation identity, temporal definition
and profile/clock authority, activation state, captures, evaluation anchor,
progress and closure evidence, execution disposition and completeness evidence
where those facts are present in its checked-package evaluation contract.
Independent facts SHALL remain independently inspectable; a missing or
unestablished fact SHALL NOT be converted to false, settled, closed or complete.

If required evidence is missing, inconsistent, unsupported by the selected
profile, or exceeds an applicable work limit, the evaluator SHALL return the
corresponding typed incomplete, refused or unsupported outcome without a
partial temporal truth. It SHALL preserve each independently settled sibling
obligation when another obligation cannot be completed, as required by FR-043.

Where the checked-package route accepts a correction or revised evaluation,
the evaluator SHALL retain the exact direct predecessor and immutable
predecessor result identity, reject a self-edge or non-direct/mismatched
predecessor, and leave prior result content unchanged. This clause applies only
to a correction relation actually carried by that route.

These obligations specify behavior on checked-package protocol evaluation
(FR-390) and its Layer-4 activation mapping (FR-449). They do not require a
separate native-temporal request/result document. The typed request and result
that carry them for quire-protocol are FR-516's; they are Rust values with no
document, schema or QSL-computed content digest (the observation owner's
identifiers on `EvidenceRef` are opaque and echoed).

For the QSL-owned native `Report` route (the value `temporal::evaluate_v2`
returns), the following hold.

*Binding.* The report SHALL carry the checked subject it was evaluated for: the
package identity and version, the clause `CheckedNodeId`, the declaration
occurrence key `{ role, ordinal }` (FR-515), and the clause's source-map entry
(FR-515 `source_span`). It SHALL carry the one opaque observation reference and
revision reference the evaluation was given (FR-043), and the FR-043 truth,
settlement, support and typed outcome. QSL treats the observation owner's
references as opaque: it defines no revision or capture-provenance shape and
derives no identity from bytes, hashes or digests.

*Decision-support identities.* The report SHALL carry its support as a
trace-index relation: for each support entry, the admitted trace position
ordinal and the FR-043 valuation-evidence row key (subject, position, anchor,
capture instance, revision, operand) it rests on, in the order FR-043 evaluates
them. Every support entry SHALL name a row present in the evidence the
evaluation admitted, and a repeated row appears as often as FR-043 retained it.
A support position index with no row, or a row the evaluation did not admit,
cannot occur in a report.

*Owner representation.* The report SHALL be immutable, with no mutator, and its
owner representation SHALL be `Report::to_owner_bytes(&self) -> Vec<u8>` and
`Report::from_owner_bytes(bytes: &[u8], subject: &CheckedSubject,
observation: &ObservationRef) -> Result<Report, ReportRefusal>`. The bytes are a
deterministic, self-delimiting encoding: a format tag and version, then the
binding, the outcome, the support relation and the optional predecessor in a
fixed order, each variable-length member length-prefixed and each enumeration
carried as a closed tag. The encoding carries no hash, digest or QSL-computed
content identity. The reader is the only constructor of a `Report` other than
the evaluator, and it SHALL admit strictly: it refuses an unknown format tag or
version (`UnsupportedVersion`), a truncated input (`Truncated`), bytes after the
last member (`TrailingBytes`), an unknown enumeration tag (`UnknownTag`), a
member out of order or repeated (`OutOfOrder`), a binding that differs from the
supplied subject or observation reference in any dimension
(`BindingMismatch`, naming the dimension), and a support entry whose row is not
in the relation or whose position ordinal is not a position of the report
(`SupportNotInTrace`). `ReportRefusal` is a closed enum and a refusal returns no
partial report. Writing then reading a report returns an equal report.

*Predecessor.* A report on a correction carries its direct predecessor as that
predecessor's FR-043 result identity (FR-043-AC-13), and the reader applies the
same predecessor rule as AC-12. QSL's rule applies only to a correction relation
carried by this native route; it grants no authority over an IR bridge join's
predecessor or supersession lineage, which Contract IR owns.

This native report contract is distinct from FR-516's quire-protocol
request/result types.

FR-043 owns temporal meaning; FR-044 owns activation and captures. The retired
M-6d request/result API criteria remain historical and do not retire these
behaviors.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-052-AC-9 | For a checked temporal obligation, the Layer-4 evaluation result's truth, settlement and support equal the FR-043 evaluation of the same selected obligation and admitted evidence; no input permits caller-supplied truth, settlement or support to determine that result. | Test |
| FR-052-AC-10 | For each independent activation, capture, anchor, profile/clock, progress, closure, execution and completeness fact present on the route, the result retains that fact distinctly; missing required evidence yields a typed non-truth outcome and never a false/default/closed/complete value. | Test |
| FR-052-AC-11 | Missing, inconsistent, unsupported or over-budget evidence produces the corresponding typed outcome with no partial truth, while an independently settled sibling retains its FR-043 result. | Test |
| FR-052-AC-12 | Where the route accepts a correction relation, a valid correction names its exact direct predecessor and changed evaluation input; self, non-direct, cross-obligation, cross-occurrence (a predecessor of another declaration occurrence of the same checked declaration) or identity-mismatched predecessors refuse, and the predecessor result remains unchanged; on the native `Report` route the predecessor is named by FR-043 result identity. This criterion does not authorize bridge predecessor or supersession lineage, which Contract IR owns. | Test |
| FR-052-AC-13 | Where the route carries a progress ledger, a regressing watermark or a conflicting completeness revision under one binding is refused as FR-502 states, leaves the retained progress and every earlier result unchanged, and a foreign clock, clause, package or profile settles nothing for the binding (the spine successor of FR-043-AC-12). | Test (TC-144, TC-1296) |
| FR-052-AC-14 | A checked temporal obligation selected under the timestamped-event finite-window profile is evaluated as FR-501 states and is never refused for its profile. | Test (TC-144, TC-1295) |
| FR-052-AC-15 | An evaluation through `temporal::evaluate_v2` returns a `Report` whose subject (package identity and version, clause `CheckedNodeId`, occurrence key, source-map entry), observation and revision references, truth, settlement, support and typed outcome equal those of the evaluation; two occurrences of one declaration sharing a node return reports differing in the occurrence key. | Test |
| FR-052-AC-16 | Every support entry of a report names a trace position ordinal and a valuation-evidence row key present in the admitted evidence; for an evaluation with three identical rows at one position and one row at another, the support relation lists the rows with that multiplicity, and no support entry exists for a position or row the evaluation did not admit. | Test |
| FR-052-AC-17 | `from_owner_bytes(to_owner_bytes(r), subject, observation)` returns a report equal to `r`, including support multiplicity and predecessor, and `to_owner_bytes` is byte-identical across two calls and two builds of the same evaluation. | Test |
| FR-052-AC-18 | Each single mutation of valid owner bytes is refused with the named `ReportRefusal` and returns no report: an unknown version or format tag (`UnsupportedVersion`), a prefix of the bytes (`Truncated`), one appended byte (`TrailingBytes`), an unknown outcome tag (`UnknownTag`), two members swapped (`OutOfOrder`), a different supplied subject, occurrence key or observation reference (`BindingMismatch` naming the dimension), and a support entry naming a row or position the report does not hold (`SupportNotInTrace`). | Test |

## Dependencies

- [FR-043](./FR-043-evaluate-bounded-native-temporal.md) owns temporal evaluation semantics and outcomes.
- [FR-044](./FR-044-activate-temporal-obligations.md) owns activation identity, guards and immutable captures.
- [FR-390](./FR-390-evaluate-checked-protocol-slots.md) owns checked-package protocol-slot evaluation.
- [FR-449](./FR-449-derive-protocol-obligation-activation-bindings.md) owns checked-package activation mapping.
- [FR-051](./FR-051-publish-checked-native-handoffs.md) is historical context for the retired native temporal subject handoff.
