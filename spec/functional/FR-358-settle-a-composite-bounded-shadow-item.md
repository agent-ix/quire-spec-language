---
id: FR-358
title: "Settle a composite bounded_shadow equality-parity claim: replay a falsified one, and tell Proved from Tested for a verified one"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-149
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
  - target: ix://agent-ix/quire-contract-codegen/FR-015
    type: references
  - target: ix://agent-ix/quire-contract-codegen/FR-028
    type: references
  - target: ix://agent-ix/quire-contract-codegen/FR-029
    type: references
  - target: ix://agent-ix/quire-contract-codegen/FR-033
    type: references
---
# FR-358: Settle a composite bounded_shadow equality-parity claim: replay a falsified one, and tell Proved from Tested for a verified one

## Description

A composite `bounded_shadow` item is a Kani obligation that CG proves over a
bounded shadow model, with a refinement obligation, instead of over
production code (CG FR-015-AC-69 to AC-76, CG FR-028, CG ADR-003 Q1). The
item is a **parity claim**, not a source predicate. It says that CG's
generated shadow computes the language's composite equality (QSpec FR-149)
at the claimed node: the equality verdict under the claimed operator, and
the occurrence-pair count. Like FR-357's operator-parity claim, it is a
claim about generated code. Its counterexample is never a violation of a
user property, so no result of this requirement settles `Refuted`.

QSL owns the verdict (QSL-640; CG AD-002). This requirement adds no terminal
value. CG builds the claim as CG FR-033 describes, and its field names
follow CG FR-033. The harness bounds are part of the claim's obligation
identity (ADR-013 O-09, ADR-014 B-4). ADR-021 TX-3 states that a harness
carries a tightened bound in its identity, and ADR-021 TX-2 names the
bounded shadow path as the code-side evidence route CG ADR-003 Q1 selects.

QSL SHALL provide two entries in the layer-6 replay facade, both taking the
original proving context as an FR-071 replay request:

- `qsl_replay::replay_composite_parity` for a falsified item. It evaluates
  the language's exact equality on the operands CG decoded from the
  falsifying draw and compares the result with the retained shadow result.
  A divergence settles `Failed`. An agreement settles
  `Inconclusive(ScalarAgrees)`, widened to parity claims (below). A native
  `Incomplete` or `ExecutionFault` settles `Failed` with no comparison, as
  in FR-357's operator arm.
- `qsl_replay::settle_verified_shadow` for a verified item. It derives the
  claim's declared bounds itself, checks CG's harness bounds against them,
  and settles `Failed`, `Incomplete`, the vacuous `Proved { Checks { 0 }, Certified }`,
  `Proved { Checks { n }, Certified }` or `Tested`. It takes no native observation.

Both entries, like `qsl_replay::replay`, take a `ReplayLimits` (the
`replay.input_bytes` bound) as their last argument, so a caller can raise it;
the FR-357 entries and `qsl_replay::compile_package` take it the same way.

There is no predicate-replay route for these items. Neither entry selects a
`Boolean` function to replay, and neither calls `qsl_replay::replay`.

## Inputs

Both entries take an FR-071 replay request carrying the original proving
context: the `package_id`, package reference, dependencies and byte
provision; the enclosing function of the claimed node as
`selected_function`; the R-6 `obligation_identity` CG minted (CG AD-002
R-6); and the request's declared domains, the `DeclaredDomain`
(`ProofBound`) values FR-071 carries, which are caller-substituted ADR-014
B-4 bounds (ADR-014 §4). Neither entry reads the request's `source`.

With the request, both entries take a `CompositeParityClaim`:

