---
id: FR-146
title: "Check total pure functions and recursion"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/FR-033
    type: references
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
---

# FR-146: Check total pure functions and recursion

## Description

When admitting a function declaration, the checker SHALL establish typed
purity, total definedness and a well-founded termination argument for every
reachable call cycle.

## Inputs

Function declarations, parameter/result types, call graph, effects declaration,
termination measure and selected type/model closure.

## Outputs

Checked callable identities or located type, effect, definedness or termination
refusals.

## Behavior

Functions may return any complete-V1 value type. Calls bind exact declaration
identity, arity and ordered arguments. Recursion uses a declared well-founded
measure proven to decrease on every cycle. Reflection, mutation, I/O and ambient
lookups are prohibited. Runtime depth/fuel exhaustion remains incomplete.

## Termination and totality contract

The checker constructs a declaration-identity call graph and checks each
strongly connected component. An acyclic component needs no termination
measure. Every recursive component declares one shared lexicographically
ordered tuple whose elements are nonnegative mathematical integers, finite
collection cardinalities or strict-subvalue depths. Each recursive edge must
prove that the substituted callee tuple is lexicographically smaller than the
caller's tuple; this rule also applies across mutual recursion.

Totality is path-sensitive: every pattern and conditional path must return the
declared result, all destructuring must be exhaustive and each partial
operation must have a proved precondition. Purity and typing are checked before
reachability, so an unreachable impure or ill-typed branch is still rejected.
Evaluation fuel is an execution resource rather than a semantic termination
argument: exhausting it returns `incomplete`, never a value or a proof of
nontermination.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-146-AC-1 | A pure recursive function with a proven decreasing finite measure is admitted and evaluates its declared result. | Test (TC-191) |
| FR-146-AC-2 | A nondecreasing or unproved call cycle refuses with the cycle and failed obligation. | Test (TC-191) |
| FR-146-AC-3 | I/O, mutation, reflection, ambient access or wrong argument/result typing refuses even in an unreachable branch. | Test (TC-191) |
| FR-146-AC-4 | A mutually recursive component is admitted only when every substituted cycle edge decreases the shared lexicographic measure. | Test (TC-191) |
| FR-146-AC-5 | Exhausting execution fuel for an admitted function returns incomplete without changing the function's totality verdict. | Test (TC-191) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.
