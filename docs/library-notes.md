# Library notes

Reference details for calling QSL as a Rust library. The
[README](../README.md) explains the language and the command line.

## Source identity and locations

- Source identity and revision labels are opaque diagnostic labels: authority,
  id, revision namespace and revision. They are not interchange source
  references.
- `Source::read_verified` checks a selected byte digest. `parse_source` keeps
  that verified, immutable source.
- Byte spans are half-open. Line and column numbers start at one, and columns
  count Unicode scalar values.
- CLI identity labels must be UTF-8. Invalid encoding exits 20. File operands
  are OS paths.

## Parse limits

The default ceilings are:

| Limit | Default |
| --- | --- |
| Source size | 1 MiB |
| Tokens | 100,000 |
| Syntax nodes | 50,000 |
| Nested delimiters and parser frames | 64 |

Callers can lower any of them. Hitting a ceiling gives *incomplete*, never a partial unit.
Malformed tokens and delimiters are syntax errors. Recovery never manufactures
a successful partial unit.

## Formatting

- `qsl_cst::format::format(unit)` keeps the default 1 MiB output ceiling.
- `format_with_limit(unit, output_bytes)` replaces that ceiling. The value is
  used as given, including zero and values above the default.
- `format_with_limits` takes a typed `FormatLimits`.
- The limit counts every emitted byte, including the final newline, and the
  formatter refuses before an append would exceed it.
- Formatting keeps comments and token spellings, normalizes whitespace and
  writes LF line endings.
- A formatted document is new source. Give it a new revision before you bind
  it into evidence.

## Rules from Markdown (`quire-extraction`)

`qsl_source::extract(original, context, selection, limits)` in the `qsl-source`
crate calls quire-rs's extractor and returns an `ExtractedSource`, which holds:

- the verified rule body in its document source map;
- the language the fence declared;
- Quire's unchanged extraction result.

It takes these inputs:

- `original`: a digest-verified `Source`.
- `context`: a loaded Quire `SemanticContext`. The `run` command builds it with
  `qsl_source::clause_context`.
- `selection`: a `Selection` naming the authored clause id, the package and the
  body identity.

The adapter checks source coordinates and exact bytes before it builds the
source map, including the final LF that Quire omits on CRLF input. It does not
compile. `mapped::compile` carries the verified map through
parse, link, check and package, so diagnostics point into the original
document. Input is bounded to 1 MiB and 4096 lines.

The tests in [`qsl-source/tests/it/quire_source.rs`](../qsl-source/tests/it/quire_source.rs)
show how the context and the selection are built. Run them with:

```sh
cargo test -p qsl-source --features quire-extraction
```

## Further reference

- [Source correspondence](source-correspondence.md): mapping extracted bodies
  back to the original document.
- [Run requests](native-standalone.md): the `native-run/1` and
  `native-compile/1` request envelopes.
- [Checked packages](native-linked-packages.md),
  [runtime inputs](native-runtime-inputs.md) and
  [evaluation](native-runtime-evaluation.md).
- [Diagnostic codes](native-error-codes.md).
- [Compiled protocol wire formats](compiled-protocol-v1.md) and
  [v2](compiled-protocol-v2.md).
