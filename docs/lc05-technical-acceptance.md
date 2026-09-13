# LC05 technical acceptance packet

Accepted compiler revision: `78828d84dc642c2ee61817494bb230321d5bf716`
(`quire-spec-language` main, 2026-09-12).

## Admitted workflow

The named ConfigVersion workflow runs end to end in the explicit
`ix:native` / `0-draft` / `state-finite/0-draft` profile. The Rust fixture
generator emits the selected model, native and Markdown sources, snapshots,
invocations, compile requests and run requests. The production paths then cover
source extraction, model admission, compilation, package verification, runtime
validation and evaluation.

The thirteen cases in `examples/config-version/README.md` retain distinct
source, package and runtime identities and reproduce healthy, violating,
refused and incomplete results. Standalone native execution has no Filament,
Quoin, Node, JVM or network dependency. Quire extraction remains an explicit
`quire-extraction` Cargo feature and does not become a second compiler.

## Setup effort

Rust 1.98.1 and `Cargo.lock` are the reproducible toolchain boundary. The native
path needs only Cargo and the checked-in inputs. The extracted path additionally
needs access to the pinned private `quire-rs` dependency.

The implementation landed as eleven scoped PRs, #14 through #24, followed by
the runtime wire and schema corrections in #32 and #34. The auditable delivery
window ran from creation of #14 on 2026-09-09 to merge of #24 on 2026-09-10,
about eight hours of wall-clock PR activity. No person-hour log was collected,
so this packet makes no labor-duration claim.

Reproduce the representative paths with:

```sh
nice -n 10 cargo run --locked --offline --target-dir target -j 1 --example config_version_fixtures -- /tmp/config-version
nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- run /tmp/config-version/healthy-parent/request.json
nice -n 10 cargo run --locked --offline --target-dir target -j 1 --features quire-extraction -- run /tmp/config-version/healthy-parent/markdown-run.json
```

## Artifact lineage

| Stage | Owned artifact or interface |
| --- | --- |
| Authored model | `examples/config-version/model.json`, AGPL-3.0-only |
| Case catalog and generator | `examples/config-version/cases.rs` and `fixtures.rs`, Rust |
| Native source path | generated `program.native` → model/link/check/package pipeline |
| Quire source path | generated `rules.md` → pinned Rust extractor → mapped compiler intake |
| Runtime inputs | generated snapshot/invocation files with case-specific identities |
| Compiled artifact | `native-linked-package/1`, selected and verified by exact digest |
| Result | native runtime report with completed/refused/incomplete stage and provenance |
| Qualification | FR-032, TC-110 and `tests/config_version.rs` |

All selected digests are derived from actual emitted bytes. Native and extracted
source identities remain distinct. Generated example inputs are not independent
execution evidence; the Rust integration tests and actual command results own
the outcome assertions.

## Recorded support gaps

These gaps do not block the admitted LC05 workflow and remain owned separately:

- #27: finish the producer-owned Contract IR resource-classification predicate.
- #29: replace the remaining positional CLI dispatch with one typed parser.
- #30: finish the historical source-profile ruling reconciliation.
- #42: bind the reserved-construct inventory to the language-progress metric.
- #68: correct a native-admission diagnostic whose source and span can disagree.

Numeric/object generated-backend coverage and broader ecosystem workflows remain
downstream capabilities. LC05 establishes one real state workflow, not a full
application release or universal backend support.
