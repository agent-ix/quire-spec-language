---
id: FR-1044
title: "Request a component composition item through analyze"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-507
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-630
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-850
    type: references
  - target: ix://agent-ix/filament-specification/FR-348
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-491
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-492
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-489
    type: depends_on
---
# FR-1044: Request a component composition item through analyze

## Description

A `native-analyze/1` item that names a checked `compose` declaration SHALL
select that declaration's composition, through the `items` shape FR-620 reads
for every other claim, and FSL SHALL settle the composition and each of its
item-backed premise slots in the same request. FSL adds no verb and no request
member. A composition that no registered composition-capable provider can
discharge settles `unsupported`, naming the `composition` capability, exit 21
(FSpec FR-348-AC-4); FSL never settles it `proved` itself.

### Decisions

- **The selection is the existing item.** FSpec FR-491 "Derived item and
  slots" derives, for a `compose` declaration under `interface`,
  `protocol-refinement` or `refinement-transfer`, one routed FSpec FR-331
  `composition` item, and FR-530 compiles, emits and reads it
  (`RequestWriter::declaration_items`). What was missing is a way for a source
  request to select it. It is selected as a refinement is (FR-630): `claim`
  the declaration's name, `kind` `composition`. The architecture ruling on
  QSL-2739 chose this over a new verb or request family, which would restate
  FR-620's name lookup, kind admission, negotiation and outcome rules.
- **The family is `protocol`.** FSpec FR-290's claim-form row gives a
  composition under the three routed rules the kind `composition` and the
  family `protocol`, and FR-507 states the same row and reads the family from
  the claim's `correspondence`/`component_composition` node (this change). An
  item of any other kind on such a claim is inapplicable by FR-507's rule.
- **An `assume-guarantee` selection is admitted and planner-local.** FSpec
  FR-491 and FSpec FR-290 derive no request item for it and give it no family:
  the planner settles it from FSpec FR-492's settlement request. It is still
  selectable by name, so the second PR can settle it by FR-492's decision
  order, a non-proved premise giving `inconclusive`, `premise-unproved`
  (written only by that planner path, never for a routed item). Until then it
  settles as the routed rules do, `unsupported` naming `composition`. FR-507
  reads a family for request items only, so this selection is not refused by
  FR-507's no-family rule. The `fixed-point` item an accepted cycle derives
  is different: FSpec FR-491 derives it as a routed `composition` item under
  any rule, `assume-guarantee` included, and FSpec FR-290 and FR-507 give it
  the family `protocol`, so it is negotiated as a routed item.
- **The premises are derived; a caller may only restate their keys.** The
  slots are FSpec FR-491's, in its order. FSpec FR-520 keeps `premises` a
  caller item member, because it names slot keys and never a proof or a
  result: a proof or result is never external to FSL (FSpec FR-489). So FR-620
  is unchanged, and for a composition item only, FSL accepts a `premises`
  member that is the derived slot list exactly and refuses any other. The
  member of every other kind keeps its existing admission, a `realizability`
  item's `premises` groups (FR-571) included.
- **A premise item is its slot's item, not the clause requested alone.** A
  `component-guarantee` slot's item is `G` over `C`'s subject checked under
  exactly the assumptions FSpec FR-491 gives it from `G`'s `rely` and
  `depends` members, and the assumption set is part of its slot key. Only a
  `G` with no such member has the item the clause settles to alone. FSL's
  typed slot key is the item's request identity plus FSpec FR-491's assumption
  set. The item `declaration_items` writes for the slot is a
  `temporal-satisfaction` item whose FSpec FR-331 wire has no member for an
  assumption set, and no engine `Engines::fsl()` registers reads one, so the
  `request` member of one guarantee under two assumption sets MAY be equal.
  FSL does not change that wire in this requirement (FSpec FR-492 defines the
  slot key as the request identity, which leaves the gap). It invents no
  engine, and it never runs a slot with a non-empty assumption set as the
  clause alone, which would settle another obligation under that slot's key.
  So a premise item whose assumption set is non-empty settles `unsupported`,
  naming kind `temporal-satisfaction`, before negotiation, and a premise item
  with an empty set is run by the engine that takes its clause. Putting the
  assumption set on the wire is remaining work: QSL-2739, second PR.
