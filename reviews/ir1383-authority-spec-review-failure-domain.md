---
id: SR-20261106200938
title: failure-domain review of same-emission monitor authority
type: SpecReview
analysis: failure-domain
scope: agent-ix/quire-spec-language; IR-1383; spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md;
  spec/test-cases/TC-1081-monitor-builds-from-the-plan-alone-and-refuses-tampered-or-unsupported-plans.md
review_set: subset
---

## Summary

Ticket IR-1383, PR 2480, source head 44cdf1333. Reviewed the two changed Markdown artifacts with the spec-review/failure-domain method.

## Verdict

**PASS** — No findings.

The owner trust boundary refuses missing premise, owner refusal and independently mismatched subject/source/clock/model/time/claim without a partial context. Equal names, labels, indices, document bytes or content-equal foreign packages cannot establish instance-bound authority. Structural node identity, clause role/ordinal and source are preserved. Exact byte limits and one-less controls prevent a partial canonical value. Owner-permitted clone/rename behavior retains the original association instead of reconstructing a second emission.

No applicable active AssuranceProfile was found under spec/. The dispatch selected base, integrity, failure-domain and dependency analyses. No enforced human-gated workflow applies. This review claims no implementation, computed matrix or runtime qualification credit; no Cargo or aggregate final gate ran.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined Scope

