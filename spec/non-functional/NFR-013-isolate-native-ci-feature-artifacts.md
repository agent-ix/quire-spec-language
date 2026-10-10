---
id: NFR-013
title: "Isolate native CI feature artifacts"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: depends_on
---
# NFR-013: Isolate native CI feature artifacts

## Statement

When a native CI feature lane is invoked, the repository shall route every
Cargo command in that lane to its selected artifact directory under the
caller-path and command-preservation contract below.

## Scope

The existing Makefile targets `ci-default-features` and `ci-all-features`,
including their ordered reachability from `ci`. With no lane override, a
nonempty `CARGO_TARGET_DIR` supplies the owned parent root; an unset or empty
root selects `target`. The children are `ci-default-features` and
`ci-all-features` respectively. Relative paths retain their interpretation
from Make's working directory. An explicit GNU Make command-line assignment
of `CI_DEFAULT_TARGET_DIR` or `CI_ALL_TARGET_DIR` selects that lane's complete
directory instead of appending a child.

Environment and GNU Make command-line `CARGO_TARGET_DIR` values supply the
original raw caller path bytes. One dollar remains one dollar; `$$` remains
two dollars, including before braces or parentheses. The root is captured
without evaluating embedded Make expressions or environment references.

Explicit GNU Make command-line `CI_DEFAULT_TARGET_DIR` and
`CI_ALL_TARGET_DIR` overrides retain recursive Make assignment decoding:
each desired literal dollar is encoded as `$$`, including before braces or
parentheses. The decoded lane path remains data at the shell/process boundary.
A caller launching Make through a shell also owns quoting its invocation;
the Rust fixture passes assignments directly through `Command::args`. Quoted
shell-variable transport does not execute path contents or expand their
embedded environment references.

The existing lane commands, in order, are:

| Lane | Position | Cargo arguments |
| --- | --- | --- |
| Default | 1 | `fmt --all -- --check` |
| Default | 2 | `clippy --locked --workspace --all-targets -- -D warnings` |
| Default | 3 | `test --locked --workspace` |
| Default | 4 | `clippy --locked -p qsl-semantics --all-targets -- -D warnings` |
| Default | 5 | `test --locked -p qsl-semantics` |
| Default | 6 | `clippy --locked -p qsl-cst --all-targets -- -D warnings` |
| Default | 7 | `test --locked -p qsl-cst` |
| All | 1 | `clippy --locked --workspace --all-targets --all-features -- -D warnings` |
| All | 2 | `test --locked --workspace --all-features` |

The feature-lane routing change leaves non-feature targets on their existing
artifact and feature policy. In particular, `ci-clean-build` retains its
separate `clean` child of the caller's `CARGO_TARGET_DIR` (falling back to
`target` for an unset or empty root), no-default-feature build, library-only
`handoff-writer` check and no-default-feature parse invocation.
The parse invocation and core xtask tools retain the caller root. No Cargo
flag, CI check or aggregate dependency order is changed by this routing policy.

## Rationale

QSL-132 requests separation of warm default/all-feature artifacts while
preserving the existing CI contract. Literal-path transport prevents a caller
root or lane override from selecting a different directory through shell
interpretation. This records that requested routing contract without adding a
product feature or changing the hosted-workflow policy.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Violations of the lane routing and command-preservation criteria | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-013-AC-1 | Without explicit lane overrides, every command selects its lane's distinct child of the owned parent root, including repeated default-to-all and all-to-default switches using the same root. | Test |
| NFR-013-AC-2 | Every feature command receives the selected caller-root child or explicit lane override as literal path data under the raw-root and lane-override assignment contracts above, including spaces, double quotes, backticks and dollar forms, without executing path contents. | Test |
| NFR-013-AC-3 | Feature routing preserves the nine Cargo argument vectors and their order above, default versus all-feature selection, default-before-all reachability from `ci`, and the existing non-feature and clean artifact/feature policy. | Test |

## Verification

[TC-915](../test-cases/TC-915-native-ci-feature-routing.md) executes the real
Makefile through Rust-owned process fixtures, compares actual environment and
complete argv against independent expectations, and rejects safe mutations of
the fixture copy. Helper cleanup after subprocess completion is a fixture
hygiene check in that TC, not an additional product or workflow requirement.
The recorder establishes routing only; real Cargo/compiler and feature-sensitive
CLI execution establish compilation and runtime behavior separately.

The full QSL-132 acceptance remains unchanged: "Running `make ci` twice in
succession from a warm target dir produces the same result both times, and no
lane can execute an artifact built under different features." Focused routing
tests, mutation controls and CLI extraction executions do not replace those
two unchanged-source warm aggregate runs.

## Dependencies

- [NFR-005](NFR-005-rust-verification-paths.md) governs Rust verification and
  canonical imported TC/AC trace attributes.
