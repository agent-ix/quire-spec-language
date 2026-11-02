---
id: SR-202610310156
title: "Gap analysis — QSL protocol refinement acceptance coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-147, FR-148, FR-214; qsl-analyze/, qsl-eval/, qsl-replay/, qsl-semantics/; tests/"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-147, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-148, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-214, type: references }
---

## Summary

Planless repository-driven matrix and changed-surface gap review for QSL-563 PR #2034. The computed whole-repository matrix reports 918 untagged criteria; eight are in the FR-147/148 acceptance subset requested for this PR review, while FR-214-AC-1..5 are tagged. The changed-surface trace audit also found two new bare FR identifiers in test comments. Plan completion was not assessed.

## Verdict

**FAIL** — the selected FR-147/148 criteria include eight untagged obligations, which the matrix classifies as high-severity coverage gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-147-AC-1 has no test carrying its criterion trace tag in the computed matrix. The named TC does not establish traceability for this criterion; add a behavior test tagged with FR-147-AC-1. | FR-147-AC-1 |
| FND-002 | high | FR-147-AC-3 has no test carrying its criterion trace tag in the computed matrix. The named TC does not establish traceability for this criterion; add a behavior test tagged with FR-147-AC-3. | FR-147-AC-3 |
| FND-003 | high | FR-147-AC-9 has no test carrying its criterion trace tag in the computed matrix. The named TC does not establish traceability for this criterion; add a behavior test tagged with FR-147-AC-9. | FR-147-AC-9 |
| FND-004 | high | FR-147-AC-10 has no test carrying its criterion trace tag in the computed matrix. The named TC does not establish traceability for this criterion; add a behavior test tagged with FR-147-AC-10. | FR-147-AC-10 |
| FND-005 | high | FR-147-AC-15 has no test carrying its criterion trace tag in the computed matrix. The named TC does not establish traceability for this criterion; add a behavior test tagged with FR-147-AC-15. | FR-147-AC-15 |
| FND-006 | high | FR-147-AC-16 has no test carrying its criterion trace tag in the computed matrix. The named TC does not establish traceability for this criterion; add a behavior test tagged with FR-147-AC-16. | FR-147-AC-16 |
| FND-007 | high | FR-148-AC-13 has no test carrying its criterion trace tag in the computed matrix. The named TC does not establish traceability for this criterion; add a behavior test tagged with FR-148-AC-13. | FR-148-AC-13 |
| FND-008 | high | FR-148-AC-14 has no test carrying its criterion trace tag in the computed matrix. The named TC does not establish traceability for this criterion; add a behavior test tagged with FR-148-AC-14. | FR-148-AC-14 |
| FND-009 | medium | The new test comment contributes a bare FR-147 trace id, which is not a criterion and appears as an untracked symbol. The #[trace] attribute correctly binds FR-148-AC-1; remove the bare FR-147 id from the comment or replace it with the exact criterion it documents. | qsl-analyze/tests/it/refinement_protocol.rs::a_node_row_matches_the_attempt_identity_and_binder |
| FND-010 | medium | The new test comment contributes a bare FR-148 trace id, which is not a criterion and appears as an untracked symbol. The #[trace] attribute correctly binds FR-148-AC-1; remove the bare FR-148 id from the comment or replace it with the exact criterion it documents. | qsl-analyze/tests/it/refinement_protocol.rs::an_any_row_can_take_a_visible_protocol_transition |
## Coverage

- Reconciliation: quoin matrix (quoin 0.29.1, quire 0.37.1); whole-repository static axis plus evidence audit.
- Plan completion: not assessed
- Whole-repository criteria: tagged 2614 · untagged 918 · tagged-by-ignored-test 0 · method-without-symbol 73.
- Evidence: no run evidence 3605; the evidence store has no bindings for this repository.
- Requested FR subset: FR-147 has 10 tagged / 6 untagged; FR-148 has 12 tagged / 2 untagged; FR-214 has 5 tagged / 0 untagged.
- Outside the requested FR subset, the whole-repository matrix reports 910 additional untagged criteria; they are not itemized in this subset review.
- Untraced behaviors / stubs: 0 among changed production and test paths reviewed; the source paths map to FR-147/148/214.
- Semantic review: skipped (optional intent↔test↔code pass was not requested); the code-review lane separately identified the protocol-side FR-148-AC-12 gap.

## Matrix review data