| Field | Type | Meaning |
| --- | --- | --- |
| `node` | `WireNodeId` | The claimed equality node, by its QSpec FR-322 node identity in the original package |
| `occurrence` | `Origin` | The claimed occurrence of `node` in the selected function's body (its role and ordinal). A node shared by two occurrences, such as `(a = b) and (a = b)`, is two obligations, each with its own identity |
| `operator` | `EqualityOperator` (`Equal` or `NotEqual`) | The claimed operation |
| `obligation_kind` | text | The CG `ObligationKind` wire string in the O-09 preimage |
| `harness_bounds` | list of `ProofBound` | The bounds the harness ran, each keyed by its `DomainKey` (for an enum position, the variants it drew, as `FiniteBound::Variants`): a parameter node id and the child-index path into its type |
| `limits` | `ScalarLimits` | The accounting limits the exact evaluation runs under, retained from the proving run |
| `content_identity` | `DigestRecord` | The canonical proof-content identity binding the claim to the proved artifact. It is carried into the claim identity and never recomputed |

`replay_composite_parity` also takes a `FalsifiedParity`:

| Field | Type | Meaning |
| --- | --- | --- |
| `operands` | two `WitnessValue`s | The left and right operands, as CG's FR-025 leaf binding reconstructs them, in FR-070's forms |
| `shadow` | `EqualityOutcome { equal: bool, pair_count: u64 }` | The retained shadow comparison at those operands: the verdict under `operator`, and the occurrence-pair count |
| `native` | `NativeParityObservation` | The driver's observation of the same proved generated artifact at those operands under the original limits: `Completed(EqualityOutcome)`, `Refused(Code)`, `Incomplete(NativeCause)` or `ExecutionFault(NativeCause)`. `NativeCause` is the driver's descriptor of why the run stopped, carried verbatim and counted by the envelope's encoded-size bound |
| `refinement` | `Refinement` | The refinement evidence of the same harness (below) |

`settle_verified_shadow` also takes a `VerifiedShadow`:

| Field | Type | Meaning |
| --- | --- | --- |
| `success_checks` | `u32` | The SUCCESS count of the verified run |
| `refinement` | `Refinement` | The refinement evidence of the verified harness (below) |

**Refinement.** One closed enum, used by both entries, records what the
native refinement run (CG FR-028) found about the shadow against the
production code: `Exhausted` (it compared the whole bounded domain and they
agree), `NotExhausted` (they agree where it compared, but it did not
compare the whole domain, or it did not run), `CeilingReached` (it stopped
at a ceiling before it finished) or `Disagreed` (it found the shadow and
the production code disagree). CG FR-028-AC-17 names the strengths of a
`bounded_shadow` harness, and CG FR-029-AC-17 maps them today to the
interim `NonProductionProof`. They map onto `Refinement` as follows, with
none retired:

| CG FR-028-AC-17 strength | `Refinement` |
| --- | --- |
| `shadow_proved_refinement_exhaustive` | `Exhausted` |
| `shadow_proved_refinement_sampled` | `NotExhausted` |
| `shadow_proved_refinement_not_run` | `NotExhausted` |
| `shadow_proved_refinement_inconclusive` (the refinement run reached a ceiling, CG FR-028-AC-24) | `CeilingReached` |
| `refinement_failed` | `Disagreed` |

A Kani run that itself reached a backend ceiling is not a replay input. It
produces neither a verified nor a falsified item, and CG settles it under
CG FR-028-AC-2 and AC-3.

`production_proved` is not a `bounded_shadow` strength (CG FR-028-AC-17),
so it never reaches this entry. For composite items, this requirement
replaces CG FR-029-AC-17's interim `NonProductionProof` row for these
strengths and its `ShadowCounterexample` row for a falsified shadow harness.

## Outputs

- `replay_composite_parity` returns a `CompositeParityReport`. It holds the
  full claim and a `CompositeParityResult`, one per falsified row F-1 to
  F-7, or `Refused`.
- `settle_verified_shadow` returns a `VerifiedShadowReport`. It holds the
  full claim and a `VerifiedShadowResult`: `Settled` (one of rows V-1 to
  V-5, with the refinement-ceiling stage recorded for row V-2) or
  `Refused`.
