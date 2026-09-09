# Native package vector provenance

New fixture data is AGPL-3.0-only. The two header-only canonical JSON files were
authored before the package encoder existed, but their positive setup was wrong:
the adopted native grammar requires an import and a clause. They are retained
as adverse empty-inventory data for TC-087; neither is a qualified producer
vector. They contain no final newline. Their source is header.native; both bind its
exact SHA-256 2bba5308efe2f3b5f684f062522b124e4a867549936f6944a83acc04bba85c27.
The Unicode case uses revision 9007199254740993 and literal expected JSON
escapes; no JavaScript numeric serialization supplies that value.

After specification correction 2c6b9b8 and all-eight re-review at 1c3aa50, the
Rust vector helper in tests/package_construction_cases/vectors.rs independently
composes a complete positive manifest for a real imported model and constant
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

The `minimal`, `controls` and `multiple` families each contain exact native
source, canonical JSON bytes, complete package JSON bytes and the expected
domain-prefixed SHA-256 in a `.sha256` file. JSON files have no final newline;
digest text has one. `model-source.json` and `model-artifact.json` freeze the
upstream model input and admitted bytes. The public producer test compares its
complete output with these files; a private unit test observes the actual
canonical pass and compares its bytes before hashing. Neither test rewrites
fixtures or substitutes the package compiler.

| Family | Canonical SHA-256 |
| --- | --- |
| minimal | `1f2de52410e43349c2528140e94952e4a888d209dcc8cb2ad783fef0683ebb61` |
| controls | `a07beb9fa48bc4742df3f51fdc74bc6938c53ad4de4842c3b134c2ef6035ca8f` |
| multiple | `ebafb411e9a74f33b5685403ae45d8f8b3e6e223c4a0afff5d7ec3485304b2d7` |

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
maintenance executable and assertions are Rust under AGPL-3.0-only. Fixed
positive data was authored after the initial producer, from the corrected
independent recipe; the earlier failed setup evidence remains unchanged.