```yaml
scope:
- id: FR-251-AC-42
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: context_only
  excerpt: The `Emission` returned with a plan-root package holds, non-empty wherever
    the acquisition had rows, the selected model declaration rows, selected model
    field rows, source-unit rows, model enum and source enum declaration and member
    rows, source union rows, source record rows, tuple declaration rows, operation-member
    rows, equality producer rows, equality fact rows, temporal binding rows and omitted
    nodes of that acquisition. Every node id in each of them (anchor, structural node,
    record node, declaration node, union payload node, record member type node, checked
    node, source-map entry node, fact nodes, omitted node and the node an omission
    cause names, temporal binding declaration) equals the plan-root package's id for
    the node, so a node the writer renames is named by its renamed id in every row
    and a row naming no renamed node is unchanged; no row is written anew. The selected
    model declaration and field rows equal the unit's acquisition rows, and IR's selected
    exact identity join accepts them against the package read back.
- id: FR-251-AC-43
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: context_only
  excerpt: For a plan root written over a unit with two temporal clauses, each subject's
    clause node and claim ordinal select, through the returned `Emission`'s temporal
    binding rows, the declaration the plan root's `clause` binding names and the declaration
    of the package's `temporal_clause_names` row of that ordinal, read back through
    the strict reader; the selection is re-keyed with the retained rows through every
    rename the writer performs or may perform, so a carry that renames a clause declaration
    returns the renamed id and not the base package's id; a subject whose clause node
    or claim ordinal the package does not hold selects nothing, and a subject's selection
    never returns the other clause's declaration.
- id: FR-251-AC-45
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: From a genuine authored tick-monitor unit, the public plan-root handoff
    returns the complete QSL-admitted model-step order and the observation owner's
    original canonical association VALUE, bound to the same final emission as its
    plan and two timing obligations. A public consumer compares the complete values
    and original source, structural clause node, claim role and ordinal, selected
    subject and checked model-time facts; the final emission temporal binding join
    equals its plan-root clause and the strict reader declaration. Both values and
    owner capabilities remain usable after temporary compilation inputs are dropped;
    clones and writer renames retain the owner-permitted association and all previously
    retained rows.
- id: FR-251-AC-46
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: Independently omit the checked order premise and progress association;
    substitute a foreign admitted emission; change the structural clause node, claim
    role, claim ordinal or original source; change the selected subject, observation
    source, clock identity/revision, model, structural model/unit/dimension/digital-operation
    node, unit, source kind, digital operation or exact period; and substitute order
    or progress values. Each candidate returns its typed missing-premise, owner refusal
    or mismatched-selection condition and no partial admitted context. Exact owner
    canonical-association byte bounds succeed; one byte less returns the original
    owner resource refusal. A name, digest, clock configuration, equal numeric index,
    full authority document or a second emission cannot turn refusal into admission.
- id: FR-251-AC-47
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: 'After original source, model and request inputs are dropped, a public
    consumer reads the actual combined checked unit retained by the plan root: its
    exact source includes the authored unit and generated entries, its selected clause
    equals the plan subject, and its checked time source retains the selected model
    node, source kind and unit. These are explicitly original checker facts, not final
    renamed identities or admitted order/progress.'
- id: FR-251-AC-48
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: A genuine dense model-time monitor root yields the complete specified canonical
    model-step value, with the actual checked model/unit/dimension identities. A digital
    source retains its exact period and checked tick-operation anchor. Changed source
    kind, unit or period changes the value. Exact-byte-ceiling encoding succeeds and
    one byte less returns Encoding with the original native canonical limit and no
    partial order value.
- id: FR-251
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: 'The plan root SHALL expose its plan, timing obligations and final emission

    through read-only accessors. A caller SHALL NOT replace any of those outputs

    while retaining a context that can admit authority for the original emission.


    The plan root SHALL retain the actual combined checked unit from which its

    final emission was written, including its exact source and checked time-source

    facts. Reads of that unit SHALL identify its keys as original checker keys,

    not final writer-renamed identities or admitted order/progress authority.


    ### Admitted order and progress association


    The request SHALL receive a genuine independently admitted observation-owner

    progress association as an explicit input under the observation owner''s

    [authority-value contract](ix://agent-ix/quire-observation/FR-004). Checked

    model-step facts may establish the model-time order; they SHALL NOT establish

    a static progress context in place of that observation authority. The

    observation owner''s canonical association VALUE and the capability that

    admitted it SHALL be retained without substituting a compiler-generated assertion.
    QSL SHALL consume the supplying owner''s bounded canonical-value

    interface; the complete progress authority document bytes SHALL NOT serve as

    the association value. A lowerable canonical-value ceiling SHALL preserve

    the owner''s typed resource refusal without a partial association.


    The canonical model-step order value SHALL be RFC 8785 JSON with exactly

    `kind: "model-step"`, `model`, `unit`, `dimension` and `source`. The model,

    unit and dimension components SHALL be the original checked time source''s

    structural node identities encoded through the existing structural-id

    conversion, not aliases or a new identity scheme. For a dense source,

    `source` SHALL contain exactly `kind: "dense"`. For a digital source it SHALL

    contain exactly `kind: "tick"`, `operation` (the original checked tick-operation

    anchor) and `period` with canonical decimal-string `numerator` and

    `denominator` of the checked exact rational. No absent tick anchor SHALL be

    defaulted. The value establishes model-step semantics under QSpec FR-252;

    it contains no timestamp sorting or event-arrival ordering claim. Encoding

    SHALL obey the caller''s finite canonical byte ceiling and preserve the native

    encoder refusal without a partial value. These original structural components

    are not final writer addresses and do not replace the final-emission join.


    The public plan-root handoff SHALL expose QSL-admitted model-step order and

    independently admitted progress-association authority bound to the same final

    emission as its plan and timing obligations. The handoff SHALL preserve the

    original canonical admitted values and the owner-established association with
    the

    selected package, clause, subject and claim occurrence. It SHALL use the

    owning admission interface; neither the monitor scope type nor its watermark

    entry establishes that authority.


    QSL SHALL independently compare the observation association''s selected subject,

    clock identity and revision, observation source, model, time unit, source kind,

    and digital operation and exact period against the independently selected

    observation inputs and original checked model-time source. These component

    comparisons SHALL NOT establish joint authority without comparing the complete

    structural model, unit, dimension and digital operation node identities and

    the actual clause claim occurrence. The clause comparison SHALL retain the

    original structural clause node, claim role and ordinal, and original source

    reference under the owning occurrence contract. QSL SHALL compare the complete

    owner-encoded structural components; a digest, alias, clause name, clock label

    or equal numeric position SHALL NOT replace any component. This requirement

    specifies the compared owner values, not a new carrier or an alternative

    claim-string encoding.


    Before issuing the admitted context, QSL SHALL join the original clause node

    and claim ordinal through that same final emission''s retained temporal binding

    rows. The selected declaration SHALL equal the plan root''s clause binding and

    the strict reader''s selected clause declaration. A foreign emission or a

    binding row from a second emission SHALL NOT satisfy this association.


    The handoff SHALL return a typed refusal without a partial admitted context

    when the checked order premise or progress association is absent, when its

    owner rejects it, or when an independent source, subject, clock, model, time or

    claim comparison fails. A foreign emission, changed selection, changed source,

    substituted order or

    substituted progress association SHALL NOT acquire authority by matching a

    clause name, type, structural label or numeric index. The handoff SHALL NOT

    fabricate an admitted value or infer authority from clock configuration. A

    bare semantic identity SHALL NOT substitute for a complete admitted value.

    QSL SHALL NOT reconstruct the association from a second emission or duplicate

    the owner admission rules.


    The handoff SHALL retain the owner capabilities required to use both values

    after temporary compilation inputs are dropped. A clone or writer rename

    SHALL preserve precisely the association the owning admission contract

    permits; a package with equal contents SHALL NOT substitute for an owner-bound

    package instance when that contract requires the original instance. The

    retained witness and final-binding requirements above remain in force.'
- id: TC-1081
  path: spec/test-cases/TC-1081-monitor-builds-from-the-plan-alone-and-refuses-tampered-or-unsupported-plans.md
  role: examined
  excerpt: '29. Produce a tick-monitor plan from genuine authored source and an independently
    derived and strict-read observation-owner progress association. Through the public
    handoff compare the complete canonical model-step order and owner association
    VALUE with their original owner values; compare the original source, structural
    clause node, claim role/ordinal, selected subject and checked model-time facts.
    Join through the same final emission temporal binding rows and compare its clause
    binding with the strict reader declaration. Drop temporary compilation inputs
    and repeat public reads of values and capabilities alongside the plan and obligations.
    Exercise owner-permitted clone and writer-rename behavior while retaining all
    old witness rows (AC-45).

    30. Independently remove the checked order premise and progress association; substitute
    a foreign emission; change the original source, structural clause node, claim
    role and ordinal, selected subject, observation source, clock identity/revision,
    model, structural model/unit/dimension/digital-operation node, unit, dense/tick
    kind, digital operation and exact period. Substitute the complete order and association
    values and the full authority document bytes. Invoke public admission and assert
    each typed refusal with no partial admitted context. Encode the genuine owner
    association at its exact observed byte length and one byte less; inspect the original
    owner resource refusal. Do not fabricate an admitted authority, encode a structural
    node as a digest, or infer association from labels, clock configuration or numeric
    positions (AC-46).

    31. Drop the original authored source, model documents and request inputs after
    producing a plan root. Read its retained combined checked unit through the public
    API; compare the authored source prefix, generated entry names, original selected
    clause key, model object key, dense source kind and millisecond unit. Separately
    join the final clause through the emission retained rows, without treating original
    checker keys as final renamed identities or progress authority (AC-47).

    32. Read the public canonical model-step order value from genuine dense and digital
    model-time roots. Compare the entire object with the checked model, unit and dimension
    identities; for digital time compare the checked operation anchor and exact period.
    Change source kind, time-source unit and digital period independently. Encode
    at the exact observed byte length and one byte less; inspect the original canonical
    limit refusal. The plan-root accessors preserve immutable ownership of the original
    plan, obligations and emission (AC-48).'
- id: TC-1081
  path: spec/test-cases/TC-1081-monitor-builds-from-the-plan-alone-and-refuses-tampered-or-unsupported-plans.md
  role: examined
  excerpt: 'The procedure defines required future evidence; this specification change
    grants

    no implementation or qualification credit. Positive joint-authority steps remain

    planned until the observation owner supplies the structural node and original

    claim-occurrence association contract (IR-1390). An owner API that accepts only

    64-hex node strings does not admit QSL''s complete structural identities; no

    conversion to a fabricated digest or claim string is permitted.


    Component controls independently change the observation source, clock identity,

    clock revision, model, unit, dense/tick kind, operation, period numerator and

    denominator against genuine checked source facts. These controls provide no

    positive joint-authority credit until the original structural node and claim

    occurrence comparisons and owner-issued association test also pass.'
```