- Each report has a `terminal_value` for the proof envelope (FR-069).
- `report.claim()` is a `CompositeIdentity`, the FULL claim on every
  outcome, a refusal included: the obligation identity, `node`, `occurrence`,
  `operator`, `obligation_kind`, `harness_bounds`, `limits`,
  `content_identity` and the observation as supplied. A falsified item's
  observation is its operands, `shadow`, `native` and `refinement`; a
  verified item's is its `success_checks` and `refinement`. There is no
  partial shape on any outcome. The consumer's check is
  `report.claim() == CompositeIdentity::new(obligation, &claim, evidence)`
  over what it sent, as in FR-357; a changed observation member makes them
  differ.

## Behavior

### Common steps

Both entries SHALL run these steps in order, stopping at the first refusal.
A refusal settles as `TerminalValue::from_replay_refusal` gives (FR-069, FR-121-AC-16): `Failed` for a fault (`ReplayRefusal::Fault`, or `Admission` with `AdmissionFailure::Fault`), otherwise `Inconclusive(ReplayRefused(code))`. It takes precedence over every
settlement row, including the vacuous row V-3.

1. **Recompile and tie the package.** Recompile the request's package by
   FR-098's rules, in FR-098's order: the ADR-015 D-4 dependency checks,
   then the `package_id` tie. A recompiled `package_id` that differs from
   the request's SHALL refuse `PackageIdMismatch`
   (`stale_dependency`/`content-mismatch`), naming both identities (CG
   AD-002 R-7).
2. **Select the node.** Resolve `selected_function` in the recompiled
   package, refusing an undeclared name `UnknownFunction`
   (`missing_declaration`/`missing-name`). Require `node` to be a node of
   the package, to lie in that function's body, to be an application of
   `operator` (QSpec FR-149's `=` or `≠`), and `occurrence` to be an
   occurrence of `node` in that function's body (`ScalarIdentity::Occurrence`;
   an occurrence of another function, or one the node does not have, refuses
   it). Each mismatch SHALL refuse
   `ReplayRefusal::ScalarIdentity` (`stale_dependency`/`content-mismatch`)
   with its own cause, as FR-357 does for an operator node. No `Boolean`
   function is selected or called.
3. **Derive the operand domains and positions.** Each operand of `node` is a
   parameter reference or a literal.
   - A **parameter operand**'s domain is its declared type. Walk that type
     through the transitive closure of its declarations, as ADR-014 §4's
     extent walk does, on an explicit heap stack (ADR-030 D-1). Key each
     position by `DomainKey::Node { node: parameter, path }`, with `path`
     the child-index path into the type (element, field, variant payload).
     The derived positions and their authored bounds are:
     - an `Int[lower, upper]` leaf: `IntegerRange [lower, upper]`; an
       `Integer` leaf: unbounded;
     - a `K<T>[minimum, maximum]` collection: `Cardinality` with that
       maximum; a `K<T>` collection: unbounded;
     - a recursive declaration: `Depth`, unbounded (ADR-014 §2), keyed at
       the path where the recursion is first entered (as ADR-014 §4's extent
       walk keys it), found when the walk reaches the declaration again. The
       walk does not descend past that second arrival;
     - an enum leaf: `Variants`, the set of every variant its declared enum
       admits. An enum's domain is finite, so a harness can draw all of it
       and the position can be covered;
     - every other non-`Boolean` leaf (text, rational, decimal, float,
       quantity or reference): a position whose declared domain is the
       leaf's whole declared domain (its text length and profile, rational
       or decimal domain and scales, float width, unit, or object type).
       Such a position can be covered only by a harness bound of a kind
       that describes its whole declared domain, compared as an
       `IntegerRange` or `Cardinality` is. ADR-014 B-4 defines no such kind
       today. A declared text length or decimal range limits one dimension
       of the domain, not every value in it: a harness that reaches the
       maximum length has still not shown it drew every scalar, scale or
       coefficient the profile admits. So these positions are never
       covered, and a claim with one settles `Tested`, never `Proved` (row
       V-5). A harness bound keyed at one refuses as a kind mismatch
       (step 5).

     A `Boolean` leaf derives no position: its two values are its whole
     domain, and the harness draws both.
   - A **literal operand**'s domain is the singleton holding its value in
     the recompiled package. It derives no position: the harness pins its
     leaves to that value (CG FR-015-AC-70), and the enclosing type's range
     never stands in for it.
