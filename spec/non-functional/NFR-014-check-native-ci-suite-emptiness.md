---
id: NFR-014
org: agent-ix
title: "Check native CI suite emptiness"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-002
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: depends_on
---
# NFR-014: Check native CI suite emptiness

## Statement

When a local native CI test lane completes, the repository shall accept its
suite coverage only under the selected-target, declared-emptiness and refusal
policy below.

## Scope

The four existing local `make ci` test invocations are:

| Lane | Original Cargo test arguments, in order |
| --- | --- |
| default-workspace | `--locked --workspace` |
| default-qsl-semantics | `--locked -p qsl-semantics` |
| default-qsl-cst | `--locked -p qsl-cst` |
| all-features-workspace | `--locked --workspace --all-features` |

A suite identity includes its selected workspace package and Cargo target
(including workspace-relative source), suite kind (test executable or library
doctests), and effective lane feature selection. Repeated target names in
different packages remain distinct. Executable paths and rustdoc names resolve
observations to selected producer identities; they do not independently grant
identity. Selected means eligible for this invocation, including package,
workspace, feature, required-feature and target-kind selection, rather than
every target mentioned by a dependency build.

The guard preserves the original test argument vector and lane target-directory
environment for discovery and execution. Expected suites follow the actual
selection: ordinary runs include selected doctest-enabled libraries; `--lib`
and `--tests` select executable tests without requiring doctest sections.
`--doc` either uses discovery compatible with doc-only Cargo execution or
returns an explicit usage refusal explaining the unsupported doc-only selection
before spawning test work. A blanket refusal of `--lib`, `--tests` and `--doc`
does not satisfy this policy. Unsupported selections and lane/feature mismatches
cannot silently fall back to another selection.

This is a developer gate policy. Hosted workflow changes, feature-directory
routing, spec validation, conformance checks and product semantics retain their
separate owners. The current empty-suite inventory is not established by this
requirement or by a historical gate log.

## Rationale

An `ok` summary with no passing tests cannot establish coverage. Legitimately
empty targets may remain selected when maintainers declare their exact identity
and an actual reason for emptiness in that target and feature scope. A generic
exception for all doctests, all proc-macro crates or all zero-pass output would
also hide lost coverage.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Accepted local lane runs violating the suite acceptance policy | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-014-AC-1 | Each of the four local test invocations reaches the guard with its original ordered Cargo arguments and selected target-directory environment; discovery and execution use the same effective package/workspace/features/target selection. Unknown lanes and unsupported or inconsistent selections return explicit usage refusal. Target flags follow the Scope rules, including no unexpected doctest requirement for `--lib` or `--tests`. | Test |
| NFR-014-AC-2 | Selected producer identities resolve unambiguously from genuine Cargo target information. Invalid JSON, missing or wrongly typed identity fields, conflicting duplicate identities/executables or normalized doctest-name collisions refuse; external dependency artifacts are excluded without discarding selected workspace targets. Relative and absolute executable paths resolve to the same selected target where applicable. | Test |
| NFR-014-AC-3 | Each selected suite has exactly one observed outcome. Unknown or malformed suite headers and duplicate suite outcomes refuse with the affected identity or unresolved header; the guard does not guess an owner from a repeated target name. | Test |
| NFR-014-AC-4 | Each observed suite has readable running and authoritative terminal summary counts. Missing, malformed, overflowing or inconsistent counts refuse. Running equals passed plus failed plus ignored plus measured; filtered-out counts are retained separately. Test output resembling an earlier summary does not replace the genuine terminal summary. | Test |
| NFR-014-AC-5 | A legitimate empty declaration names one selected target, kind and effective feature identity with a nonblank reason grounded in that target's actual source scope. Declarations are unique within a lane. Acceptance as declared empty requires running, passed, failed, ignored, measured and filtered-out counts all equal zero; the successful report identifies each such suite and its reason separately from suites with passing tests. | Test |
| NFR-014-AC-6 | A selected suite without a legitimate empty declaration refuses if it passes zero tests, including a formerly populated suite losing all tests and suites whose tests are entirely ignored, measured or filtered. A suite with passing tests and otherwise consistent counts may pass without an empty declaration. | Test |
| NFR-014-AC-7 | A declared-empty suite gaining any registered, passing, ignored, measured or filtered-out tests refuses as declaration drift, even when Cargo succeeds and passed remains zero. Removing or correcting the declaration requires maintainer assessment of the target's actual reason; the guard does not automatically allowlist new zero-pass suites. | Test |
| NFR-014-AC-8 | Every expected selected executable and doctest suite has an outcome, including nonallowlisted doctests. Missing selected suites, missing declared suites and an invocation observing no suite refuse. Targets excluded by the actual selection are not reported missing. | Test |
| NFR-014-AC-9 | An unsuccessful Cargo discovery or test child returns Cargo's original nonzero exit code when one is available; suite findings do not replace that failure with success or a generic guard code. A successful Cargo run violating the suite policy returns a nonzero guard refusal. | Test |
| NFR-014-AC-10 | After spawning owned Cargo work, a read or echo I/O failure terminates still-running owned child work and waits to reap the child before returning. The original I/O cause remains the reported failure even if cleanup also fails; failure never becomes a successful suite result. | Test |

## Verification

[TC-950](../test-cases/TC-950-native-ci-empty-suite-policy.md) exercises this
policy through Rust-owned assertions, genuine Cargo selection and the actual
Makefile, with independently specified expected identities, counts, argv and
environment. Focused transcript tests qualify only their asserted boundary;
they do not establish current whole-workspace inventory or gate acceptance.

Each test uses the imported shared trace macro with `TC-950` and only the
`NFR-014-AC-N` identities its assertions establish. Preserve the existing
NFR-005 trace convention. Retain independent negative controls and restored
mutant controls for lost coverage, ignored/filtered declaration drift, missing
doctests and selection transport. Measure current named empty suites and
source-grounded reasons in every final lane before claiming acceptance.

## Dependencies

- [NFR-002](NFR-002-reproduce-native-builds.md) owns native build and hosted
  execution policy.
- [NFR-005](NFR-005-rust-verification-paths.md) owns Rust verification and
  the shared canonical trace mechanism.