```yaml
clean: false
scope:
- id: FR-147-AC-1
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: In a build whose support set holds `protocol-abstract-side` and `refinement-internal-list`,
    `CasTwice` checks with `AbstractSide::Protocol` naming `Twice` and `abstract_internal
    = [Twice::Done]`, and writes one requirement record of kind `refinement` and no
    `temporal-satisfaction` record for the declaration. A row `step Impl::Counter::commitA
    -> node Spec::Twice::a1(self);` checks, naming node `a1`.
- id: FR-147-AC-2
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: 'Refusals: `abstract Spec::Thrice` (`missing_declaration`/`missing-name`);
    an internal list naming `Spec::Twice::zz` (`missing_declaration`/`missing-name`);
    an internal list naming `Spec::Twice::a1` beside a row `-> node Spec::Twice::a1(self)`
    (`invalid_model_binding`/`conflicting-binding`, naming the row and the entry);
    `-> node Spec::Twice::a1(self)` in `CasRefinesCounter`, whose abstract side is
    a model (`invalid_model_binding`/`malformed-declaration`). In a build whose support
    set omits `refinement-internal-list`, a non-empty list on `CasRef'
- id: FR-147-AC-3
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: In a build that holds both features, `CasTwice` with `ensure fair weak
    Spec::Counter::inc` settles its liveness half `unsupported`, `unsupported-requested-capability`,
    naming hidden abstract fields, beside the safety result of FR-148.
- id: FR-147-AC-4
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: In a build that holds both features, `CasTwice` with `finish` removed from
    its `refinement_internals` list checks, with `finish` visible and `abstract_internal`
    empty; with the list naming `finish` it checks with `finish` internal; with the
    list naming `finish` and a step row `-> node Spec::Twice::Done(self)` it refuses
    `invalid_model_binding`/`conflicting-binding`.
- id: FR-147-AC-5
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: 'Resolution of the shape, in any build (the resolver only). For `CasTwice`,
    the value above resolves to `abstract_internal = [Twice::Done]`. The value absent,
    `[]`, and `[{"refinement": "CasRefinesCounter", "internal": []}]` each resolve
    `CasTwice` to the empty list. A value naming `["Spec::Twice::Done", "lose Spec::Twice::ch"]`
    and its permutation, and the same two inside two arrays swapped, resolve to the
    same list, in the byte order of `Twice::Done` then `lose Twice::ch`. For a refinement
    with the lower-case alias `spec` over `Twice`, the val'