4. **Declared bounds.** Each derived position's declared bound is the
   request's `DeclaredDomain` for its key, where the request carries one. A
   caller-substituted B-4 bound is the claim's own domain (ADR-014 §4,
   §8). Otherwise the declared bound is the position's authored bound, or
   unbounded. Refuse, naming the key, a request `DeclaredDomain` whose key
   is not a derived position, one over a position that has an authored
   bound (B-4 replaces only an unbounded domain, ADR-014 §1), one whose kind
   differs from its position's (an `IntegerRange` over a collection, say),
   and two that share a key. The derivation refuses, naming the limit, when
   the operand types hold more positions than the check stage's node limit
   (`s3.nodes`) allows: `ParityBoundRefusal::PositionLimit`,
   `stage_limit_exceeded`.
5. **Harness bounds.** Refuse, naming the key, a harness bound whose key is
   not a derived position, two harness bounds that share a key, and a
   harness bound whose kind differs from its position's kind. For an enum
   position the kind is `Variants`. `FiniteBound` gains this kind
   (`FiniteBoundKind::Variants`): a non-empty set of the enum's variant
   member identifiers, ascending by UTF-8 bytes, with no duplicates. A
   `Variants` bound naming a variant the declared enum does not admit
   refuses, naming the key. `Variants` is a harness-bound kind only, not a
   B-4 substitution kind: an enum always has an authored domain, so step 4
   refuses a request `DeclaredDomain` over an enum position.
6. **Identity tie.** The composite parity claim's obligation identity is
   ADR-013 O-09's parity preimage, with `operator` the composite `Equal` or
   `NotEqual`, the subject `node` and `occurrence`, and one argument per
   operand in operand order:
   - a parameter operand is `graph_child` with the parameter's node id (which
     must be the node's child at that position, else
     `ScalarIdentity::OperandChild`), and its domain is `bounds` over the
     harness bounds drawn on that parameter's positions, including the
     cardinality or depth drawn for a collection or recursive position that
     has no declared maximum;
   - a composite literal operand is `graph_child` with the literal's own node
     id and the empty `bounds` domain, `{"tag":"bounds","entries":[]}`: the
     content-addressed node id already binds its value, and it names no free
     position;
   - an integer literal operand is `inline_literal` with the singleton
     `range`, as FR-357;
   - an inline literal of any other type refuses
     `ScalarIdentity::NotInlineLiteral`.

   Recompute the digest from the recompiled package and the claim. If it
   differs from the request's `obligation_identity`, refuse
   `ScalarIdentity::Obligation`, naming both digests. A preimage the encoder
   refuses refuses `ScalarIdentity::Encoding` and never yields an identity.
   CG keeps its own abstractions, size budget, pair count and unexercised
   behaviours outside this preimage, in its own identity record (CG
   FR-015-AC-76).

### Falsified item: `replay_composite_parity`

After the common steps, the entry SHALL settle by the first matching row.
A refusal from the common steps rejects the request as not being this
claim, so it comes before every row, `Disagreed` included.

