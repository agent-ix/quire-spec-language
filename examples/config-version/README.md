# ConfigVersion workflow

This newly authored AGPL-3.0-only example compiles the named ConfigVersion model
through the production native model frontend and runs actual source, snapshot and
invocation files. Its package `example/config-version`, model revision 1, explicit
reference carrier and `config_history` universe belong to this native realization.
They are not renamed historical Filament artifacts or a conversion of SemVer to
an IR revision. Historical standard/producer fixtures remain unchanged.

Generate inputs locally:

```sh
nice -n 10 cargo run --locked --offline --target-dir target-codex-backends -j 1 --example config_version_fixtures -- /tmp/config-version
nice -n 10 cargo run --locked --offline --target-dir target-codex-backends -j 1 -- run /tmp/config-version/healthy-parent/request.json
nice -n 10 cargo run --locked --offline --target-dir target-codex-backends -j 1 --features quire-extraction -- run /tmp/config-version/healthy-parent/markdown-run.json
```

Each directory contains `model.json`, `program.native`, `rules.md`, the applicable
snapshot/invocation files, and `request.json`, `markdown-run.json`, `compile.json`.
All selected digests are computed from the actual files. Native and extracted
bodies have distinct source identities; each case has its own runtime identities.

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

Completed false and refusal exit 1; incomplete exits 3. Only completed execution
includes truth. The Markdown path retains Quire's unchecked-language advisory
and available/lossy extraction separately from native execution. These are real
reference-runtime examples. The numeric backend accepts the primitive
`VersionUnchanged` state comparison and generates Rust oracle, proptest and Kani
artifacts. Object/graph clauses still refuse explicitly at their first
unrepresentable source locus. The model declares integer bounds and explicit
finite input populations, without adding a new schema-level
population-cardinality feature.

Export the update rule's primitive state projection:

```sh
nice -n 10 cargo run --locked --offline --target-dir target-codex-backends -j 1 -- lower /tmp/config-version/unchanged-version/compile.json --target state-scalar-ir/v1
```

The strict IR binder accepts this rule with separate pre/post version inputs.
Library consumers use `NativeProjection::inputs` after native runtime validation
to obtain actual values and their artifact/object provenance. IT-010 compiles and
executes its generated oracle and all strategy populations, proves the identity
subject with cargo-kani 0.67.0, and replays the violating subject's concrete
counterexample through `runtime::execute`.
