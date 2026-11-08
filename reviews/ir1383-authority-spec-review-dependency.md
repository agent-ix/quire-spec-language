---
id: SR-20261106200939
title: dependency review of same-emission monitor authority
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language; IR-1383 IR-1388 IR-1389 QSL-1599 PR2489 combined full origin/main...HEAD, including PR2542"
review_set: subset
---

## Summary

Ticket IR-1383, PR 2480, source head 44cdf1333. Reviewed the two changed Markdown artifacts with the spec-review/dependency method.

## Verdict

**PASS** — No findings.

The added hard prerequisite is observation FR-004 owner-admitted association value/capability. Its current 64-hex node form is an explicit enablement gap, addressed by IR-1390 before QSL positive joint-authority qualification; no fallback or hash is authorized. QSpec FR-252 supplies model-time step-order semantics. QSL FR-251 is the consumer feature: observation association/structural identity enablement and checked model-time facts precede QSL comparison and same-emission handoff, which precedes downstream driver/RT use. The new direct edge has no reverse FR-251 dependency in observation FR-004; no cycle is introduced.

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

## Combined stack final examination

Frozen candidate `a122d51a2e6d1e4a2f250b801bbddc9cdde1c5d0`; full `origin/main...HEAD` against main `d32cfd898`. PR2542 `b698d5d40` is included. This supersedes narrower historical scope without rewriting historical findings.

PASS for the new dependency edge: Contract IR FR-358 (enablement) precedes QSL FR-251 authority adoption (feature); the edge points outward to the owning admission contract and introduces no local cycle. Normal Cargo.lock adoption uses published main 2f775339. Contract IR961 is an adopted owner fix, not a local owner workaround. PR2542 standalone qualification is not asserted.


## Examined combined scope

