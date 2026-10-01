# Native package vector provenance

The Rust vector helper in tests/package_construction_cases/vectors.rs
independently composes a complete positive manifest for a real imported model and constant
clause. Model loci follow physical input-fragment boundaries; native offsets
follow fixed clause fragments. Package fields/order, Unicode escapes and domain
prefix are authored independently of the package encoder. The opaque admitted
model artifact is an upstream input, quoted by existing Serde. No production
package output, parsed AST or checked manifest supplies the expected fields.

The corrected oracle follows the first producer implementation and records that
true sequence. Original missing-API and failed-setup logs remain in
reviews/data/native-packages/. Additional complete clause/operation vectors and
adversarial families remain required by Task-016/017; these first vectors do not
complete TC-090.

## Frozen positive vectors

ADR-013 §7 slice S-4b regenerated the three families: the manifest's
`source` gained `authority` (`agent-ix`) and `revision_namespace` (`draft`),
the four source labels of FR-001, so every canonical and package byte string
and every identity digest changed (FR-021-AC-1).

The `minimal`, `controls` and `multiple` families each contain exact native
source, canonical JSON bytes, complete package JSON bytes and the expected
domain-prefixed SHA-256 in a `.sha256` file. JSON files have no final newline;
digest text has one. `model-source.json` and `model-artifact.json` freeze the
upstream model input and admitted bytes. The public producer test compares its
complete output with these files; a private unit test observes the actual
canonical pass and compares its bytes before hashing. Neither test rewrites
fixtures or substitutes the package compiler.

The Rust maintenance example `examples/author_native_package_vectors.rs` writes
candidate files from the independent recipe above. It calls model admission
for the upstream input, but never the package producer, parser, linker or
checker. It accepts exactly one fresh output directory and refuses an existing
directory. Candidate review and promotion are explicit; ordinary tests never
regenerate expected data. On the shared desktop run it as a separate Cargo phase:

```sh
nice -n 10 cargo run --locked --offline --target-dir target -j 1 --example author_native_package_vectors -- /tmp/new-package-vector-candidates
```

This is domain-specific fixture authoring, not a general canonicalizer, model
authority or replacement for shared Quoin evidence tooling. The recipe, new
maintenance executable and assertions are Rust under AGPL-3.0-or-later. Fixed
positive data was authored after the initial producer, from the corrected
independent recipe; the earlier failed setup evidence remains unchanged.
