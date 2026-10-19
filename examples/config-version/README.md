# ConfigVersion example

This example checks the rules from the top-level README's
[first rule](../../README.md#a-first-rule) against seventeen small data sets.
The unit is `spine-unit.native` (edition `1-draft`). Its domain model is the
`example/config-version` package, and each case has a snapshot of
`ConfigVersion` objects. The example and its files are AGPL-3.0-only.

Generate the cases, then check one rule from the command line:

```sh
cargo run --locked --example config_version_fixtures -- /tmp/config-version
examples/config-version/requests.sh /tmp/config-version/healthy-parent ParentOrder
cargo run --locked -- run /tmp/config-version/healthy-parent/run.json
cargo run --locked -- compile /tmp/config-version/healthy-parent/compile.json
```

The generator writes one directory per case. Each holds `spine-unit.native`,
`spine-model.semantic-ir.json` (the domain model), `spine-snapshot.json` and,
for operation cases, invocation documents. [`requests.sh`](requests.sh) then
writes `run.json` and `compile.json` beside them, selecting each file by its
digest. `run.json` checks the named clause for the object keyed `child`; the
[run request reference](../../docs/native-standalone.md) describes every
member.

| Case | Expected outcome | Exit |
| --- | --- | --- |
| healthy-parent | holds: parent version 1 precedes child 2 | 0 |
| absent-parent | holds: there is no parent to compare | 0 |
| unchanged-version | holds: the version after an update equals the version before | 0 |
| boundary-zero | holds: root version 0, child 1, the lowest allowed values | 0 |
| boundary-max | holds: root version 999, child 1000, the highest allowed values | 0 |
| violating-parent | broken: parent 3 does not precede child 2 | 10 |
| cycle | broken: `NoCycle` finds a two-object cycle | 10 |
| self-loop | broken: `NoCycle` finds an object that is its own parent | 10 |
| distinct-identities | broken: two equal-valued objects are still different objects | 10 |
| changed-version | broken: an update changes the version, against `VersionUnchanged` | 10 |
| dangling-parent | refused: the parent is not in the data | 20 |
| forbidden-parent-change | refused: the update changes a field the model forbids it to change | 20 |
| missing-model | refused: the imported model is not supplied | 20 |
| below-range | refused: root version -1 is outside the model's bounds | 20 |
| above-range | refused: child version 1001 is outside the model's bounds | 20 |
| incomplete-population | incomplete: the data does not claim to hold every object | 22 |
| exhausted-work | incomplete: the work budget is zero | 22 |

[`tests/it/config_version_spine.rs`](../../tests/it/config_version_spine.rs)
checks every row through the library, and runs `healthy-parent` and
`violating-parent` through the `run` command. With `requests.sh` the command
line reproduces the state-rule rows directly: `healthy-parent` 0,
`violating-parent` 10, `cycle` and `self-loop` with `NoCycle` 10,
`dangling-parent` 20 and `incomplete-population` 22. The other rows need an
operation frame, a missing import or a work limit in the request, which the
test builds.

The directories also still contain `program.native`, `request.json` and
`markdown-run.json` for the retired `0-draft` edition; `run` refuses those
with exit 20.
