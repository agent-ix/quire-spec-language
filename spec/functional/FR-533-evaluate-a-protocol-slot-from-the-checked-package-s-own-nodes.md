---
id: FR-533
title: "Evaluate a protocol slot from the checked package's own nodes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: depends_on
  - target: ix://agent-ix/quire-protocol/FR-045
    type: references
---
# FR-533: Evaluate a protocol slot from the checked package's own nodes

## Description

FR-390 states that a package the reader returns for FR-383's nodes gives the same
`ProtocolSlotEvaluation` and the same charge log as the checked package that was
emitted, and that the reader view takes nothing from the source package. The
reader-backed view the code has today does not yet meet that: it is built from an
`AdmittedPackageRead`, which retains the source `CheckedPackage`, and it takes the
slot selection, the binders with their names, levels and types, the observation's
model selection and type environment, and the body it evaluates from that source;
the reader contributes a reference-slot projection and a node-presence check. A
consumer with only a `CheckedPackageV2` cannot reach any of it. This requirement
makes the protocol-slot call of `BarePackage` (FR-531) read the package's own nodes
for every one of those, and states the equivalence with the source route.

## Semantic authority and boundary

FR-390 owns the slots, the supply, the outputs and the reader view's projections
and refusals; FR-400 owns the checked-term read vocabulary; FR-520 owns the outcome
map. This requirement adds none of them. It states that the reader view of FR-390
is the evaluation path of the bare package, with the source package removed from
its inputs, and which source-side reads go away with it.

## Inputs

A `BarePackage` with its `BareSupply` (FR-531); a `ProtocolSlot`, a
`ProtocolSupply` and a `Meter` (FR-390), unchanged. The observation of a supply
is admitted by the bare view's `admit_protocol_observation`, which takes the
caller's domain package documents, snapshots and limits as it does on the source
route.

## Outputs

`Result<ProtocolSlotEvaluation, CallFailure>` of FR-390, unchanged.

## Behavior

### What the bare view reads from the package

| Read | From the package | Retired source read |
| --- | --- | --- |
| slot selection (node, member, index) and its refusals | the admitted `protocol` node's body members (FR-464) | `protocol_selection::select` over the source package |
| binders: node key, level, name, `ValueType` | the admitted binder nodes and the kernel `ValueType` projected from them (FR-390 "Typed projection admission") | the source frame of the checked body |
| the expression evaluated | the admitted term, read as FR-400 `NodeRead` through `TermHandle` | the source `CheckedBody` |
| the read location of a binder and of each visit | the `Location` of the admitted node and the `protocol_locations` rows (FR-390) | the source package's occurrence list |
| the observation's model selection and type environment | the package's `model_root` nodes and the caller's domain package documents | `graph().model_selections()` and `graph().scope().types()` |
| callee and dispatch candidates | the admitted callable and dispatch projection nodes, within the admitted closure | the source package's dispatch tables |
| the recursive value type of a slot's root | none from the source: see "Types the bytes cannot project" | the checked root's value type |

The view takes the slot root and origin only from the slot descriptor and the
admitted nodes (FR-390-AC-50) and consults no `CheckedPackage`.

### The call

`BarePackage::evaluate_protocol_slot` and `evaluate_selected_protocol_slot` create
the view, select the slot, admit the supply and run the one S6a `Machine` over
the `NodeRead` projection, with the one `ProtocolClause` hook. A `ViewFailure` of
view creation is returned as `CallFailure::Admission` carrying the catalog pair
`unknown_required_feature`/`unsupported-feature`, with the reason and locus, before
any charge. No interpreter, family table or checking pass is added.

### Names and locations

A binder is named in refusals as FR-390's causes name it: by its `NodeKey`, and by
the declared name the admitted binder node carries where it carries one. A location
is the `Location` the package's own nodes and rows give, and `None` where the wire
carries none (`ProtocolSlotEvaluation::Unavailable.location`; `Evaluation.location`).
The source route returns the same values: after this requirement it reads the same
rows, and a location it used to take from the source `CheckedBody` that the package
does not carry is `None` on both.

### Types the bytes cannot project

A slot whose binder, literal or callee type has no kernel representation in the
admitted closure refuses `ViewFailure::Unsupported` with `MissingTypeRepresentation`
(FR-390-AC-17) on the bare route. Recursive record types are such a case today: the
wire projector has no recursive member support, and the code supplies the exact
recursive value type from the source root. The bare route has no source root, so a
slot over a recursive type refuses on it and evaluates on the source route until
Contract IR projects recursive members (IR-1194, FR-390). The refusal is the
visible form of that gap; no bare result substitutes a guessed type.

