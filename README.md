# quire-spec-language

Private native syntax implementation for `ix:native`, edition `0-draft`, profile
`state-finite/0-draft`. It parses and formats source with located diagnostics.
Imports remain unresolved: successful parsing does not establish type correctness,
model binding, executable projection, or healthy/violating state evaluation.

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
python3 tools/audit_review_fixtures.py --self-test
```

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
