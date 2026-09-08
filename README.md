# quire-spec-language

Private native syntax implementation for `ix:native`, edition `0-draft`, profile
`state-finite/0-draft`. It parses and formats source with located diagnostics.
Imports remain unresolved: successful parsing does not establish type correctness,
model binding, executable projection, or healthy/violating state evaluation.

CLI command and source identity/revision labels must be UTF-8; invalid encoding
returns usage exit 2. File operands remain OS paths. JSON paths are display text,
which may contain replacement characters; exact labels and the actual byte
digest carry the separate source correspondence. See the
[native diagnostic catalog](docs/native-error-codes.md) for stable outcomes.

Library callers can use `format::format_with_limit(unit, output_bytes)` to lower
the inclusive 1 MiB output-content ceiling. It counts every emitted byte,
including the final newline, and refuses before an append would exceed the
limit. Existing `format::format(unit)` retains the default ceiling. Allocator
capacity is not an exact content-byte accounting promise.

## Run and check

Rust 1.94.1 is pinned in rust-toolchain.toml. Cargo.lock pins dependencies. The
native CLI needs no Node or JVM. There are no optional feature requirements.
Use `--target-dir target` where a machine config points Cargo outside the checkout.

```sh
cargo run --locked --target-dir target -- parse test:parent fixture:1 tests/fixtures/parent.native
cargo run --locked --target-dir target -- format test:parent fixture:1 tests/fixtures/parent.native
cargo fmt --all -- --check
cargo clippy --locked --target-dir target --all-targets --no-default-features -- -D warnings
cargo test --locked --target-dir target --no-default-features
cargo build --locked --no-default-features --target-dir target/clean
cargo run --locked --target-dir target --bin fixture-audit -- self-test
cargo run --locked --target-dir target --bin fixture-audit -- model-bytes tests/fixtures
```

Run these checks locally while the repository stabilizes. Hosted CI exposes
only `workflow_dispatch` and has a ten-minute job timeout. Pushing commits or
opening a pull request does not request CI; hosted execution requires a separate
explicit dispatch.
Tests also need read access to the pinned private `agent-ix/ix-trace-rs` Git
dependency (or its exact cached revision). A future hosted runner needs that
access configured; local results do not qualify hosted credentials.

CLI arguments are `parse|format`, source identity, source revision, and file path.
`parse` emits JSON with status `parsed`; `format` writes source to stdout and
preserves comments/token spellings while normalizing whitespace to LF. Neither
command changes a file. Callers must assign the formatted document its appropriate
new revision before binding it into evidence. Diagnostics contain original byte,
line and Unicode scalar column positions. Exit 0 is success, 1 refusal, 2 usage/I/O
failure and 3 resource exhaustion (incomplete). These are local syntax outcomes,
not the portable verification result envelope.

The library returns immutable ParsedUnit syntax or a boxed Diagnostic. Source
identity/revision are opaque diagnostic labels; they do not implement the future
interchange source reference or numeric IR revision mapping. Source::read_verified
checks a selected byte digest; parse_source retains that verified immutable source. Source byte spans are half-open; located lines/columns are one-based.

Default ceilings are 1 MiB source, 100,000 tokens, 50,000 syntax nodes and 64
nested delimiters/parser frames. Callers can lower these ceilings. Formatting
also bounds output to 1 MiB and can return incomplete if expansion exceeds it.
Long flat expressions use an arena; grouping and all original source bytes are
retained. Balanced reserved forms refuse as unsupported; malformed tokens and
delimiters are syntax errors. No recovery manufactures a successful partial unit.

The [source correspondence API](docs/source-correspondence.md) checks exact
body-to-original segments with explicit layout transformations and returns
original document locations. Maps preserve all corresponding byte regions and
refuse changed bytes, foreign source bindings, malformed segments and budget
exhaustion. Wire decoding and existing-repository extraction remain separate.

## Design and status

