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
`quire-contract-ir`. When it extracts edges, the check SHALL give each package in
the named shared-leaf set, `quire-exact` and `quire-semantic-value`, no
ecosystem repository, wherever it is sourced from. These are ADR-011 FB-05's
shared `no_std` leaf crates: the kernel K (§6.1 "K is a leaf") and the
semantic-value leaf SV, whose dependencies are K, ADR-013's one RFC 8785
encoder `quire-canonical`, `serde` and `thiserror`. Neither depends on a QSL
layer or on an IR, RT or CG crate, so an edge into either closes no cycle. Every other package sourced from the QSL repository classifies as
QSL. This exemption is local to edge extraction; the shared `graph::classify`
classifies a QSL-sourced shared leaf as QSL.

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
| FR-059-AC-6 | Run against the real, current-head resolution of quire-contract-ir's manifest, the check reports no FB-05 edge from IR into QSL and no FB-11 cycle between QSL and IR, since neither IR manifest declares a QSL dependency (ADR-011 OBS-029). Resolving QSL's own workspace manifest through the same edge-resolution path the check uses for `--qsl` yields QSL's normal edge on the git-sourced `quire-contract-model`, classified as QSL → IR, the permitted direction. | Test (TC-156) |
| FR-059-AC-8 | A resolved RT dependency on the git-sourced `quire-exact` yields no edge and no finding; a resolved RT dependency on another crate sourced from the QSL repository, such as `qsl-eval`, yields an RT → QSL edge reported as an FB-05 violation. | Test (TC-156) |
| FR-059-AC-9 | A resolved RT dependency on the git-sourced `quire-semantic-value` yields no edge and no finding; resolved RT dependencies on `qsl-eval` and on `qsl-semantics`, from the same QSL source, each yield an RT → QSL edge reported as an FB-05 violation. | Test (TC-156) |

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

Remaining work (implementation A5, QSL-477): `tools/arch-lint` still has the
`duplicate-revisions` subcommand (`duplicate_revisions.rs`, its `main.rs`
entry and its `graph.rs` helpers) tracing the deleted FR-061 and TC-158, and
the Makefile's `arch-lint-duplicate-revisions` target runs it in `make ci`.
The subcommand, its traces and the target are deleted; `graph::classify`
stays for this check.

`arch-lint direction` also still compares each `--ir`/`--rt`/`--cg` clone's
head with its remote `main` and prints every resolved revision, under the
deleted FR-059-AC-7 (implementation A5, QSL-477). That comparison, its
`--offline` switch and its traces are deleted.