| Row | Condition | `CompositeParityResult` | `TerminalValue` |
| --- | --- | --- | --- |
| F-1 | `refinement` is `Disagreed` | `Disagreed` | `Failed`, whatever the operands' admission, `native` or the comparison would give |
| F-2 | `native` is `Incomplete` or `ExecutionFault` | `GeneratedFault { native }`, keeping the native outcome and its `NativeCause` | `Failed`, a generated-artifact fault; checked before admission, so a CG artifact fault is never hidden |
| F-3 | an operand fails admission | `RefusedInput(OperandRefusal)`, naming its index (0 or 1) and its code | `Inconclusive(ReplayRefused(code))` |
| F-4 | admitting an operand reaches one of the request's accounting limits | `Incomplete { stage: IncompleteStage::Admission(Incomplete) }`, naming the counter, its configured value and the count reached | `Incomplete(ResourceExhausted)` |
| F-5 | QSL's exact evaluation reaches a limit before an outcome | `Incomplete { stage: IncompleteStage::ExactEvaluation(Incomplete) }`, naming QSL's counter, its configured value and the count reached | `Incomplete(ResourceExhausted)` |
| F-6 | `refinement` is `CeilingReached` | `Incomplete { stage: IncompleteStage::RefinementCeiling }` | `Incomplete(ResourceExhausted)` |
| F-7 | otherwise: compare the exact outcome with `shadow` | `Diverged { exact, shadow, charges }` if the verdict or the pair count differs; `Agrees { agreement, charges }` if both agree | `Failed` for `Diverged`; `Inconclusive(ScalarAgrees)` for `Agrees` |

- **Native fault (row F-2).** This is the rule FR-357's operator arm
  applies, so one rule covers both parity arms, and QSL leaves no
  precedence to CG. No admission and no exact evaluation run, so a native
  fault is never hidden by an operand that fails admission or by an
  accounting limit.
- **Admission (rows F-3 and F-4).** The entry SHALL admit each operand
  against its domain through FR-098's conversion and S6a admission, under the
  request's accounting limits as FR-098 states. A parameter operand is
  admitted against its declared type. A literal operand is admitted when it
  equals the package's literal under the kernel's equality relation. A
  failing operand's code is `invalid_runtime_input` (row F-3), and nothing is
  evaluated. An accounting limit reached while admitting is row F-4, whose
  stage is `Admission`: no exact evaluation ran, so it never names
  `ExactEvaluation`.
- **Exact evaluation (rows F-5 and F-7).** The entry SHALL evaluate the
  kernel's equality relation (`plan_equality`) on the admitted operands
  under `limits`. The exact outcome is the verdict under `operator`
  (`NotEqual` negates `Equal`, with the same pair count) and the
  occurrence-pair count, counted with no early exit after an unequal pair
  (QSpec FR-149). The exact evaluation runs before row F-6 is checked, so
  a QSL limit is reported as QSL's stage even when the refinement also
  reached its ceiling.
- **Limit stages stay distinct.** A native stop (row F-2) is
  `GeneratedFault { native }`, settling `Failed`, with the driver's
  `NativeCause`. An accounting limit of the request (row F-4) is
  `IncompleteStage::Admission`. A QSL exact-evaluation limit (row F-5) is
  `IncompleteStage::ExactEvaluation`, carrying QSL's `Incomplete` record. A
  refinement ceiling (row F-6) is `IncompleteStage::RefinementCeiling`. Rows
  F-4, F-5 and F-6 share the terminal value `Incomplete(ResourceExhausted)`.
  The report keeps the stage in its result, so a reader tells them apart
  without the terminal value.
- **Compare (row F-7).** `Diverged` holds both outcomes and the evaluation's
  charges. It is a CG defect, which settles the plain `Failed` and never
  `Refuted`; CG maps it to its own `CgDefect` (CG FR-029). `Agrees` means
  the counterexample does not reproduce.
- `ScalarAgrees` SHALL widen to parity claims, keeping its name and its
  wire spelling, consistently with FR-357. FR-357's `ScalarClaim` gains a
  `CompositeEquality` arm holding the obligation identity, `node`,
  `operator`, both admitted operands, `content_identity` and `limits`.
  FR-357's `ScalarOutcome` gains an `Equality(EqualityOutcome)` arm for the
  agreed outcome. The cause holds no predicate verdict, and the proof
  envelope's encoded-size bound (FR-069-AC-4) counts it.
- A `native` of `Completed` or `Refused` SHALL be carried in the report as
  bound evidence, and SHALL NOT change the settlement.
