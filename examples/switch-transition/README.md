# Switch transition example

The authored Boolean Switch pair: a domain model with a `Switch` object whose
Boolean field `on` is modified by the operation `set(flag)`, and a complete-V1
source with one `post` clause over `set`. The producer derives one
`state`/`transition` node for the Boolean model field (FR-105-AC-13). Both
files are QSL-owned and AGPL-3.0-only; a reader copies nothing, it passes
these paths.

| File | Role |
| --- | --- |
| `switch.native.in` | the QSL source template (edition `1-draft`); its `model` line carries the placeholder `{model_digest}`, never a digest |
| `switch.semantic-ir.json` | the domain model package document the source selects by digest |

FR-056 selects models by digest, so no committed file carries it: a model edit
would silently stale it. The digest is derived from `switch.semantic-ir.json`
at run time, by `emit_checked_v2`.

Emit the `quire.checked-package/v2` bytes from the repository root (the
placeholder is filled in memory from the single `--model`):

```sh
cargo run --locked --example emit_checked_v2 -- \
  --model examples/switch-transition/switch.semantic-ir.json \
  examples/switch-transition/switch.native.in
```

To also write the rendered, digest-carrying source (the generated
`switch.native`, for a consumer that needs the source file itself), add
`--source-out`:

```sh
mkdir -p target/switch-transition
cargo run --locked --example emit_checked_v2 -- \
  --model examples/switch-transition/switch.semantic-ir.json \
  --source-out target/switch-transition/switch.native \
  examples/switch-transition/switch.native.in
```

The generated pair is `target/switch-transition/switch.native` plus
`examples/switch-transition/switch.semantic-ir.json`; `target/` is untracked,
so nothing generated is committed. The emitted bytes embed the source path
exactly as given, so a caller that needs stable bytes passes the canonical
repository-relative paths above from the repository root every time.

Tests (`tests/it/emit_checked_v2.rs`) render the template themselves, derive
the digest from the model file, and never assert a literal; one test scans this
directory and fails if any file carries a `sha256-jcs:` digest literal.
