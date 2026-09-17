---
id: TC-136
title: "Admit exact composed state views and typed outcomes"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: verifies }
---
# TC-136: Admit exact composed state views and typed outcomes

## Description

Verify the public admitted-artifact evaluation boundary, recursive value types,
authority checks, graph storage keys and closed outcome vocabulary.

## Test Procedure

Evaluate a selected value from an independently read compiled artifact using
exact binder and population inputs containing every value kind. Mutate the
declaration/value owner, binding requirement, nominal type/unit, model,
authority, observation anchor, object full key and available/unavailable state
one axis at a time. Include duplicate same-anchor objects, the same logical
object at pre/post anchors, a complete population with a missing target and an
identical incomplete-closure view. Attempt to reach evaluation from raw bytes
and a freely constructed wire package at compile time.

Build the positive inputs only by translating declaration keys from an admitted
domain package and selections from the F observation contract revision
`4d6230eb8aa9766ff3017360962f2d6368d74cb3`. Substitute the domain package digest,
the observation revision and one owned identity independently; do not replace
either authority with a local mock that invents it.

## Expected Results

Only the admitted package and exact inputs complete. Each structural, type,
identity or authority mutation returns its typed refusal. Missing known data is
incomplete only when reached; it never constructs a semantic placeholder.
Duplicate full keys refuse, pre/post storage remains distinct and permitted
identity comparison does not retag either observation. The public report exposes
one typed outcome plus limits/usage without message parsing.