- No row settles `Refuted`.
- The report SHALL carry the full claim (`report.claim()`) on every outcome,
  a refusal included.

### Verified item: `settle_verified_shadow`

After the common steps, a derived position is covered exactly when a
harness bound has its key and reaches its declared bound:

- `IntegerRange`: the harness lower bound is at or below the declared lower
  bound, and the harness upper bound is at or above the declared upper
  bound;
- `Cardinality` and `Depth`: the harness maximum is at or above the declared
  maximum (the harness reaches `Sequence<T>[0, 3]` up to 3);
- `Variants`: the harness's drawn variants include every variant the
  declared enum admits. A partial set does not cover.

A derived position with no harness bound is not covered. A position whose
declared bound is unbounded (no authored bound and no request
`DeclaredDomain`) is never covered, since any finite harness bound is
tighter (ADR-014 §8: a bounded result is evidence only for the bounds it
ran). The empty harness list covers exactly when the derived position set
is empty, as for a claim between two literals or over operands with no
bounded position.

The entry SHALL settle by the first matching row, and SHALL read no native
observation:

| Row | Condition | `TerminalValue` | Category |
| --- | --- | --- | --- |
| V-1 | `refinement` is `Disagreed` | `Failed`, the plain variant with no cause of its own. The shadow disagrees with the production code, a CG defect; CG maps `Failed` to its own `CgDefect`. Never `Refuted` | internal failure |
| V-2 | `refinement` is `CeilingReached` | `Incomplete(ResourceExhausted)`; the result records the refinement-ceiling stage | incomplete |
| V-3 | `success_checks` is 0 | `Proved { basis: Checks { success_checks: 0 }, certification: Certified }`, the vacuous proof | inconclusive, cause `kani_vacuous_proof` |
| V-4 | `refinement` is `Exhausted`, and every derived position covered | `Proved { basis: Checks { success_checks }, certification: Certified }` | success |
| V-5 | otherwise: `NotExhausted`, or `Exhausted` with a derived position not covered | `Tested`: success, never promoted to proof | success |

- `Tested` SHALL map to `success` and SHALL never be promoted to `Proved`
  (FR-069).
- The report SHALL carry the full claim (`report.claim()`) on every outcome,
  a refusal included.

## Acceptance Criteria

