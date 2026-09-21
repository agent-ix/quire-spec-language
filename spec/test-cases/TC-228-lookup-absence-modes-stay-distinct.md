---
id: TC-228
title: "lookup returns the declared absence mode for a genuinely unmatched key"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: verifies
---
# TC-228: lookup returns the declared absence mode for a genuinely unmatched key

## Description

Verify that a `lookup<T>` query whose key genuinely matches no member of an
admitted binding returns the declared absence mode, a genuine completed
result. Scope: FR-084-AC-3.

An earlier version of this test case also tried to construct "the same
binding with subtype closure not established for `T`" as a second,
distinguishing case. That state is not constructible: `population.rs`'s
`lookup` performs no subtype-closure check of its own — the model
selection's generalization-graph closure is decided once, at admission
(`admit_binding`'s `subtype_closure: GeneralizationClosure` parameter), and
an unclosed graph yields the distinct unknown-closure outcome there,
carrying no binding at all (FR-084-AC-1, TC-226). There is consequently no
admitted `PopulationBinding` a `lookup` call could ever run against whose
closure is unresolved, so no step of this test case can reach that state.
The admission-level scenario now lives in TC-226, backing FR-084-AC-1
instead.

Catches an implementation that reports a `TypeMismatch` or other refusal as
the declared absence mode, or vice versa, on a genuinely unmatched key.

## Test Procedure

1. Declare a population with extent `closed`, member type `Item`, with a
   closed generalization graph, and admit a binding with two `Item`
   members, neither matching key `r`.
2. Query `lookup<Item>(p, r) absent Undefined` against this binding.

## Expected Results

Step 2 returns `Undefined` (the declared absence mode), a genuine, completed
result — not a refusal and not an incomplete outcome. A mutant that returns
a refusal or incomplete outcome for a genuinely unmatched key fails this
assertion.
