---
id: TC-1190
title: "The HP-2 product identity binds each SMT outcome to its item and instance"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: verifies
---
# TC-1190: The HP-2 product identity binds each SMT outcome to its item and instance

## Description

Verify that `encode_hyper_query` returns a QSL-minted
`HyperProductIdentity` made of the clause occurrence, the subject
projections and the instance, that a caller depending only on `qsl-replay`
can name it but not build it, that it refuses a bad instance or package,
and that FR-182 settles an HP-2 SMT outcome only for the item and instance
its identity names.

Scope: FR-315-AC-12 to FR-315-AC-14, FR-182-AC-11.

## Test Procedure

1. Over ADR-023 §8's secure vault with the `vaults` universe `{v, w}`,
   encode `InductionBase{depth: 1}`, `InductionStep{depth: 1}` and
   `InductionStep{depth: 2}` for `NonInterference` at instance `v`, and call
   `HyperProductIdentity::of` over the same request. Read `clause()`,
   `subjects()` and `instance()`, and compare them with the `claim`
   occurrence of the clause's `hyperproperty` node in the package's
   checked-package v2 emission and with FR-336's projection of the subject.
2. Repeat step 1's base query at instance `w`; for the clause with
   `fair { weak V::Vault::step }` on `a`; for a subject that differs only in
   a proof bound; and with a meter of a different budget. Compare each
   identity and script with step 1's.
3. Call `HyperProductIdentity::of` and `encode_hyper_query` for
   `NonInterference` with an empty `instance`, with the key `x` over the
   universe `{v}`, and for `SavesPower` with subjects carrying two
   different checked packages. Call `HyperProductIdentity::of` for
   `NonInterference` with a `match` conjunct whose evaluation exhausts the
   request's `meter`, and read the meter's consumed counters.
4. Build a test crate whose only QSL dependency is `qsl-replay` that names
   `HyperProductIdentity`, `HyperQuery`, `HyperQueryRequest` and
   `SubjectIdentity` and calls `encode_hyper_query` and
   `HyperProductIdentity::of`. Add two compile-fail cases to it: a struct
   literal of `HyperProductIdentity`, and deserializing one.
5. Settle step 1's `Inductive{depth: 1}` outcome, carrying the instance-`v`
   identity, with FR-183-AC-8's matched witness for `v` whose `Accepted`
   check comes with an equal identity, for the item at instance `v`, then
   for the item at instance `w`. Settle an `Inductive{depth: 1}` and an
   `InductionNotClosed{depth: 1}` outcome carrying the fair clause's
   identity for `NonInterference`. Settle the instance-`v` outcome with an
   `Accepted` check that comes with the fair clause's identity.

Tag the tests `#[trace("TC-1190", "FR-315-AC-n")]` and
`#[trace("TC-1190", "FR-182-AC-11")]`.

## Expected Results

- Step 1: three equal `product` values, each equal to the
  `HyperProductIdentity::of` result; `clause()` equals the `hyperproperty`
  node's `claim` occurrence, `subjects()` holds the one FR-336 projection,
  and `instance()` is the one pair binding the parameter to `v`.
- Step 2: instance `w` differs only in `instance()`; the fair clause gives
  byte-equal scripts and differs only in `clause()`; the proof-bound
  subject differs only in `subjects()`; the other meter budget gives an
  equal identity.
- Step 3: `instance-binding` at the parameter for the empty instance and
  for `x`, `product-identity` at the clause for `SavesPower`, from both
  calls, each with no identity and no script; the `match`-conjunct request
  returns its identity with every consumed counter at zero.
- Step 4: the crate compiles; both compile-fail cases fail to compile.
- Step 5: the instance-`v` settlement is `proved`, `decisive-witness`,
  `Uncertified`; the instance-`w` settlement writes no record and refuses
  `ProductMismatch` naming `instance`; both fair-clause outcomes refuse
  `ProductMismatch` naming `clause`; the outcome with the fair clause's
  check settles `inconclusive`, `VacuousMatch`.