The unit for every criterion declares
`record Q { x: Int[0, 9]; s: Sequence<Int[0, 9]>[0, 3]; }`,
`f(a: Q, b: Q): Boolean { a == b }` (node `E`, operator `Equal`),
`g(a: Q): Boolean { a != Q { x: 1, s: [] } }` (node `N`, operator
`NotEqual`, a literal right operand),
`record List { head: Int[0, 9]; tail?: List; }`,
`h(a: List, b: List): Boolean { a == b }` and
`k(a: Sequence<Int[0, 9]>, b: Sequence<Int[0, 9]>): Boolean { a == b }`.
`B` is the covering harness bounds of `f`: `[0, 9]` on each `x`,
cardinality 3 on each `s`, and `[0, 9]` on each `s` element. Unless a
criterion says otherwise, each request's `obligation_identity` is the one
step 6 recomputes over that call's own harness bounds.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-358-AC-1 | Falsified (row F-7), at `E`, with `refinement: Exhausted`, operands `Q { x: 1, s: [2] }` and `Q { x: 1, s: [3] }`, and `native` `Completed` or `Refused`: the exact outcome is `{ equal: false, pair_count }`, counting every pair. A retained shadow `{ equal: true }` with that count, and one with the right verdict but a count one lower, each settle `Diverged`, which is `Failed`, category internal failure. A shadow equal to the exact outcome settles `Agrees`, which is `Inconclusive(ScalarAgrees)` holding a `CompositeEquality` claim and an `Equality` outcome, category inconclusive. At `N`, the exact verdict is the negation of `Equal`'s with the same pair count, and an agreeing shadow settles `Agrees`. No case settles `Refuted`. | Test (TC-907) |
| FR-358-AC-2 | Falsified, rows F-2 to F-6, each with an agreeing shadow: `native` `Incomplete` and `ExecutionFault` each settle `GeneratedFault` holding that observation and its `NativeCause`, which is `Failed`, with no admission and no exact evaluation, even with an operand that fails admission, with request accounting limits too small to admit the operands, under exact-evaluation limits too small, and with `refinement: CeilingReached` (F-2 before every later row). A right operand at `N` other than the package's literal, and a left operand at `E` with `x: 12`, each settle `RefusedInput` (indexes 1 and 0), which is `Inconclusive(ReplayRefused(invalid_runtime_input))`, with nothing evaluated, even with `refinement: CeilingReached` (F-3). Request accounting limits too small to admit the operands settle `Incomplete { stage: Admission }` naming the counter, never `ExactEvaluation`, even with `refinement: CeilingReached` (F-4). Limits too small for the exact evaluation settle `Incomplete { stage: ExactEvaluation }` naming QSL's counter, even with `refinement: CeilingReached` (F-5 before F-6). `refinement: CeilingReached` with admitted operands, a `Completed` native and enough limits settles `Incomplete { stage: RefinementCeiling }`. All three stages are `Incomplete(ResourceExhausted)`, and the report keeps them distinct. | Test (TC-907) |
| FR-358-AC-3 | Both entries refuse, with the obligation identity carried, before any evaluation or settlement row: a request whose source was edited so its recompiled `package_id` differs (`PackageIdMismatch`, `content-mismatch`); an undeclared `selected_function` (`UnknownFunction`, `missing-name`); a `node` the package does not hold, a `node` outside `f`'s body, and `operator` `NotEqual` at `E` (each `ScalarIdentity`, `content-mismatch`, with its own cause); an `occurrence` the node does not have in `f`'s body, and one of another function (each `ScalarIdentity::Occurrence`); and an `obligation_identity` computed over `[0, 9]` on the first `x` while the supplied harness bound is `[0, 8]` (`ScalarIdentity::Obligation`, naming both digests). At `d(a: Q, b: Q): Boolean { (a = b) and (a = b) }`, one node with two occurrences, each occurrence settles under its own identity, and the other occurrence's identity refuses `ScalarIdentity::Obligation`. The identity equals the SHA-256 of the canonical O-09 text written out independently, for `f` over `B` and for `g` (a literal right operand); a composite literal operand is `graph_child` over its own node id with the empty `bounds` domain; an operand of a recursive type is identified by the depth the harness drew, so two drawn depths are two obligations. Every outcome of both entries carries the full claim (`report.claim()`), and a changed observation member makes it differ from the claim sent. A raised `ReplayLimits.input_bytes` admits an S1 limit that the default refuses (`LimitAboveReader`). Neither entry calls `qsl_replay::replay` or selects a `Boolean` function to call. | Test (TC-907) |
| FR-358-AC-4 | Verified, at `E`, with `success_checks` 4, `refinement: Exhausted` and `B`: `Proved { basis: Checks { success_checks: 4 }, certification: Certified }`, category success (row V-4). The same holds with harness `[-1, 10]` on the first `x`. At `N`, the literal operand derives no position, and `B`'s left half covers. At `k`, with request `DeclaredDomain`s substituting cardinality 5 for each `s`, harness cardinality 5 and `[0, 9]` on each element give `Proved`. With `enum Color { Red, Green, Blue }` and `c(a: Color, b: Color): Boolean { a == b }`, harness bounds `Variants { Blue, Green, Red }` on each operand give `Proved { basis: Checks { success_checks: 4 }, certification: Certified }`. | Test (TC-907) |
| FR-358-AC-5 | Verified, with `success_checks` 4, `Tested`, category success and never `Proved` (row V-5), for each of: `NotExhausted` (CG's `_sampled` and `_not_run`) with `B`; `Exhausted` at `c` with `Variants { Green, Red }` on `a` and every variant on `b` (a partial variant set); `Exhausted` with `B` over `t(a: R, b: R): Boolean { a == b }`, where `record R { x: Int[0, 9]; label: Text[0, 4]; }` and `B` covers each `x` (a text leaf's whole declared domain cannot be shown covered); `Exhausted` with cardinality 2 on one `s`; `[0, 8]` on one `x`; no harness bound on one `x`; an empty harness list at `E`; harness depth 3 at `h`, whose recursive position has no `DeclaredDomain`; and harness cardinality 5 at `k` with no `DeclaredDomain`. | Test (TC-907) |
| FR-358-AC-6 | Verified: `refinement: Disagreed` (CG's `refinement_failed`) settles `Failed`, category internal failure, never `Refuted`, whatever `success_checks` and the bounds (row V-1). `CeilingReached` (CG's `_inconclusive`) settles `Incomplete(ResourceExhausted)` (row V-2), with 4 checks and with 0. With `success_checks` 0, `Exhausted` and `B`, it settles `Proved { basis: Checks { success_checks: 0 }, certification: Certified }`, category inconclusive, cause `kani_vacuous_proof` (row V-3). | Test (TC-907) |
| FR-358-AC-7 | Verified, each of these refuses with no settlement row, including with `success_checks` 0, so the refusal takes precedence over row V-3: a harness bound keyed at a path `Q` does not have (refused input, naming the key); two harness bounds on one key; an `IntegerRange` harness bound keyed at an `s`; and a request `DeclaredDomain` keyed at an `x`, which has an authored bound. At `c`, a `Variants` bound naming `Purple`, and a request `DeclaredDomain` over an enum position, each refuse naming the key. | Test (TC-907) |
| FR-358-AC-8 | Falsified, row F-1: `refinement: Disagreed` settles `Disagreed`, which is `Failed`, never `Refuted`, for each of: a diverging shadow; an agreeing shadow; a left operand with `x: 12` (overriding the admission refusal); `native` `ExecutionFault` (overriding the native fault); and limits too small for the exact evaluation. A common-step refusal (an edited source) still refuses with `refinement: Disagreed`. | Test (TC-907) |

