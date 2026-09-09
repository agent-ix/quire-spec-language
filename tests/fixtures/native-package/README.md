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