- id: FR-147-AC-6
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: 'Defects, in any build, each refused with no list, located at (the unit''s
    value, pointer): `{"refinement": "CasTwice"}` at `/0`, `"internal": "Spec::Twice::Done"`
    at `/0/internal`, a non-string entry at `/0/internal/0`, `"refinement": 7` at
    `/0/refinement`, and an object key `visible` at `/0` (each `invalid_model_binding`/`malformed-declaration`);
    the entries `""`, `frob x`, `Spec::Twice` and ` Spec::Twice::Done` (each `malformed-declaration`
    at its pointer); `{"refinement": "Nope", ...}` at `/0/refinement`, the entries
    `Other::Twice::Done`, `Sp'
- id: FR-147-AC-7
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: In a build whose support set omits the features named, the refinement check
    given a non-empty resolved list for `CasRefinesCounter` (model side), in a build
    whose support set omits `refinement-internal-list`, refuses exactly once, `unsupported_construct`/`not-yet-implemented`
    at the first token of the name after `abstract` (its model alias), naming `refinement-internal-list`,
    the refinement and the profile identity, classifies `unsupported` (FR-341-AC-5),
    produces no `CheckedRefinement` and no `invalid_model_binding`. Given a non-empty
    resolved
- id: FR-147-AC-8
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: 'In any build, each class spells and resolves over `Full` and `Rich`: a
    node path for an attempt, event, send, receive, timeout, finish and fence node
    and a template path for a `cattempt`, and `fork`, `join`, `cend`, `duplicate`,
    `lose`, `spawn`, `retire`, `activate` and `memory flush` entries, each making
    exactly its own class internal; the resolved list for all of them is in the byte
    order of their alias-free spellings.'
- id: FR-147-AC-9
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: 'In a build that holds both features, `abstract_internal` is alias-free:
    for `CasTwice`, `Twice::Done`. Renaming the abstract alias `Spec` to `S`, with
    the value''s spellings renamed to match, gives the same `abstract_internal` and
    the same refinement node identity; the empty list and `[Twice::Done]` give different
    node identities.'
- id: FR-147-AC-10
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: 'In any build, the locus of a list defect is the pair (input, pointer):
    a defect in the unit''s value is located at (the unit''s value, its pointer);
    the same defect in a supplied library''s value is located at (that library''s
    value, its pointer) and refuses the whole compilation, the unit included, whether
    or not the unit imports that library (a unit with no import of it is refused all
    the same). No refusal names a compile-request member.'
- id: FR-147-AC-11
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: 'Precedence, in a build whose support set omits `refinement-internal-list`:
    a list on `CasTwice` (protocol subject) with an unresolved entry (`Spec::Twice::zz`)
    reports `missing_declaration`/`missing-name` and no `not-yet-implemented` naming
    `refinement-internal-list`, and, because the build also omits `protocol-abstract-side`,
    the independent `not-yet-implemented` refusal for that feature at `Twice`; a value
    attributable to no refinement (`{}`, `"refinement": 7`) reports its defect alone;
    a non-empty resolved list on `CasRefinesCounter` reports'
- id: FR-147-AC-12
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: In a build that holds `refinement-internal-list`, a non-empty list for
    `CasRefinesCounter` (model side) refuses `invalid_model_binding`/`malformed-declaration`
    at (the unit's value, `/0`), with no `CheckedRefinement` and no `not-yet-implemented`;
    an empty list does not refuse.
- id: FR-147-AC-13
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: In a build that holds `protocol-abstract-side`, `CasTwice` checks to the
    same `CheckedRefinement` whether `protocol Twice` is declared before or after
    the refinement in the unit's source. With `Twice` refused by its own checks, the
    unit reports `Twice`'s refusals and no `missing-name` for the clause name and
    no `CheckedRefinement` for `CasTwice`.
- id: FR-147-AC-14
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: 'In a build that holds `protocol-abstract-side`, over `Twice`, whose attempt
    `a1` applies `Spec::Counter::inc` (no parameter and no result, so its binder record
    is the receiver alone), the rows `-> node Spec::Twice::a1(self)`, `-> node Spec::Twice::a1(_)`
    and `-> node Spec::Twice::a1` are admitted; `-> node Spec::Twice::a1(self, 1,
    2)` and `-> node Spec::Twice::a1(self, _)` each refuse `ill_typed`/`operator-ineligible`
    at the node retaining `expected` 1 and `actual` 3 and 2; `-> node Spec::Twice::a1(true)`
    refuses `ill_typed`/`type-mismatch` at '
- id: FR-147-AC-15
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: In a build that holds `protocol-abstract-side` and `refinement-internal-list`,
    with `Spec` and `Twice` supplied as a dependency package (FR-135-AC-1's dependency
    form) and declared nowhere in the unit, `CasTwice` checks with `AbstractSide::Protocol`
    whose `dependency` is that package's `package_id` and whose `clause` is the key
    of `Twice` the dependency's `CheckedGraph` holds, with `abstract_internal = [Twice::Done]`;
    the row `step Impl::Counter::commitA -> node Spec::Twice::a1(self);` checks, naming
    node `a1`, and `-> node Spec::Twice::a1(true
- id: FR-147-AC-16
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  role: examined
  excerpt: In a build that holds `protocol-abstract-side`, a dependency package that
    imports the models `Spec` and `Other` and declares a protocol `OnOther` whose
    roles act only on `Other` refuses `abstract Spec::OnOther` `invalid_model_binding`/`malformed-declaration`
    at `OnOther`, with no `CheckedRefinement`; with `Spec` local, a protocol `OnImpl`
    of the declaring package whose roles act only on the concrete model `Impl` refuses
    `abstract Spec::OnImpl` the same way, at `OnImpl`. A dependency whose `Twice`
    fails its own checks gives `CompileRefusal::Depe
- id: FR-148-AC-1
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: '`check_initial` on `CasTwice`''s concrete initial state gives one abstract
    state, at rest at `a1` with `value` 0. After `beginA`, `commitA` the set holds
    the state at rest at `a2` with `value` 1; after a second commit, the state with
    `value` 2 and the protocol finished, since the closure takes the internal `finish`.'
- id: FR-148-AC-2
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: '`CasTwice` settles `proved`, basis `closed-scope`, as QSpec FR-379 settles
    a completed explicit-state run with no failure, holding for the concrete initial
    states and universe `counters = {c}`, labelled `Uncertified` (ADR-018 PC-1), since
    FR-149 does not certify protocol-subject step decisions and the further checks
    of QSpec FR-177 have no core certificate checker (ADR-020 CT-4).'
- id: FR-148-AC-3
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: The lost-update model (ADR-020 §8) with `peek` removed, against `Twice`
    with the same rows, settles `refuted` with `NoAbstractMatch` at the commit that
    leaves `value` unchanged, and the counterexample replays to `reproduced-with-evaluated-witness`.
- id: FR-148-AC-4
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: '`CasTwice` against the protocol `Once` (`a1` then `finish`) settles `refuted`
    with `NoAbstractMatch{position: 4}`, whose step is the second commit, and the
    counterexample replays.'
- id: FR-148-AC-5
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: '`CasTwice` with `peek` kept in the concrete model and mapped `-> stutter`
    settles `refuted`, basis `decisive-counterexample`, by QSpec FR-177''s divergence-freedom
    check with divergence observable, with a lasso whose loop is one `peek` step at
    the initial state, where the abstract `a1` is enabled. It refutes with no `assume`
    or `ensure` row present, because QSpec FR-177-AC-6 refutes an observable internal
    loop under every fairness set, including an empty one.'
- id: FR-148-AC-6
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: When the abstract protocol has one thread whose only enabled node maps
    to concrete operation `inc`, its derived weak whole scheduler constraint implies
    the matching concrete weak whole `assume inc` over every reachable paired lasso;
    the assumption-weakening check passes. Separately, a directly required temporal
    clause with `fair weak whole inc` contributes exactly its checked operation-class
    row, identified by the clause `NodeKey` and the row's `FairnessOperation`; an
    unrelated temporal clause's fairness row contributes nothing.
- id: FR-148-AC-7
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: 'When the abstract protocol declares adversarial scheduling, has no memory
    constraint and has no directly required temporal clause, while the concrete model
    declares weak whole fairness for an operation that remains enabled on an infinite
    stutter loop, the loop satisfies empty `S` and violates `I`; the check returns
    `inconclusive`, `unsettled`, with `ProtocolAssumptionNotEstablished{reason: strengthened}`
    and the exact identity plus paired path, and replay reproduces the same predicates.
    An unrelated temporal clause''s fairness row is not in `S`.'
- id: FR-148-AC-8
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: 'With concrete `assume` constraints on two distinct operation classes and
    an abstract protocol that directly requires only a temporal clause carrying `fair
    weak whole left`, a reachable lasso that satisfies this checked row but violates
    the `right` concrete assumption yields `ProtocolAssumptionNotEstablished{reason:
    strengthened}` with the exact required clause `NodeKey`, checked left `FairnessOperation`,
    and right implementation identity plus paired paths; an unrelated clause carrying
    `fair weak whole right` is excluded. When a product limit pr'
- id: FR-148-AC-9
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: For a single finite operation-class domain, the first abstract protocol
    directly requires a temporal clause carrying `fair strong each inc`, whose exact
    source identity includes its clause `NodeKey` and checked `FairnessOperation`;
    this specification constraint implies a weak whole `assume inc` on the implementation
    side. In a separate protocol requiring a clause carrying `fair weak whole inc`,
    the specification constraint does not imply strong implementation fairness on
    a lasso where `inc` is enabled infinitely often but not continuously and i
- id: FR-148-AC-10
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: 'For an abstract `tso` protocol with `scheduling adversarial` and no directly
    required temporal fairness clause, the specification assumption set includes the
    derived `fair weak each flush(thread)` memory constraint. A reachable paired lasso
    that cycles a buffered `tick` write and its flush, never takes the continuously
    enabled `inc`, satisfies the memory fairness constraint but violates the concrete
    `assume fair weak whole inc`; the product returns `ProtocolAssumptionNotEstablished{reason:
    strengthened}` with exact identities and paired paths, '
- id: FR-148-AC-11
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: At each reachable product state whose incoming step rules passed, a refusal
    violation reports `RefusalNotPermitted{position, label}` with QSpec FR-379's canonical
    full `ObservationLabel`; a terminal-success mismatch reports `TerminalSuccessMismatch{position,
    direction}`. The first failing step rule takes precedence, then refusal precedes
    terminal success, and replay recomputes each recorded condition; model abstract
    subjects report neither kind.
- id: FR-148-AC-12
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: 'When an identified concrete or protocol fairness class has `ContractUndetermined`
    enabledness at a reachable paired state and no violating paired lasso exists,
    the product returns `ProtocolAssumptionNotEstablished{reason: relation-undetermined}`
    with the exact checked identities and both finite prefixes to the undecided state;
    FR-145 replay reproduces the same undecided predicate and the item settles `inconclusive`,
    `unsettled`, never `refuted` or `proved`. This is distinct from product-limit
    `incomplete`.'
- id: FR-148-AC-13
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: An undeclared or alias-only `requires temporal` name refuses at S3 per
    FR-218-AC-16; a bound requirement `NodeKey` with no emitted temporal node refuses
    at S4 per FR-383-AC-17. Each refusal occurs before product evaluation and produces
    no `ProtocolAssumptionNotEstablished` result. The product uses only admitted clause
    `NodeKey`s, never a clause or target name. A uniquely resolved required clause
    with no fairness rows contributes no authored constraint.
- id: FR-148-AC-14
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  role: examined
  excerpt: When a required temporal clause contributes `fair weak whole inc` to authored
    `S`, a separate `ensure fair weak whole inc` guarantee is satisfied by the checked
    lasso. The `ensure` row is absent from both `I` and `S`; the same assumption relation,
    source identities and result occur with the guarantee row present or removed.
- id: FR-214-AC-1
  path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md
  role: examined
  excerpt: A refinement of an abstract model `Abs`, whose object type `Acc` has operation
    `add()`, by ADR-027 §7's `Fill` over model `M`, with FR-135's population row for
    `Abs`'s population and object row mapping `Acc` from `Cell` (which let FR-136
    type the `self` receiver against `Abs::Acc`), and with step rows `step M::Cell::inc
    -> Abs::Acc::add(self)`, `step M::Cell::setTwo -> Abs::Acc::add(self)`, `step
    fork M::Fill::Both -> stutter`, `step join M::Fill::Both -> stutter` and `step
    M::Fill::Done -> stutter`, checks; every concrete step class has exactl
- id: FR-214-AC-2
  path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md
  role: examined
  excerpt: Removing the `join` row refuses `missing_declaration`/`missing-name` naming
    `Both`'s join; removing the `finish` row refuses naming `Done`. Adding a second
    `step M::Cell::inc -> …` row refuses `invalid_model_binding`/`conflicting-binding`.
- id: FR-214-AC-3
  path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md
  role: examined
  excerpt: Adding `step M::Fill::A -> stutter` beside `step M::Cell::inc -> Abs::Acc::add(self)`
    maps `A`'s steps by the node row (`Internal`) and leaves the `inc` operation row
    unused by `A`. Over ADR-027 §7.1, omitting the `Refund` template's `cend` row
    refuses naming the template; over `Chatter` (FR-212), omitting either of `ch`'s
    two channel rows (`duplicate` and `lose`, since its delivery is `unknown`) refuses
    naming `ch`.
- id: FR-214-AC-4
  path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md
  role: examined
  excerpt: A row `-> any` gives `Either`. An abstract protocol whose internal list
    names `fork` as internal gives that abstract step internal, and its other abstract
    steps visible.
- id: FR-214-AC-5
  path: spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md
  role: examined
  excerpt: The rows of FR-214-AC-1 check whether the `Fill` protocol is declared before
    or after the refinement in the unit's source; with `Fill` refused by its own checks,
    the unit reports `Fill`'s refusals and no coverage refusal (FR-136-AC-8).
- id: a_node_row_matches_the_attempt_identity_and_binder
  path: qsl-analyze/tests/it/refinement_protocol.rs
  role: examined
  excerpt: '/// FR-147/FR-148 (TC-553): a node row compares its expression with the

    /// attempt''s typed receiver binder before accepting the protocol node.'
- id: an_any_row_can_take_a_visible_protocol_transition
  path: qsl-analyze/tests/it/refinement_protocol.rs
  role: examined
  excerpt: '/// FR-148 (TC-553): `any` admits the union of internal stutter and visible

    /// protocol matches; this `commitA` takes the visible `a1` branch.'
- id: FR-379-AC-8
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: QSpec FR-379 AC-8 specifies settled proved outcomes for completed runs
    with no failure.
- id: FR-379-AC-9
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: QSpec FR-379 AC-9 specifies settled refuted outcomes with decisive finite
    counterexamples.
- id: FR-379-AC-10
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: QSpec FR-379 AC-10 specifies replayable counterexamples for finite step
    failures.
- id: FR-379-AC-11
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: QSpec FR-379 AC-11 specifies replayable protocol refusal witnesses.
- id: FR-379-AC-12
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: QSpec FR-379 AC-12 specifies replayable terminal mismatch witnesses.
- id: FR-379-AC-13
  path: quire-specification/spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md
  role: context_only
  excerpt: QSpec FR-379 AC-13 specifies replay reproduces reported result and identity
    data.
- id: TC-318
  path: quire-specification/spec/test-cases/TC-318-protocol-refinement-step-rules.md
  role: context_only
  excerpt: Protocol refinement step cases exercise FR-379 AC8-13 settled outcomes
    and replay parity for protocol witnesses.
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-147-AC-1
  related_ids:
  - TC-552
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  lines: '319'
  excerpt: In a build whose support set holds `protocol-abstract-side` and `refinement-internal-list`,
    `CasTwice` checks with `AbstractSide::Protocol` naming `Twice` and `abstract_internal
    = [Twice::Done]`, and writes one requirement record of kind `refinement` and no
    `temporal-satisfaction` record for the declaration. A row `step Impl::Counter::commitA
    -> node Spec::Twice::a1(self);` checks, naming node `a1`.
  finding: FR-147-AC-1 has no test carrying its criterion trace tag in the computed
    matrix. The named TC does not establish traceability for this criterion; add a
    behavior test tagged with FR-147-AC-1.
- fnd: FND-002
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-147-AC-3
  related_ids:
  - TC-552
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  lines: '321'
  excerpt: In a build that holds both features, `CasTwice` with `ensure fair weak
    Spec::Counter::inc` settles its liveness half `unsupported`, `unsupported-requested-capability`,
    naming hidden abstract fields, beside the safety result of FR-148.
  finding: FR-147-AC-3 has no test carrying its criterion trace tag in the computed
    matrix. The named TC does not establish traceability for this criterion; add a
    behavior test tagged with FR-147-AC-3.
- fnd: FND-003
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-147-AC-9
  related_ids:
  - TC-552
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  lines: '327'
  excerpt: 'In a build that holds both features, `abstract_internal` is alias-free:
    for `CasTwice`, `Twice::Done`. Renaming the abstract alias `Spec` to `S`, with
    the value''s spellings renamed to match, gives the same `abstract_internal` and
    the same refinement node identity; the empty list and `[Twice::Done]` give different
    node identities.'
  finding: FR-147-AC-9 has no test carrying its criterion trace tag in the computed
    matrix. The named TC does not establish traceability for this criterion; add a
    behavior test tagged with FR-147-AC-9.
- fnd: FND-004
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-147-AC-10
  related_ids:
  - TC-552
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  lines: '328'
  excerpt: 'In any build, the locus of a list defect is the pair (input, pointer):
    a defect in the unit''s value is located at (the unit''s value, its pointer);
    the same defect in a supplied library''s value is located at (that library''s
    value, its pointer) and refuses the whole compilation, the unit included, whether
    or not the unit imports that library (a unit with no import of it is refused all
    the same). No refusal names a compile-request member.'
  finding: FR-147-AC-10 has no test carrying its criterion trace tag in the computed
    matrix. The named TC does not establish traceability for this criterion; add a
    behavior test tagged with FR-147-AC-10.
- fnd: FND-005
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-147-AC-15
  related_ids:
  - TC-552
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  lines: '333'
  excerpt: In a build that holds `protocol-abstract-side` and `refinement-internal-list`,
    with `Spec` and `Twice` supplied as a dependency package (FR-135-AC-1's dependency
    form) and declared nowhere in the unit, `CasTwice` checks with `AbstractSide::Protocol`
    whose `dependency` is that package's `package_id` and whose `clause` is the key
    of `Twice` the dependency's `CheckedGraph` holds, with `abstract_internal = [Twice::Done]`;
    the row `step Impl::Counter::commitA -> node Spec::Twice::a1(self);` checks, naming
    node `a1`, and `-> node Spec::Twice::a1(true
  finding: FR-147-AC-15 has no test carrying its criterion trace tag in the computed
    matrix. The named TC does not establish traceability for this criterion; add a
    behavior test tagged with FR-147-AC-15.
- fnd: FND-006
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-147-AC-16
  related_ids:
  - TC-552
  path: spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md
  lines: '334'
  excerpt: In a build that holds `protocol-abstract-side`, a dependency package that
    imports the models `Spec` and `Other` and declares a protocol `OnOther` whose
    roles act only on `Other` refuses `abstract Spec::OnOther` `invalid_model_binding`/`malformed-declaration`
    at `OnOther`, with no `CheckedRefinement`; with `Spec` local, a protocol `OnImpl`
    of the declaring package whose roles act only on the concrete model `Impl` refuses
    `abstract Spec::OnImpl` the same way, at `OnImpl`. A dependency whose `Twice`
    fails its own checks gives `CompileRefusal::Depe
  finding: FR-147-AC-16 has no test carrying its criterion trace tag in the computed
    matrix. The named TC does not establish traceability for this criterion; add a
    behavior test tagged with FR-147-AC-16.
- fnd: FND-007
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-148-AC-13
  related_ids:
  - TC-553
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  lines: '267'
  excerpt: An undeclared or alias-only `requires temporal` name refuses at S3 per
    FR-218-AC-16; a bound requirement `NodeKey` with no emitted temporal node refuses
    at S4 per FR-383-AC-17. Each refusal occurs before product evaluation and produces
    no `ProtocolAssumptionNotEstablished` result. The product uses only admitted clause
    `NodeKey`s, never a clause or target name. A uniquely resolved required clause
    with no fairness rows contributes no authored constraint.
  finding: FR-148-AC-13 has no test carrying its criterion trace tag in the computed
    matrix. The named TC does not establish traceability for this criterion; add a
    behavior test tagged with FR-148-AC-13.
- fnd: FND-008
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-148-AC-14
  related_ids:
  - TC-553
  path: spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md
  lines: '268'
  excerpt: When a required temporal clause contributes `fair weak whole inc` to authored
    `S`, a separate `ensure fair weak whole inc` guarantee is satisfied by the checked
    lasso. The `ensure` row is absent from both `I` and `S`; the same assumption relation,
    source identities and result occur with the guarantee row present or removed.
  finding: FR-148-AC-14 has no test carrying its criterion trace tag in the computed
    matrix. The named TC does not establish traceability for this criterion; add a
    behavior test tagged with FR-148-AC-14.
- fnd: FND-009
  severity: medium
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: qsl-analyze/tests/it/refinement_protocol.rs::a_node_row_matches_the_attempt_identity_and_binder
  related_ids:
  - FR-147
  - FR-148-AC-1
  path: qsl-analyze/tests/it/refinement_protocol.rs
  lines: 1820-1824
  excerpt: '/// FR-147/FR-148 (TC-553): a node row compares its expression with the

    /// attempt''s typed receiver binder before accepting the protocol node.'
  finding: 'The new test comment contributes a bare FR-147 trace id, which is not
    a criterion and appears as an untracked symbol. The #[trace] attribute correctly
    binds FR-148-AC-1; remove the bare FR-147 id from the comment or replace it with
    the exact criterion it documents.'
  test_id: a_node_row_matches_the_attempt_identity_and_binder
  claimed_ac: FR-147
  correct_ac: FR-148-AC-1
- fnd: FND-010
  severity: medium
  confidence: high
  method: gap-analysis
  check: trace
  artifact_id: qsl-analyze/tests/it/refinement_protocol.rs::an_any_row_can_take_a_visible_protocol_transition
  related_ids:
  - FR-148
  - FR-148-AC-1
  path: qsl-analyze/tests/it/refinement_protocol.rs
  lines: 1732-1736
  excerpt: '/// FR-148 (TC-553): `any` admits the union of internal stutter and visible

    /// protocol matches; this `commitA` takes the visible `a1` branch.'
  finding: 'The new test comment contributes a bare FR-148 trace id, which is not
    a criterion and appears as an untracked symbol. The #[trace] attribute correctly
    binds FR-148-AC-1; remove the bare FR-148 id from the comment or replace it with
    the exact criterion it documents.'
  test_id: an_any_row_can_take_a_visible_protocol_transition
  claimed_ac: FR-148
  correct_ac: FR-148-AC-1
bindings:
- test_id: an_internal_entry_beside_its_node_row_refuses
  ac_id: FR-147-AC-2
  trace: correct
- test_id: each_refusal_of_the_protocol_side_is_located
  ac_id: FR-147-AC-2
  trace: correct
- test_id: a_node_the_list_does_not_name_is_visible
  ac_id: FR-147-AC-4
  trace: correct
- test_id: the_list_resolves_in_the_byte_order_of_its_alias_free_spellings
  ac_id: FR-147-AC-5
  trace: correct
- test_id: each_defect_of_the_value_is_refused_at_its_pointer
  ac_id: FR-147-AC-6
  trace: correct
- test_id: a_shape_defect_of_a_named_object_is_reported_once
  ac_id: FR-147-AC-6
  trace: correct
- test_id: an_unbuilt_refinement_feature_is_unsupported_and_a_list_defect_is_refused
  ac_id: FR-147-AC-7
  trace: correct
- test_id: a_build_without_the_features_refuses_a_non_empty_list_once
  ac_id: FR-147-AC-7
  trace: correct
- test_id: each_class_spells_and_resolves_over_a_protocol_that_has_it
  ac_id: FR-147-AC-8
  trace: correct
- test_id: a_memory_drain_entry_makes_the_drain_class_internal_under_every_model
  ac_id: FR-147-AC-8
  trace: correct
- test_id: the_timeout_cattempt_cend_and_activate_classes_resolve
  ac_id: FR-147-AC-8
  trace: correct
- test_id: resolution_defects_are_reported_before_the_unbuilt_feature
  ac_id: FR-147-AC-11
  trace: correct
- test_id: the_defects_of_the_value_are_reported_beside_another_refinements_refusal
  ac_id: FR-147-AC-11
  trace: correct
- test_id: a_non_empty_list_on_a_model_subject_is_malformed_once_the_feature_is_built
  ac_id: FR-147-AC-12
  trace: correct
- test_id: a_refused_abstract_clause_reports_alone_over_a_concrete_protocol
  ac_id: FR-147-AC-13
  trace: correct
- test_id: the_protocol_is_checked_before_the_refinement_whatever_the_source_order
  ac_id: FR-147-AC-13
  trace: correct
- test_id: a_clause_refused_by_its_controls_check_gives_no_name_error
  ac_id: FR-147-AC-13
  trace: correct
- test_id: a_node_row_whose_arguments_fit_the_binder_record_checks
  ac_id: FR-147-AC-14
  trace: correct
- test_id: a_node_row_with_the_wrong_argument_count_is_refused_at_the_node
  ac_id: FR-147-AC-14
  trace: correct
- test_id: a_node_row_argument_of_the_wrong_type_is_refused_at_the_argument
  ac_id: FR-147-AC-14
  trace: correct
- test_id: the_binder_record_has_a_field_per_parameter_result_and_record_field
  ac_id: FR-147-AC-14
  trace: correct
- test_id: the_argument_count_is_the_field_count_of_every_node_kind
  ac_id: FR-147-AC-14
  trace: correct
- test_id: an_argument_of_the_wrong_type_is_refused_at_its_field
  ac_id: FR-147-AC-14
  trace: correct
- test_id: check_initial_and_steps_retain_protocol_control_candidates
  ac_id: FR-148-AC-1
  trace: correct
- test_id: an_any_row_can_take_a_visible_protocol_transition
  ac_id: FR-148-AC-1
  trace: correct
- test_id: a_node_row_matches_the_attempt_identity_and_binder
  ac_id: FR-148-AC-1
  trace: correct
- test_id: a_twice_protocol_refinement_holds_uncertified
  ac_id: FR-148-AC-2
  trace: correct
- test_id: lost_update_protocol_counterexample_replays
  ac_id: FR-148-AC-3
  trace: correct
- test_id: a_second_commit_against_once_replays
  ac_id: FR-148-AC-4
  trace: correct
- test_id: a_stuttering_peek_loop_is_protocol_observable_divergence
  ac_id: FR-148-AC-5
  trace: correct
- test_id: protocol_scheduler_fairness_can_imply_a_concrete_assumption
  ac_id: FR-148-AC-6
  trace: correct
- test_id: a_stronger_concrete_assumption_never_settles_proved
  ac_id: FR-148-AC-7
  trace: correct
- test_id: one_protocol_class_does_not_imply_a_second_concrete_operation_assumption
  ac_id: FR-148-AC-8
  trace: correct
- test_id: a_protocol_product_limit_stops_assumption_decision_incomplete
  ac_id: FR-148-AC-8
  trace: correct
- test_id: required_strong_each_fairness_implies_concrete_weak_whole
  ac_id: FR-148-AC-9
  trace: correct
- test_id: required_weak_whole_fairness_does_not_imply_concrete_strong_each
  ac_id: FR-148-AC-9
  trace: correct
- test_id: tso_flush_fairness_replays_against_an_unmet_inc_assumption
  ac_id: FR-148-AC-10
  trace: correct
- test_id: a_terminal_refusal_of_a_protocol_event_replays
  ac_id: FR-148-AC-11
  trace: correct
- test_id: a_refusal_is_permitted_when_one_stable_protocol_candidate_refuses
  ac_id: FR-148-AC-11
  trace: correct
- test_id: an_incoming_step_rule_failure_precedes_refusal_at_the_same_state
  ac_id: FR-148-AC-11
  trace: correct
- test_id: an_intended_protocol_terminal_mismatch_replays
  ac_id: FR-148-AC-11
  trace: correct
- test_id: a_concrete_terminal_success_mismatch_replays
  ac_id: FR-148-AC-11
  trace: correct
- test_id: model_abstract_refinements_exclude_protocol_refusal_and_terminal_failures
  ac_id: FR-148-AC-11
  trace: correct
- test_id: an_undetermined_assumption_predicate_has_a_typed_finite_witness
  ac_id: FR-148-AC-12
  trace: correct
- test_id: a_protocol_refinement_has_one_row_per_step_class
  ac_id: FR-214-AC-1
  trace: correct
- test_id: a_missing_or_doubled_row_refuses
  ac_id: FR-214-AC-2
  trace: correct
- test_id: a_combined_refinement_needs_its_concrete_rows
  ac_id: FR-214-AC-2
  trace: correct
- test_id: an_attempt_node_row_takes_precedence_over_its_operation_row
  ac_id: FR-214-AC-3
  trace: correct
- test_id: a_template_a_trigger_and_event_nodes_need_their_own_rows
  ac_id: FR-214-AC-3
  trace: correct
- test_id: a_channel_needs_a_row_for_each_step_its_delivery_admits
  ac_id: FR-214-AC-3
  trace: correct
- test_id: a_row_target_gives_its_observation
  ac_id: FR-214-AC-4
  trace: correct
- test_id: a_concrete_protocol_refines_an_abstract_protocol
  ac_id: FR-214-AC-4
  trace: correct
- test_id: the_coverage_waits_for_the_clause_and_a_refused_clause_reports_alone
  ac_id: FR-214-AC-5
  trace: correct
```