- **The composition names its premises by slot key.** FSpec FR-331 `premises`
  holds `{role, key}` per slot and no result. A routed main item holds the
  slots of FSpec FR-331's five roles (`component-guarantee`, `rely`,
  `liveness-dependency`, `fairness`, `environment`), which are FSpec FR-489's
  four premise lists and the component guarantees, and holds no `fixed-point`
  slot: FSpec FR-331 has no `fixed-point` role, and an item that breaks its
  `premises` rule refuses before consumption. The `fixed-point` slot is its
  own item (FSpec FR-491). That FSpec FR-331 names no `fixed-point` role for
  the slot is remaining work: QSL-2739. FSL settles every premise item
  before it writes the composition item's outcome, and it never copies a
  premise result into the composition. How a provider reads those results is
  the second PR's decision.
- **A composition provider is outside this requirement.** Discharging a
  composition from independently proved components (FSpec FR-348-AC-1) and
  refuting a declared interface mismatch, naming the component and interface
  (FSpec FR-348-AC-2), are not delivered here, and nothing here reports an
  `assume-guarantee` verdict as FSpec FR-348-AC-2. FSpec FR-492's composition
  result is never `refuted` while FSpec FR-348-AC-2 refutes, so the provider
  that meets AC-2 for the routed rules is open. Remaining work: QSL-2739,
  second PR.

## Inputs

- A `native-analyze/1` request over a program that declares one or more
  `compose S rule R { ... }` (FSpec FR-491) with the component packages it
  imports.
- An `items` entry `{"claim": "S", "kind": "composition"}`, `S` the
  declaration's name. `kind` is required: the `protocol` family admits several
  kinds, so FR-620's `absent-kind` rule applies. The entry carries `required`,
  `backend` and `options` as any item does. It MAY carry `premises` and then
  only as Behavior states: the derived slot list, never a proof or result.

## Outputs

Each selected composition writes, in this order and only for the composition
it selects:

1. The FR-621 item at the entry's `request_index`.
2. Its premise items, after the caller's items and after FSpec FR-368's
   derived items, grouped per selected composition in the order of the
   caller's entries and, within a group, in FSpec FR-491 slot order: one item
   per `component-guarantee` slot, then the `fixed-point` slot's item when the
   composition has one. A `rely`, `liveness-dependency`, `fairness` or
   `environment` slot is not item-backed and writes no item.

A premise item with the same slot key as another derived item is written once,
at its first position, and shared. A caller's own entry is never merged with a
derived item, and the same name listed twice writes two composition items, at
two indices, over one shared set of premise items.

- The composition item of a routed rule has the `request` member
  `ClaimItem::request_item_wire` writes, its `premises` the derived slot list
  (no `fixed-point` slot), empty
  `candidates`, and a record `unsupported`, `settled_by` `disposition`, no
  `backend`, no counterexample, settled at negotiation (FR-621 `S6b`), with
  diagnostic `unsupported_projection`/`unsupported-requested-capability`
  naming kind `composition`.
- The item of an `assume-guarantee` selection is planner-local: it has no
  FSpec FR-331 request item, so it carries no `request` member and no
  `candidates`, and no negotiation runs for it. FSL settles it before
  negotiation by a disposition: `claim_name` the declaration's name, record
  `unsupported`, `settled_by` `disposition`, no `backend`, the same diagnostic
  and kind, stage `S4` (FR-621, an item settled by a disposition before any
  engine ran), category and exit as any `unsupported` item.
- Each premise item carries its own record, extent, trust basis and
  certification (FR-530-AC-6); a premise item with a non-empty assumption set
  is settled `unsupported` at stage `S4` as Behavior states. The composition
  inherits none: a `proved`
  premise does not make it `proved`, and a `refuted` one does not make it
  `refuted`.
- The document's category and exit code are FR-285's multi-item code, the most
  severe present in the order 30, 20, 21, 22, 10, 11, 0.

## Behavior

- When an entry names a checked `compose` declaration under `interface`,
  `protocol-refinement` or `refinement-transfer`, FSL SHALL resolve it by
  FR-620's name lookup and build the item `declaration_items` writes for it,
  whose `premises` are the derived slot list.
