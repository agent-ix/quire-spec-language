# ConfigVersion workflow

This newly authored AGPL-3.0-only example compiles the named ConfigVersion model
through the production native model frontend and runs actual source, snapshot and
invocation files. Its package `example/config-version`, model revision 1, explicit
reference carrier and `config_history` universe belong to this native realization.
They are not renamed historical Filament artifacts or a conversion of SemVer to
an IR revision. Historical standard/producer fixtures remain unchanged.

Generate inputs locally, then check one rule from the `1-draft` unit:

```sh
cargo run --locked --example config_version_fixtures -- /tmp/config-version
examples/config-version/requests.sh /tmp/config-version/healthy-parent ParentOrder
cargo run --locked -- run /tmp/config-version/healthy-parent/run.json
cargo run --locked -- compile /tmp/config-version/healthy-parent/compile.json
```

`requests.sh` writes `run.json` and `compile.json` beside the generated files.
They select `spine-unit.native` and `spine-model.semantic-ir.json` by digest,
and `run.json` checks the named clause for the object keyed `child` in
`spine-snapshot.json`. Through this path `healthy-parent` exits 0,
`violating-parent` 10, `dangling-parent` 20 and `incomplete-population` 22;
`cycle` with `NoCycle` exits 10.

The generated `request.json` and `markdown-run.json` select the historical
`0-draft` `program.native`. The `run` command now refuses those as retired
(exit 20), so the table below describes the native fixtures' intended
outcomes; the spine tests in `tests/it/config_version_spine.rs` check the
`1-draft` path.

Each directory contains `model.json`, `program.native`, `rules.md`, the applicable
snapshot/invocation files, and `request.json`, `markdown-run.json`, `compile.json`.
All selected digests are computed from the actual files. Native and extracted
bodies have distinct source identities; each case has its own runtime identities.

FR-108 writes the spine counterpart of every native file into the
same directory: `spine-unit.native` (the shared `1-draft` unit),
`spine-model.semantic-ir.json` (the Semantic IR 2.0.0 domain package),
`spine-*` snapshot/invocation documents, and `clause-run-request.json` (the
`ClauseRunRequest` inputs `tests/it/config_version_spine.rs` reads through
`run_clause`, FR-109).

| Directory | Expected outcome |
| --- | --- |
| healthy-parent | completed true: parent version 1 precedes child 2 |
| violating-parent | completed false: parent 3 does not precede child 2 |
| absent-parent | completed true: the absent parent's dereference is skipped |
| cycle | completed false: NoCycle detects the two-object cycle |
| self-loop | completed false: NoCycle detects the self-loop |
| distinct-identities | completed false: equal-valued objects have different identities |
| unchanged-version | completed true: post version equals its pre value |
| changed-version | completed false: a permitted version update violates VersionUnchanged |
| forbidden-parent-change | refused: changing parent violates the model's frame |
| dangling-parent | refused: the complete population lacks the parent target |
| incomplete-population | incomplete: the offered population is not complete |
| missing-model | refused: the selected model import is unavailable |
| exhausted-work | incomplete: expression work is limited to zero |
| boundary-zero | completed true: root version 0, child 1 (QSpec FR-180-AC-4's lower bound) |
| boundary-max | completed true: root version 999, child 1000 (QSpec FR-180-AC-4's upper bound) |
| below-range | refused: root version -1 is outside the domain's nominal bounds |
| above-range | refused: child version 1001 is outside the domain's nominal bounds |

Completed true exits 0, completed false 10, refused 20 and incomplete 22 (see
[the diagnostic catalog](../../docs/native-error-codes.md)). Only completed
execution includes truth. The Markdown path retains Quire's unchecked-language advisory
and available/lossy extraction separately from native execution. These are real
reference-runtime examples. The model declares integer bounds and explicit
finite input populations, without adding a new schema-level
population-cardinality feature.