```yaml
scope:
- id: FR-251-AC-45
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: From a genuine authored tick-monitor unit, the public plan-root handoff
    returns the complete QSL-admitted model-step order and the observation owner's
    original canonical association VALUE, bound to the same final emission as its
    plan and two timing obligations. A public consumer compares the complete values
    and original source, structural clause node, claim role and ordinal, selected
    subject and checked model-time facts; the final emission temporal binding join
    equals its plan-root clause and the strict reader declaration. The handoff exposes
    the actual emitting package's original opaque IR Ad
- id: FR-251-AC-46
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: Independently omit the checked order premise and progress association;
    substitute a foreign admitted emission; change the structural clause node, claim
    role, claim ordinal or original source; change the selected subject, observation
    source, clock identity/revision, model, structural model/unit/dimension/digital-operation
    node, unit, source kind, digital operation or exact period; and substitute order
    or progress values. Each candidate returns its typed missing-premise, owner refusal
    or mismatched-selection condition and no partial admitted context. Owner admission
    at its exact supported docume
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
- id: FR-251-AC-49
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: 'The authority handoff and order materialization receive the caller''s
    one mutable NativeJoinBudget, precharge every specified selection/materialization
    work and logical-storage step, and preserve previous consumption when used before
    row adaptation and native closure preparation. Measure genuine dense and tick
    handoffs at ample allowance: exact combined work and storage succeed; one short
    independently preserves the first unpaid native kind, full amount, original bound
    and non-owning locus without a context. Precharge part of the same budget before
    entry, then demonstrate that two individually '
- id: FR-251-AC-50
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: A genuine dense authority returns its original checked ModelTimeSource
    by borrow and no IR tick period; a genuine tick authority returns its original
    checked source and an actual borrowed IR CheckedRational with the exact same checked
    numerator/denominator. The source borrow names the same element of the immutable
    retained checked-unit source slice. The IR period comes from the selected same-package
    clock's validated canonical period components, compared against the checked association
    and copied once under the original shared budget; exact storage for the actual
    IR container and two component
- id: FR-531-AC-28
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: Producer and bytes-only equality admission pass the caller's same `NativeJoinBudget`
    through IR's admitted equality reader after identity/type projection. Decisions
    and non-resource witness refusals are retained for later unmetered selection and
    projection. Successful reads consume native work and storage cumulatively; exact
    measured bounds admit, and one unit less in either resource refuses with the original
    typed IR equality denial and recorded first unpaid charge, never an unknown witness
    or internal fault. Reusing an admitted slot consumes no additional native allowance.
- id: FR-531-AC-29
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: Materializing an authentic emission's equality witness with a shared native
    budget preserves every application, fact and source-unit row, including original
    identities and order. The source admission and handoff paths use the same budgeted
    row builder. Exact measured work and storage allowances return the complete witness;
    one unit less in either resource refuses without allocating the denied material
    or a diagnostic, retains the first unpaid resource, amount and original locus,
    and returns no partial witness. Two individually affordable materializations that
    exceed one shared allowance refuse
- id: FR-531-AC-30
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: 'Source-enum materialization and QSL-to-IR model-enum-shape and selected-collection
    conversion preserve original structural/declaration/member identities, canonical
    case order and complete Exact types on one shared native budget. Exact measured
    bounds admit; work or storage one short and zero storage return allocation-free
    compact denials at the first unpaid locus with prior consumption intact. A later
    conversion cannot reset a preceding adapter''s charges or replace its first denial.
    A selected source map preserves its original optional key node and complete Exact
    map value type through actual '
- id: FR-531-AC-31
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: An authentic emission with source records, record members and unions produces
    complete identity rows equal to the original producer claims after native joining
    and QSL materialization. The measured work and storage include the QSL producer
    input copies, the single native join and each later QSL declaration, field, record/member
    and union copy. Exact allowances admit; one unit below either measured allowance
    refuses at its first unpaid step with no partial identity rows, the original configured
    bound and the admitted prefix intact. Denied external row copies retain AdapterRow(i)
    at their origin
- id: FR-531-AC-32
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: A real tick-monitor plan's retained context supplies the same emission's
    plan, obligations, admitted order and progress association to the actual native
    row join and selected type-closure projection. The resulting native nodes, original
    declaration/member identities and Exact types equal those of that emission. Substituting
    another genuine emission's context, even with identical authored names, or omitting
    required order/progress authority returns a typed refusal and no executable context.
    Borrowing retained context leaves native storage consumption unchanged.
- id: FR-531-AC-33
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: Allocation observation encloses only the adapter/conversion call, with
    fixture preparation and assertions outside it. A successful owned materialization
    performs observed allocations; a denial before its first allocation performs none,
    and a later denial allocates neither the denied material nor an owned diagnostic.
    The controls include fresh owner-produced lazy numeric payloads, values `2^127`
    and `-(2^127 + 1)`, and larger heap-backed values without prior warming or formatting.
    These values remain exact on successful copies; insufficient work/storage retains
    the first real row locus and orig
- id: FR-071-AC-7
  path: spec/functional/FR-071-implement-typed-replay-request.md
  role: examined
  excerpt: A request whose encoded size exceeds the configured reader bound refuses,
    and no truncated or partially-populated request is returned.
- id: FR-098-AC-7
  path: spec/functional/FR-098-execute-a-replay-request.md
  role: examined
  excerpt: A request whose `dependencies` entries are swapped, or repeat one identity,
    refuses `ReplayRefusal::DependencySelections` (`invalid_package`/`invalid-value`
    at `/dependencies`) before any recompile; one carrying an extra entry no import
    reaches refuses `DependencySelections` after the recompile; two entries whose
    sources share one authority and identity refuse `ReplayRefusal::DependencyInput`
    (`invalid_package`/`conflicting-definition`); one lacking an entry refuses `ReplayRefusal::Recompile`
    carrying `missing_import`/`missing-selection` at the import; an entry naming two
    sources of one librar
- id: FR-098-AC-20
  path: spec/functional/FR-098-execute-a-replay-request.md
  role: examined
  excerpt: 'A request for a proved package whose refinement `CasTwice` has the internal
    list `[{"refinement": "CasTwice", "internal": ["Spec::Twice::Done"]}]` carries
    it at `/package/lock/refinement_internals` and recompiles. A well-shaped list
    naming an unknown refinement or an unresolved internal class (`Spec::Twice::zz`
    at index 0) refuses `RefinementInternals`, retaining the complete native compiler
    refusal with its original code, cause and relative locus, and separately locating
    the request at `/package/lock/refinement_internals` followed by that relative
    pointer. A root value of `{}` is instead a bo'
- id: FR-350-AC-16
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'A `replay-request` file whose `package` is the checked-package document
    named `orders` with `contract_version` `quire.checked-package/v2`, whose `lock.sources`
    holds the one `{authority, identity}` entry `("repo", "orders.qsl")`, and whose
    `dependencies` holds `{identity: "lib/money", sources: [("repo", "money.qsl")]}`,
    decodes to a request with package name `orders`, that one source and one dependency
    entry with `identity` `lib/money` and its one source, and no version member; the
    same file with `package` removed, with `package` set to the string `"orders"`,
    with `contract_version` set to `qu'
