---
id: FR-257
title: "Build S2 forms and control anchors at any depth"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: depends_on
---
# FR-257: Build S2 forms and control anchors at any depth

## Description

S2 SHALL build a form for every unit S1 admits, whatever the depth of its
expressions, types and control clauses (ADR-030 D-4.2). S2 builds at most one
form node per CST node, so S1's node limit (`s1.nodes`) bounds it, and S2
takes no limit of its own.

## Behavior

1. **Arena forms.** S2 SHALL store `qsl_forms::Expression` as one vector of
   nodes per declaration body, with each child named by a typed index into
   that vector, so that cloning, comparing, formatting for debug and
   dropping a body each visit its nodes without native recursion. Debug
   formatting of the tree SHALL walk it over an explicit heap stack.
2. **Iterative build.** S2 SHALL map the CST to forms over an explicit heap
   stack, for expressions, type forms and every family's forms.
3. **Iterative control anchors.** S2's control-anchor walk SHALL run over an
   explicit heap stack of frames, each holding a control node, its child
   cursor and the scope length on entry. When a frame is left, S2 SHALL
   restore the scope to the length recorded on entry.
4. **No limit set.** S2 SHALL take no limit set. A body S1 admits SHALL build
   or be refused by its content, through the causes FR-091, FR-102 and
   FR-112 define.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-257-AC-1 | On a thread with a 512 KiB stack, every 100,000-deep input of FR-256-AC-1 that is a function body builds its forms under S1 limits raised to fit it, and its form tree is as deep as the body. The built unit clones, compares equal to its clone, formats for debug and drops on the same thread. | Test (TC-724) |
| FR-257-AC-2 | On a thread with a 512 KiB stack, a protocol clause holding a 100,000-deep `await … then` chain and one holding a 100,000-deep `repeat … exhausted` chain each build their scoped anchors under S1 limits raised to fit them, and every anchor resolves in the scope FR-112 gives it. | Test (TC-724) |
| FR-257-AC-3 | `qsl_forms::build_unit` takes the S1 output and no limit set, and an invariant whose body is 128 nested `not`s around `true` builds its form (FR-102). | Test (TC-724) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-4.2.
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  defines S2's `Value` production, [FR-102](FR-102-build-state-clause-forms.md)
  its state-clause forms and
  [FR-112](FR-112-build-protocol-scoped-anchor-forms.md) its scoped anchors.
- [FR-256](FR-256-parse-source-at-any-nesting-depth.md) bounds the S1 output
  S2 consumes.

## References

- QSpec FR-460, the ecosystem depth rule (Linear STD-143, which supersedes
  STD-125).
- Linear QSL-381.