- For every rule, `assume-guarantee` included, the derived slot list is the
  declaration's FSpec FR-491 slots in order but for its `fixed-point` slot.
- When no registered backend advertises `composition`, FSL SHALL settle the
  item `unsupported` naming the capability, and SHALL NOT settle it `proved`,
  `refuted` or `inconclusive`.
- When an entry names a declaration under `assume-guarantee` with `kind`
  `composition`, FSL SHALL settle it `unsupported` as the planner-local item
  Outputs gives, without routing it to a provider.
- When a premise item's assumption set is non-empty, FSL SHALL settle it
  `unsupported` before negotiation, by a disposition at stage `S4`, with the
  diagnostic `unsupported_projection`/`unsupported-requested-capability`
  naming kind `temporal-satisfaction`, its `request` echo as written, empty
  `candidates` and no `backend`, without running an engine on it.
- When an entry names a name no declaration resolves, FSL SHALL settle that
  item `invalid-request` with `invalid_capability`/`unknown-claim` naming the
  name, and the request's other items still run.
- When an entry's `kind` does not apply to its claim, whether the claim is not
  a composition (`kind` `composition`) or is one (another `kind`), FSL SHALL
  settle that item `invalid-request` with FR-507's `inapplicable-kind` and its
  payload `request_index`, `capability` and `family` (absent for an
  `assume-guarantee` claim), and no other member.
- When an entry names a composition with no `kind`, FSL SHALL settle that item
  by FR-620's `absent-kind`.
- If a `kind` `composition` item carries a `premises` member that is not an
  array of closed objects `{role, key}`, `role` one of FSpec FR-331's five
  roles, then FSL SHALL refuse the whole request at the request stage with
  `invalid-request`, exit 20, naming the member's RFC 6901 pointer
  (`/items/<i>/premises`, or `/items/<i>/premises/<j>` and its member), running
  no item, as FR-620 does for any mistyped member. A `premises` member of an
  item of another `kind` keeps its existing admission.
- When a `kind` `composition` item carries a well-formed `premises` that
  parsed equals the derived slot list, FSL SHALL treat the item as one with no
  `premises`. Two lists are equal when they have the same length, and at each
  position the same `role` and the same `key` as parsed JSON values, so the
  order of an object's members is not significant and the order of the list
  is.
- If a `kind` `composition` item carries a well-formed `premises` that is not
  equal to the derived slot list (a slot removed, added, reordered, relabelled
  or with another key), then FSL SHALL settle that item, after name and kind
  admission and the cycle refusal, `invalid-request` at stage `S4` with
  `invalid_runtime_input`/`invalid-value` at `/items/<i>/premises`, as FSpec
  FR-492-AC-2 refuses a slot list that differs from the derived one, deriving
  no premise item for it, and the request's other items still run. The
  comparison is made after the declaration is checked, because the derived
  list does not exist before. An item naming an unknown name or a name that is
  not a composition keeps its code from the bullets above, derives no premise
  item and compares no `premises`.
- When a declaration's dependency graph has a cycle and no selected profile
  declares FSpec FR-176's lattice, FSL SHALL settle the composition item
  `invalid-request` with `invalid_model_binding`/`malformed-declaration`
  naming the cycle's guarantees in edge order, as FSpec FR-491 states,
  without deriving a premise item for it.
- When a cycle's profile declares the lattice, FSL SHALL write the
  `fixed-point` slot's item, under any rule, as one more premise item: a
  routed `composition` item of family `protocol` whose `premises` are the
  cycle's component-guarantee slot keys and whose `options` name the declaring
  profile (FSpec FR-491), written as the one entry `profile` with the
  profile's name as the slot key records it; it is settled at negotiation
  `unsupported` naming `composition` as a routed composition item is.
- When a component-guarantee premise does not settle `proved`, FSL SHALL
  return that premise's own record and SHALL NOT settle the composition
  `proved`.
- When a request has no `items` member, FSL SHALL add the checked compose
  declarations to FR-620's default items, one `composition` item each in
  source order, with their premise items after FSpec FR-368's derived items,
  as FR-620's default rule gives every claim of the program. An
  `assume-guarantee` declaration is a default item of FR-620's rule only as
  far as FSpec FR-290 gives it an item, which is none.