The [requirements index](spec/spec.md) covers LC01–LC05 through discrete Quoin
catalog artifacts. [Authoring status](docs/spec-workflow.md) records the exact
skills, validation results and outstanding review work. These requirements are
drafts; the completed native readiness reviews and local results are recorded
in the authoring status. The owner adopted specification PR8 at
`e897f810a7356d4ce8fd19026221ebda7b65596f` for internal LC02 implementation.
[LC02's test matrix](spec/model-linking/tests.md) remains planned pending the
native linking/typechecking implementation. Accepted Contract IR ADR-0054 at
`690bde7f2dc58662cf9ff0595c2c0e3b17107c6f` closes #54 and removes the former
Filament-reader prerequisite: generic compilation uses the existing public
DeclarationEnvironment/check_expression and executable binder APIs. A concrete
archetype projection is specified only for a clause that needs its semantics.

See [architecture review](docs/architecture-review.md) for concrete fixes and
remaining linking/identity work. The current parser uses declarative Logos tokens
and a Pratt expression parser; JSON escapes use serde_json. Binding/lowering will
remain modules and use the existing model/IR authorities. No second domain model,
backend binder or Markdown expression parser is created here.

New implementation and newly authored implementation fixtures use
[AGPL-3.0-only](LICENSE). [Dependency inventory](docs/dependencies.md) preserves
third-party grants. [License decision](LICENSE-DECISION.md) records owner approval
and deferred standard-artifact terms. No public release is authorized.

Private tracking: [LC01](https://github.com/agent-ix/quire-spec-language/issues/2),
[compiler epic](https://github.com/agent-ix/quire-spec-language/issues/1).
The shared contracts and LC02–LC05 acceptance remain separate work.

## Model producer review fixture

[Model input](tests/fixtures/model-source/README.md) and
[production provenance](tests/fixtures/model-output/provenance.json) provide an
actual synthetic ConfigVersion model from the existing Filament TypeSpec
frontend: version numbers bounded to 0..1000, an optional parent field, and an
operation declaration. These are retained structural/provenance fixtures;
they supply no native formal-model binding or evaluated state result. Generated
datatypes do not establish formal bounds, reference or observation semantics.

The Rust `fixture-audit` binary replaces all four Python helpers under
[Agent A #58](https://github.com/agent-ix/quire-research/issues/58). Model-bytes
checks five recorded artifact digests and the exact producer revision. Earlier
fresh, selected-lock and stale-lock observations remain historical evidence.
The provenance JSON retains its original Python/Node command strings for that
reason. Fresh TypeSpec/Node qualification awaits separate owner approval:
`fixture-audit model-producer` exits 3 with `producer-language-unapproved`
without launching a process.

```sh
cargo run --locked --target-dir target --bin fixture-audit -- review /path/to/specification/proposals/state-core/fixtures
cargo run --locked --target-dir target --bin fixture-audit -- roles /path/to/specification/proposals/state-core/fixtures
cargo run --locked --target-dir target --bin fixture-audit -- rule-syntax /path/to/specification/proposals/state-core
QUIRE_STATE_CORE=/path/to/specification/proposals/state-core cargo test --locked --target-dir target --test fixture_audit -- --ignored
```

Review checks 23 artifact files, seven invocation cases and six independent
negative controls. The second command checks the optional private FS02 packet's exact
artifacts and source regions. It is producer bookkeeping, not an independent
semantic-reference matcher. These optional checks run separately from the native
runtime.

Rule-syntax wraps the FS03 rule examples in native source units and checks
their syntax with the existing parser library. It checks the selected rule/profile digests,
then expects 50 parsed expressions and one explicit unsupported refusal. It
does not interpret the abstract type environments or execute their 51 authored
typing/evaluation expectations. The owner adopted the separately digest-bound
FS03 refinements for internal implementation; that decision does not rewrite
the original profile bytes or establish executed typing/evaluation evidence.

The final command runs the three explicitly selected private-packet integration
tests, including independent corruptions. The normal Rust test suite needs no
private sibling repository. Audit intake bounds files to 8 MiB, aggregate reads
to 64 MiB, files/decoded values to 10,000, and JSON nesting to 64 values; it refuses
escaped duplicate keys, invalid fields, foreign paths and exhausted budgets.
Fixture trees must stay immutable during a run. See the
[audit error catalog](docs/audit-error-codes.md) and
[remediation evidence](docs/rust-verification-remediation.md).

New tests use the shared `ix_trace_rs::trace` macro with canonical attributes
such as `#[trace("TC-006", "FR-012-AC-6")]`. Quire's declared grammar binds the
IDs; the macro checks argument shape. `#[cfg(test)]` controls compilation.
Read [license decision](LICENSE-DECISION.md) before adding code or normative artifacts. Publication requires a fresh review; do not flip private history public.

## Implementation language — owner directive

Executable implementation, including production and qualification tools/tests, is Rust. New TypeScript choices require explicit owner approval; surface all other executable languages. Quoin remains in its current implementation for now: contain its spread through versioned structured interfaces, do not rewrite it wholesale. Filament changes are excluded. See AGENTS.md and the language gate on the owning issue. Normative prose/schema need not be executable Rust.