- id: FR-350-AC-17
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'A `byte_provision` row whose `bytes_path` is `missing.bin` (absent from
    the vector directory), `unlisted.bin` (present but not listed as `kind: bytes`),
    `/etc/hostname` (absolute) or `../x.bin` (parent-traversing) is a `tool-failure`
    with cause `corpus-decode` and pointer `/byte_provision/0/bytes_path`, and no
    request is constructed; a listed `kind: bytes` path is read as raw bytes.'
- id: FR-350-AC-18
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: A `stage_limits` map holding the key `depth` (no FR-255 setting of that
    name) is a `tool-failure` with cause `corpus-decode` and pointer `/stage_limits/depth`;
    the key `s1.input_bytes` with the value `"1024"` decodes to the entry `("s1.input_bytes",
    1024)`.
- id: FR-480-AC-7
  path: spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md
  role: examined
  excerpt: 'AC-1''s package replays: an FR-098 request whose package reference lists
    `U1` and `U2` in canonical unit order, with both sources in the byte provision,
    recompiles to the same structural package and replays the D10 call. The same reference
    with the two sources reversed recompiles to the same structural package and replays;
    with one source repeated it refuses before any recompile, `invalid_package`/`invalid-value`
    at `/package/lock/sources`. A reference naming a definition document refuses before
    any recompile.'
- id: FR-484
  path: spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md
  role: examined
  excerpt: 'A function-value parity claim says that the generated code of a

    non-`Boolean` function `f` returns QSL''s exact `f(b)` at the parameter

    bindings `b` it was run at (FR-357, function level). Its obligation is the

    whole function: on every argument vector the declared signature of `f`

    admits, the generated outcome equals QSL''s exact one. A counterexample is

    one such vector together with the generated outcome observed there.


    QSL SHALL own this obligation''s identity. It SHALL define the preimage,

    mint it as the preimage value itself, compared by structural equality

    (ADR-013 O-09), hand it to the cla'
- id: FR-488
  path: spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md
  role: examined
  excerpt: "The S4 package emitter (ADR-011 layer 4 `package`, edge E4) SHALL emit\
    \ each\nchecked hyper clause over `behaviours` (FR-172) as one\n`quire.checked-semantic-graph/v2`\
    \ node with `node_tag: \"claim\"` and\n`semantic_form: \"hyperproperty\"`, whose\
    \ body is the seven-binding aggregate\nQSpec FR-402 states, together with the\
    \ nodes that aggregate references: the\nclause's object-parameter and subject\
    \ model nodes, its `match` conjunct\nnodes, its `align skip` operation anchors\
    \ and its body's formula nodes.\nQSL's I2 reader reads the node back, and the\
    \ checked clause exposes its\n`claim` occurrence (\xA7 \"Outputs\")."
- id: TC-1081
  path: spec/test-cases/TC-1081-monitor-builds-from-the-plan-alone-and-refuses-tampered-or-unsupported-plans.md
  role: examined
  excerpt: 'Verify that the executable monitor plan of FR-251 is self-sufficient,
    that admission refuses any plan not derived from the selected clause and target,
    that built monitors do not share state, that executing the plan agrees with QSL''s
    reference evaluator, and that every build, admission and execution fault is a
    typed non-verdict result.


    Scope: FR-251-AC-4 to FR-251-AC-12, FR-251-AC-14, FR-251-AC-15, FR-251-AC-17,
    FR-251-AC-19, FR-251-AC-21, FR-251-AC-23 to FR-251-AC-25, FR-251-AC-27 to FR-251-AC-30,
    FR-251-AC-32 to FR-251-AC-39, FR-251-AC-41, FR-251-AC-42, FR-251-AC-43 through
    FR-251-AC-50.'
- id: TC-1325
  path: spec/test-cases/TC-1325-the-runner-decodes-a-replay-request-file.md
  role: examined
  excerpt: 'Verify FR-350-AC-11 to FR-350-AC-14 and FR-350-AC-16 to FR-350-AC-18 (AC-15
    is retired) over fixture `replay-request` files. A wrong implementation this test
    catches: an adapter that executes no replay vector, one that keeps revision labels,
    one that reports a malformed corpus file as a QSL refusal.'
- id: TC-1331
  path: spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: 'Verify the bytes-only entry and its selected-monitor preparation boundary.

    Scope: FR-531-AC-1 through FR-531-AC-33. **R** is the package of FR-517.

    The isolated bytes-only consumer test crate depends on `quire-contract-model`,

    `qsl-package` and `qsl-eval` only. Producer transport and selected-monitor

    preparation tests additionally obtain authentic final emissions, admitted model

    inputs and the FR-251 plan context through their public QSL APIs before handing

    rows or bytes to the consumer.'
