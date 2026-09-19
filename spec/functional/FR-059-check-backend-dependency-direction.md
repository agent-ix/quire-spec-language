---
id: FR-059
title: "Check the backend dependency direction across QSL, Contract IR, Runtime and Codegen"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-059: Check the backend dependency direction across QSL, Contract IR, Runtime and Codegen

## Description

ADR-011 §3 fixes two forbidden bypasses over the four-repository ecosystem
(QSL, quire-contract-ir, quire-contract-runtime, quire-contract-codegen):

- **FB-05**: no backend (IR, RT or CG) depends on QSL, except CG's normal
  dependency on the QSL layer-6 `replay` facade.
- **FB-11**: no dependency edge, normal or dev, closes a cycle among the four
  repositories.

QSL SHALL provide a check, runnable over real `cargo metadata` output for the
four repositories, that reports every edge violating FB-05 and every cycle
violating FB-11, and passes only when neither set of findings is non-empty.

## Inputs

- Resolved dependency graphs (`cargo metadata --format-version=1`) for each of
  the four repositories' own manifests.
- Each resolved edge's source repository, target repository, dependency kind
  (normal or dev) and dependency package name.

## Outputs

- An FB-05 report: the list of edges into QSL that are not CG's normal
  dependency on QSL.
- An FB-11 report: the list of simple cycles over the combined normal+dev
  edge graph of the four repositories, each reported once regardless of which
  repository the cycle is discovered from.
- A pass/fail result: pass exactly when both reports are empty.

## Behavior

The check SHALL classify a resolved package as one of the four repositories by
package name and dependency source URL, treating `quire-contract-model` (the
crate name quire-contract-ir's workspace member publishes under) as
`quire-contract-ir`.

The check SHALL treat a `build`-kind dependency as a normal edge for FB-05/
FB-11 purposes, and a `dev`-kind dependency as a dev edge.

The check SHALL report CG's normal dependency on QSL as the one FB-05
exception. The check SHALL report a *dev* dependency from CG on QSL, since a
dev dependency is not the stated exception.

The check SHALL combine normal and dev edges into one directed graph for
FB-11, and SHALL report a cycle exactly once regardless of its rotation or
which repository it started the search from.

This check is crate-level only: it establishes that an edge into QSL exists at
all, not that the edge's caller code stays inside the layer-6 `replay` facade.
[FR-060](FR-060-check-qsl-api-surface-boundary.md) verifies the latter.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-059-AC-1 | A graph with only CG's normal dependency on QSL, and no cycle, reports both FB-05 and FB-11 clean. | Test (TC-156) |
| FR-059-AC-2 | Any normal or dev edge from IR or RT into QSL is reported as an FB-05 violation. | Test (TC-156) |
| FR-059-AC-3 | A *dev* edge from CG into QSL is reported as an FB-05 violation; the stated exception covers only CG's normal edge. | Test (TC-156) |
| FR-059-AC-4 | A 2-repository or longer cycle over normal and/or dev edges among the four repositories is reported exactly once as an FB-11 violation, however many repositories the search starts from. | Test (TC-156) |
| FR-059-AC-5 | An acyclic graph with edges only running toward QSL and CG reports no FB-11 violation. | Test (TC-156) |
| FR-059-AC-6 | Run against the real, current-head resolution of quire-contract-ir's manifest, the check reproduces ADR-011 OBS-029's real, currently observed FB-05 violation (IR depends on QSL) and the corresponding FB-11 cycles. | Test (TC-156) |

## Dependencies

- ADR-011 §3 FB-05, FB-11 and §7.1 T-12
  (`ix://agent-ix/quire-spec-language/ADR-011`).
- [FR-060](FR-060-check-qsl-api-surface-boundary.md) checks the FB-05
  exception's call-site boundary.

## Status

Specified and implemented under
[#215](https://github.com/agent-ix/quire-spec-language/issues/215) as the
`arch-lint direction` subcommand (`tools/arch-lint/graph.rs`,
`tools/arch-lint/metadata.rs`).
