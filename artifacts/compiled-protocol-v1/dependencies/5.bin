# Observation result consumer and output-mapping contract

Draft F-owned consumer boundary for OB03 and the composed `1-draft` baseline.
It reconciles structured observation results with executable and export consumers.
It does not define an IR type, target-language mapping profile, model schema,
canonicalizer, CLI, backend, parser, or evidence store. Native Quire remains the
only editable clause authority.

## Input identity and per-obligation disposition

A consumer receives a result for one requested assessment subject. For every
obligation in that subject, including an obligation the consumer cannot represent,
the input retains:

| Value | Required meaning |
| --- | --- |
| Subject and request | Exact linked-package/static subject, runtime assessment-input identity, requested capability, selected observation binding, and separately identified decision scope and surrounding execution. |
| Source correspondence | Native clause, source revision/digest/span, declaration/anchor/capture identities, and every selected definition/profile identity, revision, and digest. |
| Result facts | Independent B-owned assessment execution, truth, settlement basis, exact decision support, activation, participation, completeness, adequacy, claim strength, assumptions, and provenance facts; no summary may erase another fact. |
| Decision-scope progress | Exact progress for the obligation's decision scope, retaining its scope, authority and boundary identities independently of surrounding-execution progress. |
| Decision-scope closure | Exact closure state and premises for that decision scope, independently of its progress, assessment execution, completeness and surrounding-execution closure. |
| Surrounding-execution progress | Exact progress for the surrounding execution, retaining its own scope, authority and boundary identities; decision-scope progress cannot replace it. |
| Surrounding-execution closure | Exact closure state and premises for the surrounding execution; closing or settling one decision scope cannot close that execution. |
| Complete-global-conformance closure | B's fifth typed closure record: `closed`, `open`, `incomplete` or `contradicted` with exact required execution, branch and workflow closure-premise identities for a global-conformance claim; `not-required` with no global premise set for another claim. Per-obligation settlement or another closure axis cannot replace it. |
| Historical supersession | Earlier result identity and immutable bytes plus the linked superseding/invalidating result, contradicted premise, and corrected input identity; no result is restamped or silently replaced. |
| Consumer selection | Consumer identity/version, selected target/output profile, configuration, and declared supported capability. |
| Mapping outcome | One of `preserved`, `conditional`, `unrepresented`, or `refused`, with an exact source-bound reason and any required premise or loss. |

The consumer preserves all four mapping outcomes independently from the observation
availability state. A result that is pending, incomplete, refused, untriggered,
unsupported, or not executed remains that state at the boundary; it cannot be
converted to a Boolean pass, a mapped success, or a qualified run. A settled
truth on an open subject retains its selected decisive basis and exact support;
the open surrounding execution is not a reason to erase either fact. Definition
or profile identity/revision/digest drift refuses rather than using a current
default.

The four decision-scope/surrounding-execution progress/closure axes and the fifth
complete-global-conformance closure record retain
[B's FR-061 meaning](../../spec/functional/FR-061-report-orthogonal-results.md).
The consumer cannot merge them into one subject state or use assessment completion
or observation completeness as a closure substitute. Omitting a required retained
axis or cross-wiring it to another axis or scope refuses the inconsistent
contribution. Explicit open/incomplete surrounding-execution facts are retained
without erasing a settled truth whose exact deciding support is complete.
Compatible changes to surrounding-execution progress or closure preserve that
truth, settlement basis and support. Complete global success requires the fifth
record to be `closed` with every exact required premise under the selected B
contract. Omitted, duplicated, cross-wired or scope-mismatched records/premises
refuse. A global premise under `not-required`, a global claim marked
`not-required`, or another axis replaced with `not-required` also refuses.

## Mapping outcome rules

- **preserved** means the selected consumer contract represents the named
  obligation and every required selected premise. It is not a claim that a foreign
  parser accepted text, a target executed, or an external tool agreed.
- **conditional** means the output retains an identified premise outside the
  selected consumer contract. The premise, source identity, and missing
  correspondence remain visible.
- **unrepresented** means the selected target profile has no representation for
  the obligation or an essential selected fact. The original obligation remains
  listed; it is not silently omitted.
- **refused** means input identity, capability, profile, mapping selection, or
  resource admission prevents deterministic output. Retain the typed reason and
  affected identity; do not emit a plausible substitute.

A target syntax parser, serializer, or foreign-tool acceptance check is only a
structural observation. It does not promote any outcome to preserved or establish
semantic equivalence.

## Consumer restrictions

Codegen/executable consumers may consume this envelope only under their declared
capability and must not become a clause authority, recreate B's result/evidence
family, infer missing observations, or replace D/E-owned mapping semantics.
Exporters may emit OCL, SysML v2/KerML, or FRETish text as outputs, but no
foreign runtime is required for generation or qualification. Every first-party
executable adapter, reader, loss reporter, fixture audit, and test path is Rust
1.98.1.

The existing codegen `analyze_bound_coverage` output is an immutable,
`provenance: unqualified` domain observation over an already bound generated
bundle. A consumer may retain it as one input fact with its own codegen identity,
but it cannot establish observation completeness, activation, participation,
adequacy, native execution, authenticated producer provenance, or obligation
discharge. The planned aggregate native-run analysis and codegen CLI remain
separate parked work; this contract does not authorize either implementation.

## Planned controls

For one common O1/O2 assessment, map a preserved obligation and independently
exercise conditional capture/clock correspondence, an unrepresented history or
collection feature, a refused stale profile, pending/incomplete/refused result
states, and a parser-accepted generated-text control. The controls retain their
source/binding/model/profile identity and prove no target parse or serialization
result becomes a semantic or execution claim. Deliver both decisive-witness
satisfaction and decisive-counterexample violation with open decision scope and
surrounding execution; vary compatible surrounding-execution facts while retaining
the same deciding support. Independently omit and cross-wire each decision-scope
progress, decision-scope closure, surrounding-execution progress and
surrounding-execution closure axis; independently duplicate each required axis.
Preserve all four typed global closure states and the non-global `not-required`
record with no premise set. Independently omit, duplicate or cross-wire the fifth
record and each global premise; add a premise under `not-required` or substitute
`not-required` for a global claim or another axis. No invalid record/premise
becomes global success. These controls are planned in
[TC-145](../../spec/test-cases/TC-145-report-observation-adequacy.md),
[TC-146](../../spec/test-cases/TC-146-map-observation-results-to-consumers.md)
and [IT-051](../../spec/integration/IT-051-observation-output-consumer-contract.md).
