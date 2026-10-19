# Run and compile requests

`quire-spec run` and `quire-spec compile` each read one JSON request file. The
request selects every input by file name and digest, so a result always says
exactly which source, model and data it was computed from. Nothing else is
needed: no network service and no other tool.

This page describes requests for `1-draft` units. The quickest way to see one
is the ConfigVersion example:

```sh
cargo run --locked --example config_version_fixtures -- /tmp/config-version
examples/config-version/requests.sh /tmp/config-version/healthy-parent ParentOrder
target/debug/quire-spec run /tmp/config-version/healthy-parent/run.json
target/debug/quire-spec compile /tmp/config-version/healthy-parent/compile.json
```

[`requests.sh`](../examples/config-version/requests.sh) writes the two request
files described below.

## Selecting a file

Every source file is selected with the same object:

| Member | Meaning |
| --- | --- |
| `file` | Path relative to the request file. |
| `digest` | `sha256:` and the file's lowercase hex SHA-256. A mismatch refuses the request. |
| `authority`, `identity`, `revision_namespace`, `revision` | The source identity carried by every diagnostic. |
| `document`, `formal_revision` | The document name and revision recorded in the result. |

## Compile request

```json
{"format": "native-compile/1", "request": {
  "models":  [{"format": "semantic-ir/2.0.0", "source": { …file selection… }}],
  "program": {"source": { …file selection of the unit… }},
  "libraries": []
}}
```

`models` lists the domain models the unit imports. `libraries` is optional and
names other QSL units the program depends on, each as `{"identity", "source"}`.
On success stdout is the checked package as `quire.checked-package/v2` JSON.

## Run request

A run request has the same `models`, `program` and optional `libraries`, plus
exactly one of `call` or `clause`.

**`call`** evaluates one pure function:

```json
"call": {"function": "lt", "arguments": [
  {"parameter": "a", "value": 3}, {"parameter": "b", "value": 5}]}
```

**`clause`** checks a rule against data. `observations` maps names to snapshot
or invocation files, and `selection` names exactly one of `clause`, `frame` or
`function`:

```json
"clause": {
  "selection": {"clause": {
    "name": "ParentOrder",
    "observation": "current",
    "self": {"population": "ix://example/config-version/config_history", "key": "child"},
    "anchor": {"kind": "handler", "name": "validate"}}},
  "observations": {"current": "spine-snapshot.json"}
}
```

| Selection | Checks |
| --- | --- |
| `clause` | An invariant or `pre`/`post` rule for one object (`self`) in the named observation, at an optional `anchor` (`initialization` or `handler`). |
| `frame` | That an operation, named `M::Type::op`, only changed what its frame allows, using the named `invocation` document. |
| `function` | A function run as a claim over the named `snapshot`, with its `arguments`. |

`call` and `clause` both take an optional `accounting` object that lowers any of
the work counters; the defaults are generous.

## Results and exit codes

A run prints one JSON result on stdout and ends with the outcome's exit code:

| Exit | Outcome |
| --- | --- |
| 0 | completed, and the rule holds (or the function returned) |
| 10 | completed, and the rule is broken; or undefined |
| 20 | refused: a malformed request, a digest mismatch, a dangling reference, a value outside its type |
| 21 | unsupported by this profile or backend |
| 22 | incomplete: the data does not claim to be complete, or a work counter ran out |
| 30 | internal failure |

Only a completed run carries a truth value. A refused or incomplete result has
none, so it can never be read as *false*. Request errors are JSON on stderr.
The [diagnostic codes](native-error-codes.md) page lists every code.

The request shapes are pinned by the CLI tests in
[`tests/it/config_version_spine.rs`](../tests/it/config_version_spine.rs) and
[`tests/it/spine_run.rs`](../tests/it/spine_run.rs).
