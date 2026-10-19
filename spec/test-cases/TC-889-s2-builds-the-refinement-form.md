---
id: TC-889
title: "S2 builds the refinement form with every row in source order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: verifies
---
# TC-889: S2 builds the refinement form with every row in source order

## Description

Verify that S2 builds one `RefinementForm` per `refinement` declaration with
every row as written, open arguments and population-valued forms included,
and resolves no name.

Scope: FR-310-AC-1 to FR-310-AC-4.

## Test Procedure

Fixtures: ADR-020 §8's `CasRefinesCounter`; FR-139's `RingIsQueue`;
FR-147's `CasTwice`.

1. Build the S2 form of `CasRefinesCounter`.
2. Build the forms of `RingIsQueue` and `CasTwice`, and a refinement with
   the step row `step Impl::P::a::S -> stutter;`.
3. Build a declaration with history rows whose initial values are `-3`,
   `true`, `none`, `"a"`, `rational(1, 2)` and `Impl::Mode::idle`, and a
   fairness row `assume fair strong Impl::Counter::retryA;`.
4. Build the forms of a declaration whose abstract alias names no model and
   of one with a duplicate `population` row, then run S3 on them.

Tag the tests `#[trace("TC-889", "<AC id>")]`.

## Expected Results

- Step 1: abstract side `Model(Spec)`, concrete `Impl`, the rows in source
  order with spans; the `ensure` granularity `each`.
- Step 2: `seq` and `only` expression forms; `Apply` with `[self.ring,
  Open]`; `Protocol(Spec, Twice)`, a `Named` source of kind `fork` with a
  `Node` target for the protocol step row; `step Impl::P::a::S -> stutter;`
  builds a `Named` source with kind `None` and name `Impl::P::a::S`, and a
  `memory` source builds `Memory`.
- Step 3: the six history literals build as `Integer`, `Boolean`, `None`,
  `Text`, `Exact` and `EnumMember` forms, none as an expression form; the
  fairness row builds kind `Strong`, granularity absent.
- Step 4: both forms build; S3 refuses each with FR-135-AC-2's code.