- When a `package` request (FR-850) selects the same unit's checked package,
  FSL SHALL settle the same items, records and exit code as the source request.

- When a registered backend advertises `composition`, FSL SHALL negotiate the
  routed item to it (FSpec FR-348-AC-3) and send it, through the external
  provider request, the item with its derived `premises` slot keys and its
  `options`. The `options` are negotiated as for any provider item: FR-967
  for a named backend, FR-243 and FR-635 against the candidates' manifests
  otherwise. FSL sends no premise result and no component proof with it. FSL
  settles the provider's verdict as for any external provider, with these
  causes: `unknown` is `inconclusive` `solver-inconclusive` (FSpec FR-331);
  `refuted` is `inconclusive` `replay-refused` with the code
  `unknown_required_feature`, because a composition has no replay (FR-290
  "Replay context"); and `proved` carries the certification FSL assigns the
  provider's origin (FR-635: `trusted` for a plugin, `uncertified` for a
  linked provider), never one the provider states. How a provider reads the
  premises' results, and what it refutes, is the second PR's decision.
- FSL SHALL accept a provider's `proved` for a composition only when the
  composition has component-guarantee premises and every one of them settled
  `proved` in the same request. FSL decides this from its own records, never
  from the provider, and it reads each premise by the request index of the
  item its typed slot key (the assumptions included) was given, so a
  guarantee equal on the wire but checked under other peer guarantees is
  another item and never discharges this one. A premise that settled
  otherwise, by an engine, by disposition before any engine ran
  (`unsupported` under peer guarantees), or not at all, fails the check, and
  so does a composition with no component-guarantee premise to check. The
  composition then settles `inconclusive` with cause `replay-refused` and the
  code `unsupported_projection`, naming the provider, with no certification.
  FSpec FR-331 (lines 856-858) defines `replay-refused` as a core-only cause,
  settled by the core's own replay or Kani run, so this use is the nearest
  existing core-only cause under the architecture ruling on QSL-2739, not
  that definition's own case: `premise-unproved` is planner-only and
  `hypotheses-not-established` is the invariant item's. The standard has no
  cause for a provider's proof FSL does not accept; that gap remains work on
  QSL-2739. FSL runs no composition algorithm of its own and adds no proof.

### Current system behavior