## Dependencies

- [FR-357](FR-357-replay-scalar-parity-claims.md): the operator-parity shape
  this claim follows, its identity refusals, and the `ScalarAgrees` cause it
  widens.
- [FR-069](FR-069-implement-typed-proof-result-envelope.md): `TerminalValue`,
  its categories and its replay-refusal row; ADR-013 C-09 and O-16.
- [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md): the
  operand `WitnessValue` forms.
- [FR-071](FR-071-implement-typed-replay-request.md) and
  [FR-098](FR-098-execute-a-replay-request.md): the request, its declared
  domains, the recompile and its identity checks, and operand conversion and
  admission.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): an input outside its
  declared domain is refused input.
- QSpec FR-149 (composite equality and its pair count) and FR-322 (node
  identity).
- ADR-013 O-09 and §2; ADR-014 §1, B-4, §2, §4 and §8; ADR-021 TX-2 and
  TX-3.
- CG FR-015 (the harness and its own identity record), CG FR-028-AC-17
  (strengths), AC-24 (the refinement ceiling), and AC-2 and AC-3 (the backend ceiling), CG FR-029-AC-17 and AC-24 (the interim rows
  this replaces, and CG's precedence), and CG FR-033 (the claim CG builds).

## Status

Implemented (QSL-640): the identity tie (step 6) recomputes ADR-013 O-09's
parity preimage through `parity_obligation` and runs the operand membership
checks. A composite literal operand and an operand with no declared maximum
are defined in O-09's parity preimage text, and each has a test.

The `K<T>` cases of
FR-358-AC-4 and AC-5 are tested on the derivation and coverage seam, over
each of `Sequence`, `Set`, `Bag` and `OrderedSet`, not through a source
function, because the grammar takes only `K<T>[min, max]`.
