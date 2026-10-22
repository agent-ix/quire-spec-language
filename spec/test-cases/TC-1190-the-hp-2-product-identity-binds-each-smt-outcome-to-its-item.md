---
id: TC-1190
title: "The HP-2 product identity binds each SMT outcome to its item and instance"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: verifies
---
# TC-1190: The HP-2 product identity binds each SMT outcome to its item and instance

## Description

Verify that `encode_hyper_query` returns a QSL-minted, in-process
`HyperProductIdentity` made of the clause occurrence, the subject
identities and the instance, that a caller depending only on `qsl-replay`
can name it from its `replay` module but cannot build, serialize or
deserialize it, that its refusals come in their stated order, that
`SubjectIdentity` is `qsl-replay`'s order-independent type, and that
`settle_hyper_smt` settles an HP-2 SMT outcome only for the item and
instance its identity names, checking any witness itself.

Scope: FR-315-AC-12 to FR-315-AC-14, FR-182-AC-11, FR-182-AC-12, FR-336-AC-6.

## Test Procedure

1. Over ADR-023 §8's secure vault with the `vaults` universe `{v, w}`,
   encode `InductionBase{depth: 1}`, `InductionStep{depth: 1}` and
   `InductionStep{depth: 2}` for `NonInterference` at instance `v`, and call
   `HyperProductIdentity::of` over the same request. Read `clause()`,
   `subjects()` and `instance()`, and compare them with the `claim`
   occurrence of the clause's `hyperproperty` node in the package's
   checked-package v2 emission and with `SubjectIdentity::of` over the
   subject.
2. Repeat step 1's base query at instance `w`; for the clause with
   `fair { weak V::Vault::step }` on `a`; for a subject that differs only in
   a proof bound; and with a meter of a different budget. Compare each
   identity and script with step 1's.
3. Call `HyperProductIdentity::of` and `encode_hyper_query` for
   `NonInterference` with an empty `instance`; with the key `x` over the
   universe `{v}`; for `SavesPower` with its `N` subject over another
   checked package; for `NonInterference` over a subject whose checked
   package has no emission; for the HP-3 clause of FR-315-AC-10; and for
   FR-176-AC-3's liveness clause with an empty `instance`. Call
   `HyperProductIdentity::of` for `NonInterference` with a `match` conjunct
   whose evaluation exhausts the request's `meter`, and read the meter's
   consumed counters.
4. Build a test crate whose only QSL dependency is `qsl-replay`, whose
   `replay` module names `HyperProductIdentity`, `HyperQuery`,
   `HyperQueryRequest`, `SubjectIdentity` and `HyperSmtOutcome` and calls
   `encode_hyper_query`, `HyperProductIdentity::of` and `settle_hyper_smt`.
   Add three compile-fail cases: a struct literal of
   `HyperProductIdentity`, serializing one, and deserializing one.
5. Call `qsl_replay::SubjectIdentity::of` over FR-336-AC-4's subject and
   compare it with the identity `qsl-analyze`'s request path writes into
   the item; repeat with its initial states, its universe's keys and its
   proof bounds each in reverse order and compare identities and hashes;
   call it over a subject whose checked package has no emission.
6. With `settle_hyper_smt` for the `NonInterference` item at instance `v`,
   settle: an `Inductive{depth: 1}` outcome carrying the item's identity,
   with FR-183-AC-8's matched witness, then with its unmatched witness; an
   `InductionNotClosed{depth: 1}` outcome carrying the item's identity;
   outcomes carrying the identity for instance `w`, for the proof-bound
   subject, for the fair clause (both results) and for the fair clause at
   instance `w`. Then settle with the item's `instance` empty.
7. With `settle_hyper_smt` for the same item, settle outcomes carrying its
   identity: `BoundReached{depth: 2}`; the leaky vault's `Violated`
   `Lockstep` lasso (method `SmtUnrolling`), then the same lasso with one
   post-state digest altered, then the same lasso with its `instance`
   binding `w`; FR-315-AC-9's undefined-match base model as an `Undefined`
   counterexample (method `KInduction`); `Unknown{depth: 1, method:
   KInduction}`; `Cancelled{source: Requested, method:
   KInduction}`; and
   `EncodingRefused` with `match-conjunct`, `hyper-body` and `query-kind`.
   Then settle three solver stops at depth 1: one under the backend's own
   wall-clock limit (`TimedOut`), one under the request's `max_states`
   (`LimitStopped`), and an internal resource stop no request limit names
   (`Unknown`).
   Repeat each with the identity for instance `w`.

Tag the tests `#[trace("TC-1190", "FR-315-AC-n")]`,
`#[trace("TC-1190", "FR-182-AC-11")]`,
`#[trace("TC-1190", "FR-182-AC-12")]` and
`#[trace("TC-1190", "FR-336-AC-6")]`.

## Expected Results

- Step 1: three equal `product` values, each equal to the
  `HyperProductIdentity::of` result; `clause()` equals the `hyperproperty`
  node's `claim` occurrence, `subjects()` holds the one `SubjectIdentity`,
  and `instance()` is the one pair binding the parameter to `v`.
- Step 2: instance `w` differs only in `instance()`; the fair clause gives
  byte-equal scripts and differs only in `clause()`; the proof-bound
  subject differs only in `subjects()`; the other meter budget gives an
  equal identity.
- Step 3: from both calls, with no identity and no script:
  `instance-binding` at the parameter for the empty instance and for `x`;
  `subject-binding` at alias `N`; `product-identity` at the clause;
  `hyper-form`; `instance-binding`, not `hyper-body`, for the liveness
  clause. The `match`-conjunct request returns its identity with every
  consumed counter at zero.
- Step 4: the crate compiles; all three compile-fail cases fail to compile.
- Step 5: the two identities are equal; the reversed listings give an
  equal identity with an equal hash; the subject with no emission is
  refused `SubjectIdentityRefusal::Package`.
- Step 6: `proved`, `decisive-witness`, `Uncertified`, method `k-induction`
  with the matched witness; `inconclusive`, `VacuousMatch` with the
  unmatched one; `inconclusive`, `InductionNotClosed{depth: 1}`; then no
  record and `ProductMismatch` naming `instance`, `subjects`, `clause`
  (both results) and `clause` for the fair clause at `w`; and
  `ItemIdentity` carrying `instance-binding` for the empty instance.
- Step 7: `inconclusive`, `BoundReached{depth: 2}`, method `smt-unrolling`;
  `refuted`, `decisive-counterexample`, method `smt-unrolling`, from
  QSL's own replay; `inconclusive`, `ReplayRefused`; no record and
  `ProductMismatch` naming `instance` for the `w` lasso, never `refuted`;
  `refuted`, `UndefinedEvaluation`, method `k-induction`; `inconclusive`,
  `InductionNotClosed{depth: 1}`, method `k-induction`;
  `failed`, `Incomplete(Cancelled{Requested})`; `inconclusive`,
  `MatchUndetermined`; `unsupported`, `unsupported-requested-capability`;
  no record and `QueryKind`. Every repeat with the instance-`w` identity
  writes no record and refuses `ProductMismatch` naming `instance`.
  The three solver stops settle `failed`, `Incomplete(TimedOut)`; `failed`,
  `Incomplete(LimitReached)` naming `max_states`; and `inconclusive`,
  `InductionNotClosed{depth: 1}`. Neither of the first two settles
  `InductionNotClosed`.