The architecture ruling on QSL-2739 (PR #3287) lands this requirement with
the following typed results where the system cannot yet deliver an AC. Each
AC below stays as written and open on QSL-2739.

- **AC-7.** No compiler or profile step binds a profile that declares FSpec
  FR-176's lattice to a source unit, so a request through `fsl analyze` meets
  no lattice and a cycle settles by the AC-6 refusal. No public request
  reaches the `fixed-point` item: it is written only for a checked package
  that carries the lattice, which no source or package route builds today.
  Which profile declares the lattice, and its binding, are decisions for the
  standard before the second PR.
- **AC-10, `package` handoff.** Today a `package` handoff of a unit that
  declares compositions is refused whole at the request stage, and settles no
  item: the strict package reader refuses the package the producer emits, at
  an operand row of its `semantic_graph`, and FSL reports it
  `invalid_package` with no cause, category `refusal`, exit 20, for a request
  with or without `items`. No composition item reaches the package route's own
  resolution. The package route itself reads no composition term's slots or
  component packages: over a package the reader admits, a request with no
  `items` is refused as `unsupported_construct`/`not-yet-implemented` naming
  the `composition` rule, and an item naming a composition or a name no row
  carries is refused naming the `unnamed-claim` family, never as
  `unknown-claim`. That second behavior is the route's source behavior; no
  public request reaches it yet, and its evidence is deferred with AC-10 to the
  package-consumer API lane, which owns the package form.
- **Text mode.** Text rendering of the outcome belongs to the external driver.
  No test in this repository exercises it, and this requirement does not
  weaken the text half of the fixture.

## Acceptance Criteria

The fixture. The domain package `test/counter` is FR-620's Counter fixture
(field `value: Int[0, 3]`, operation `inc`), and a request's default subject
is that package with FR-620's one initialization snapshot `start` holding
counter `a` at 0, which is the initial state every clause below is checked
from. Two checked component packages each select `test/counter` as `M`:
`test/part-a` declares
`temporal Bounded using t0 over (c: M::Counter) clock "model-steps" on origin { always holds(c.value <= 3) }`
and `test/part-b` declares the same clause named `Capped`. Their subject
identities differ, being their own package's and `M`'s. The unit
`test/system` imports them as `A` and `B`, selects `test/counter` as `M`,
declares the clause `Local` (`always holds(c.value <= 3)`, family
`temporal-trace`), and declares, each over
`component PA from A using M; component PB from B using M; guarantee PA::Bounded; guarantee PB::Capped;`:
`Sys` under `interface`; `SysPR` and `SysRT` under `protocol-refinement` and
`refinement-transfer`; `SysAG` under `assume-guarantee`; `SysRely`, `Sys`
with `rely PB::Capped on PA::Bounded;`; and `SysCycle`, `Sys` with that and
`rely PA::Bounded on PB::Capped;`. The unit `test/system-lattice` is
`test/system` selecting the profile `fixed-point-lattice` (the profile FR-530's
emit fixture declares FSpec FR-176's lattice with), declaring `SysLat`
(`SysCycle` under `interface`) and `SysLatAG` (under `assume-guarantee`). The
unit `test/system-default` imports the same packages, selects `test/counter`
as `M`, declares no `pre` or `post` clause on `M::Counter::inc`, the clause
`Local`, then `Sys`. Every request runs through `fsl analyze`, machine and text mode.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1044-AC-1 | Over `test/system`, the item `{"claim": "Sys", "kind": "composition"}` writes the composition at `request_index` 0 with a `request` member whose `premises` are the two component-guarantee slot keys (the derived slot list) and empty `candidates`, and a record `unsupported`, `settled_by` `disposition`, no `backend`, diagnostic `unsupported_projection`/`unsupported-requested-capability` naming `composition`; it writes two premise items, `Bounded` at index 1 and `Capped` at index 2, each `proved` by EN-1; the document exits 21 and the composition is not `proved`. | Test |
| FR-1044-AC-2 | `SysPR` and `SysRT` write the composition as AC-1 does and exit 21. `SysAG` writes at index 0 an item with no `request` member and no `candidates`, `claim_name` `SysAG`, a record `unsupported` with `settled_by` `disposition`, no `backend`, the AC-1 diagnostic naming `composition`, stage `S4`, and premise items `Bounded`, `Capped` at indices 1 and 2; exit 21. | Test |
| FR-1044-AC-3 | The items `{"claim": "Sys", "kind": "composition"}`, `{"claim": "Missing", "kind": "composition"}`, `{"claim": "Local", "kind": "composition"}`, `{"claim": "Sys", "kind": "temporal-satisfaction"}`, `{"claim": "Sys"}` and `{"claim": "SysAG", "kind": "temporal-satisfaction"}` settle, in order: `unsupported`; `invalid-request` `invalid_capability`/`unknown-claim` naming `Missing`; `invalid-request` `inapplicable-kind` with payload `request_index` 2, `capability` `composition`, `family` `temporal-trace`; `invalid-request` `inapplicable-kind` with `request_index` 3, `capability` `temporal-satisfaction`, `family` `protocol`; `invalid-request` `absent-kind`; `invalid-request` `inapplicable-kind` with `request_index` 5, `capability` `temporal-satisfaction` and no `family`; the document exits 20. | Test |
| FR-1044-AC-4 | Over `test/system`: (a) `{"claim": "Sys", "kind": "composition", "premises": P}`, `P` AC-1's `request.premises` with the members of each object written in reverse order, writes AC-1's items, records and exit; the same with `SysAG` and `P` its derived slot list (the same two keys) writes AC-2's items, records and exit. (b) With `P` having the `Capped` slot removed, the two slots swapped, or the second slot's `role` relabelled `rely`, and the same three variants over `SysAG`, each followed by the item `{"claim": "Local"}`, the composition item settles `invalid-request` at stage `S4` with `invalid_runtime_input`/`invalid-value` at `/items/0/premises`, no premise item is written, `Local` is `proved`, and the document exits 20. (c) `"premises": {}`, `"premises": [1]` and `"premises": [{"role": "nope", "key": {}}]` each refuse the whole request at the request stage, `invalid-request`, exit 20, naming `/items/0/premises`, `/items/0/premises/0` and `/items/0/premises/0/role`, and write no item. (d) `{"claim": "Missing", "kind": "composition", "premises": []}` settles `unknown-claim` and `{"claim": "Local", "kind": "composition", "premises": []}` settles `inapplicable-kind`, each per item with no premise item. (e) A FR-571 `realizability` item carrying `premises` `{"role_knowledge": [], "environment": []}` is not refused for `premises` and settles as FR-571 gives it. | Test |
| FR-1044-AC-5 | `SysRely` selected as in AC-1 writes `Bounded` at index 1 `proved`, and at index 2 `Capped`, whose assumption set is `Bounded`, with the `request` member of the clause as `test/part-b` declares it, empty `candidates` and no `backend`: a record `unsupported`, `settled_by` `disposition`, stage `S4`, diagnostic `unsupported_projection`/`unsupported-requested-capability` naming `temporal-satisfaction`, no engine run; no item is written for the `rely` slot; the composition is `unsupported` naming `composition`; exit 21. | Test |
| FR-1044-AC-6 | `SysCycle` over `test/system`, with no lattice profile, writes one item, `invalid-request` `invalid_model_binding`/`malformed-declaration` naming `PB::Capped` and `PA::Bounded` in edge order, and no premise item; exit 20. | Test |
| FR-1044-AC-7 | Over `test/system-lattice`, `SysLat` writes the composition at index 0 (routed, AC-1's record), `Bounded` at 1 and `Capped` at 2, each `unsupported` naming `temporal-satisfaction` (each assumes the other), and at index 3 the `fixed-point` item: a routed `composition` item of family `protocol` with a `request` member whose `premises` are the two component-guarantee slot keys and whose `options` are `{"profile": "fixed-point-lattice"}`, empty `candidates`, `unsupported` naming `composition`; the item at index 0 holds the same two slot keys and no `fixed-point` slot; `SysLatAG` writes the same items with index 0 planner-local as in AC-2, and index 3 the same routed `fixed-point` item; each exits 21. | Test |
| FR-1044-AC-8 | The items `Sys` twice (`kind` `composition`) write two composition items at indices 0 and 1 and the `Bounded` and `Capped` items once, at indices 2 and 3. With `SysRely` as a third entry, the compositions are at 0, 1 and 2, `Bounded` at 3 (shared, written once) and `Capped` at 4 `proved`, and at 5 a second `Capped`, from `SysRely`'s group, whose `request` member equals item 4's and whose record is `unsupported` as AC-5 states. | Test |
| FR-1044-AC-9 | With `test/part-b`'s `Capped` replaced by `always holds(c.value <= 2)`, `Sys` writes `Capped` `refuted` with a counterexample ending in `value = 3` and `Bounded` `proved`, each keeping its own extent and certification, and the composition `unsupported` and not `refuted`; the document exits 21. | Test |
| FR-1044-AC-10 | Over `test/system-default`, a request with no `items` writes `Local` at index 0, `Sys` at 1, the subject's derived deadlock-freedom item at 2 (FR-620-AC-5), then the premise items `Bounded` at 3 and `Capped` at 4, and exits 21; over the unit's `package` handoff (FR-850) it writes the same items, records and exit code. | Test |

## Dependencies

- FR-530 (compile, emit and read the composition declaration and its derived
  item and slots); FR-620 and FR-621 (the analyze request, item resolution,
  default items and outcome document); FR-507 (the family and the
  `inapplicable-kind` rule; its rows for a composition are added with this
  requirement); FR-630 (the precedent for selecting a declaration by name);
  FR-850 (the package route); FR-281 and FR-285 (negotiation and exit codes).
- FSpec FR-348 (the routed composition obligation; AC-1 and AC-2 stay with its
  provider), FSpec FR-491 (the declaration, derived item, slots and cycles),
  FSpec FR-492 (the planner settlement an `assume-guarantee` item uses),
  FSpec FR-489 (the premise rule), FSpec FR-290 (the family) and FSpec FR-331
  (the `composition` item and its `premises`).