### Equivalence

For one package, one slot, one supply and one meter limit set, the bare route and
the source route (`CheckedPackage` and the reader-backed view over an
`AdmittedPackageRead`) return the same `ProtocolSlotEvaluation` or the same
refusal, including value, outcome category, exit status, catalog code and cause,
unavailable binder, location and ordered losses, and the same charge log:
every counter of FR-255 at every charge point, including the explicit-dereference
distinction of FR-390-AC-19 and the one-visit-per-application rule for shared
terms. Once IR-1280 lands the per-binder attribution and `accounting_version` are
carried by both.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-533-AC-1 | For every slot of FR-390's table that **R** holds (the activation guard, the captures of each stage, the trigger guard, the retry relation, the recovery predicate of `Partial` and `Full`, and the constraints), over the supplies of TC-1070 and TC-1315, the result and the ordered charge log over the `BarePackage` equal those over the in-process `CheckedPackage` and over the reader-backed view of an `AdmittedPackageRead`, with value, category, exit status, catalog code and cause, unavailable binder, location and losses compared. | Test (TC-1333) |
| FR-533-AC-2 | For each outcome row of FR-520's map (true and false guard, retry relation and recovery predicate; an unavailable binder; a wrongly typed value; an unsupported construct; a meter one below measured usage), the bare route produces the successor outcome, and the exact-usage and one-below vectors of FR-390-AC-9 hold for **R**'s `retry.relation` over the `BarePackage`. | Test (TC-1333) |
| FR-533-AC-3 | A `BarePackage` whose package was read in a run in which the source unit was never compiled evaluates AC-1's slots with the results of AC-1; a test that replaces, in the in-process comparison, the retained source with the source of a different slot of the same package leaves the bare result unchanged. | Test (TC-1333) |
| FR-533-AC-4 | Inspection finds that no function reachable from `BarePackage::evaluate_protocol_slot` takes or reads a `CheckedPackage`, `CheckedBody`, `AdmittedPackageRead::source` or a source `Graph`, that `protocol_selection::select` over a source is not called on the bare route, and that the bare route's `NodeKind` match has no wildcard arm. | Inspection (TC-1333) |
| FR-533-AC-5 | A bare observation admission over the domain package documents of **R**'s model admits the snapshot that the source route admits, with equal `AdmissionUsage`; a model selection the package does not hold refuses `invalid_model_binding`/`wrong-model-selection` as FR-390-AC-21 states, with equal records on both routes. | Test (TC-1333) |
| FR-533-AC-6 | Over a package whose `model_root` nodes select a model other than the supply's, the bare slot call refuses before any charge with the record the source route gives for the same mismatch. | Test (TC-1333) |
| FR-533-AC-7 | A slot whose binder type is a recursive record refuses on the bare route with `ViewFailure::Unsupported` and `MissingTypeRepresentation` at the `Node` locus of the binder, carrying `unknown_required_feature`/`unsupported-feature`, no partial value and no charge, while the same slot evaluates on the source route; the refusal names the type and no value of a different type is substituted. | Test (TC-1333) |
| FR-533-AC-8 | A binder unavailable and read returns `Unavailable` naming the binder and the read, with the location the package's rows give (FR-390-AC-32), equal on both routes; where the wire carries no leaf location the result is `None` on both and the source route no longer reports a source-derived location. | Test (TC-1333) |
| FR-533-AC-9 | Three slot calls over one `Meter` on the bare route leave a charge log that, per FR-520-AC-7, equals the sum on fresh meters for `work_units` and `result_units` and the maximum for the other eight counters, and equals the log of the same three calls on the source route. | Test (TC-1333) |
| FR-533-AC-10 | `BarePackage::new`, `with_supply`, view creation, slot selection and supply admission charge no counter and take no `Meter`; a refusal at any of them leaves a caller's meter byte-identical. | Test (TC-1333) |

## Dependencies

- **Upstream:** FR-390 and its open reader-view criteria (AC-13, AC-15, AC-16,
  AC-18 to AC-22, AC-24 and the location and projection criteria that the reader view
  must meet from the package alone), FR-383, FR-400, FR-520, FR-531; IR-1193,
  IR-1194, IR-1280; QSL-1377.
- **Downstream:** quire-protocol FR-045.
