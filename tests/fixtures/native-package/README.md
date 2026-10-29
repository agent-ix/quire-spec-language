# Native package vector provenance

The Rust vector recipe in tests/package_construction_cases/vectors.rs
independently composes a complete positive manifest for a real imported model and constant
clause. Model loci follow physical input-fragment boundaries; native offsets
follow fixed clause fragments. Package fields/order, Unicode escapes and domain
prefix are authored independently of the package encoder. The opaque admitted
model artifact is an upstream input, quoted by existing Serde. No production
package output, parsed AST or checked manifest supplies the expected fields.

## Positive vectors

The `minimal`, `controls` and `multiple` vectors (tests/support/package_vector_setup.rs)
are composed by that recipe at test time from the offered model: the native
source, the canonical JSON bytes, the complete package JSON bytes and the
domain-prefixed SHA-256 identity. Every digest they carry (the model's, the
source's and the package's) is computed from the bytes it names when the test
runs, so no digest is frozen in a file.

`model-source.json` is the upstream model input. `model-artifact.json` is its
admitted artifact as written before the package producer existed; its
`{source_digest}` is the SHA-256 of `model-source.json`, filled in at test time.
`header.native` is the shared unit header.