- id: QSpec-FR-402-AC-5
  path: agent-ix/quire-specification/spec/objects/interfaces/FR-402-hyperproperty-obligation-identity.md
  role: examined
  excerpt: A `behaviours` clause's `quantifiers` binding is a Tuple of one quantifier
    Group per quantifier, each Group binding `variable`, `quantifier` and `alias`
    to text literals and then holding one `reference` per fairness constraint to its
    `temporal`/`fairness` node; with `fair weak V::Vault::step` on `a` of TC-337 HB-01,
    `a`'s Group holds one reference and `b`'s none; the body passes strict wire validation
    of FR-322's grammar with no `malformed_wire`, and the reader admits the package.
    A finite-domain clause's Groups bind `variable` and `quantifier` only and its
    `subjects` is empty.
- id: QSpec-FR-402-AC-6
  path: agent-ix/quire-specification/spec/objects/interfaces/FR-402-hyperproperty-obligation-identity.md
  role: examined
  excerpt: 'Each `subjects` binding references the `model`/`model_root` node of the
    package its alias selects: two aliases'' clauses over one object type of each
    package reference two roots, and a clause whose alias binds no object parameter
    has that alias''s root as its subject; a `subjects` target that is a `model`/`object_type`
    node refuses `invalid_model_binding`/`malformed-declaration` at the target; a
    quantifier `alias` naming no subject refuses `missing_declaration`/`missing-name`
    at the `alias` binding; a `subjects` binding no Group names, and a body of six
    bindings, each refuse `invalid_semantic_gr'
- id: QSpec-FR-402-AC-7
  path: agent-ix/quire-specification/spec/objects/interfaces/FR-402-hyperproperty-obligation-identity.md
  role: examined
  excerpt: 'Each quantifier fairness refusal is located at the reference `/semantic_graph/nodes/{n}/body/members/3/value/members/{g}/members/{m}`:
    a fairness node whose target is a `protocol_root` target refuses `invalid_model_binding`/`malformed-declaration`;
    one whose `operation` target names no effective operation refuses `missing_declaration`/`missing-name`;
    one whose declaring node belongs to another alias''s package refuses `invalid_model_binding`/`malformed-declaration`;
    and a non-empty quantifier fairness set under a body wrapper whose profile is
    `quire.temporal.event-position.false-extension/v1` r'
```

## Validation boundaries

The lead owns the single final combined gate after fixes. No aggregate gate, workspace build, make ci or Kani was run by this reviewer. Full mutation testing was prohibited on the frozen branch; static old-code sensitivity was checked from the oracles. Coder receipts were inspected as secondary evidence and are not substituted for reviewer runtime results.

Combined additional examined unit:

```yaml
scope:
- id: FR-447-AC-4
  path: spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md
  role: examined
  excerpt: The request writer writes for `SlowEnough` one item of kind `temporal-satisfaction`,
    extent `bounded`, carrying the hybrid-dynamics requirement, whose `claim` names
    `SlowEnough`'s emitted `temporal_clause` node and its `claim` occurrence; with
    an in-process fixture hybrid provider registered the item routes to it, which
    reads `Braking` from the checked package; with only a process fixture hybrid provider
    registered the item routes to it, which receives the package's v2 bytes and reads
    `Braking` and `SlowEnough` from their QSpec FR-481 nodes; with none registered
    it settles `unsupported`, `unsu
```

## Current evidence and limitations

Computed `quire matrix --scope . --format json` produced zero JSON bytes after more than seven minutes of CPU work and was explicitly stopped with SIGTERM. A scoped manual tag/test/source inventory was used; repository-wide untagged counts, matrix completeness and a global matrix PASS are unavailable. Plan completion: not assessed.

Focused reviewer runtime receipts are in this scratchpad. Coder clippy/fmt receipts are secondary; the user assigned the aggregate checks, cargo deny and final gate to the lead, so they were not duplicated. Standalone hybrid_probe.rs uses public APIs linked to the freshly built current qsl_replay library, outside the frozen tree, and reproduces wrong-claim binding without mutating the candidate. No review clone, source edit, commit or push was made.

Focused frozen-head checks: original AdapterRow output, sparse retained field index, tick complete three-resource handoff, root refinement false replay, dependency-isolated refinement false replay, hyper fairness references and two-alias model-root subjects each passed one exact test. Map/sequence actual native projection and cold equality payload tests also passed. The model-enum native closure check FAILED at monitor_root.rs:1308 (expected one Tier row; actual []); historical IR1388 gap FND001 remains still-open. These are focused checks only.
