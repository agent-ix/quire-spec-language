# Source correspondence API

The native compiler now validates exact body-to-original byte correspondence.
This is an in-process Rust API, not a registered FS05 wire reader, Markdown
extractor or model binding. C retains the existing-repository adapter and its
independent consumer tests. The optional `quire_source` consumer now calls the
existing Rust extractor inside this compiler, validates its exact reported body
and uses this map. Its CRLF adaptation drops only the verified final newline;
the original Quire output remains available unchanged.

## Loading and parsing

`Source::read` accepts explicit source identity/revision/path and bounded UTF-8
bytes, preserving the original content. The source stores its actual SHA-256;
`Source::read_verified` additionally compares it with an independently selected
expected ByteDigest. Digests bind bytes only and use sha2 0.10.9, the same pinned
version inspected in contract IR. No contract-IR canonical identity is redefined.

`parse_source` consumes that immutable source directly, retaining the same bytes,
identity and digest in ParsedUnit. It rechecks the caller's source-byte limit
before tokenization. The existing `parse` convenience entry point loads bytes
and delegates to this path. The CLI's successful parse output includes the actual
source digest; it does not verify an expected artifact reference supplied elsewhere.

SourceIdentity remains local parser metadata. Registered authority, revision
namespace mapping, original obligation identity, positive numeric IR revision
mapping, and source artifact selection belong to the reviewed shared contract.
An adapter must verify those bindings as well as bytes; an arbitrary digest
computed from arbitrary input does not authorize that input as an authored source.

## Verifying a map

`SourceMap::verify(original, body, region, segments, layout, segment_limit)`
validates all of the following before returning an immutable map:

- the selected original region and every segment are half-open UTF-8 boundaries;
- nonempty body segments cover every body byte exactly once in order;
- corresponding original bytes are identical and appear monotonically inside
  the selected original region, with no overlap;
- skipped original bytes are admitted by the explicitly selected Layout policy;
- original and body do not reuse one identity/revision for different bytes;
- the map is within the caller's segment budget and the 50,000 segment ceiling.

Default Layout admits only verbatim correspondence. Three independent flags can
permit removal of actual leading ASCII spaces/tabs, CR in CRLF, and the final
LF/CRLF. Leading indentation is checked against original line boundaries, with
an index constructed once. Interior spaces, interior newlines, bare CR and
non-layout bytes cannot be dropped. No inserted or rewritten bytes are accepted.
This API does not infer a transform from visual similarity or expand tab widths.
The extraction adapter supplies its declared policy and verified region selection.

The full original document is retained. Text outside the selected region is not
silently adopted as a clause; proving which authored clause selects that region
remains the adapter/source-manifest obligation. An API-valid map alone does not
prove that the extractor selected the correct fence or obligation.

## Mapping diagnostics and regions

`map_span(source, span)` requires the exact body identity, revision, path and
content digest. Passing a different Source with the same offsets refuses.
Returned LocatedSpans retain original byte offsets and one-based Unicode scalar
line/column coordinates. Concatenating their original slices reproduces the
selected body bytes exactly. Discontiguous regions remain separate; their
bounding box must not be used as an exact byte/digest correspondence.

At a zero-width segment boundary, select the next original segment. At body EOF,
select the last mapped byte's end, before a trimmed final newline. An empty body
maps its zero-width location to the selected region start. Original document and
body EOF remain distinct when extraction drops layout.

The parser still consumes the admitted complete native unit grammar. A map does
not invent language headers, imports or missing source text to make an extracted
fragment parse successfully. Generated wrapper text requires its own explicitly
identified derivation and a separately qualified source correspondence contract.

## Reproducible checks

```sh
cargo test --locked --target-dir target --test it source_map:: --no-default-features
```

The tests include a parser unsupported diagnostic mapped through an indented
CRLF native unit to a Unicode original document; exact and discontiguous regions;
EOF behavior; explicit transform refusal; same-identity changed bytes; stale
expected digest; malformed/out-of-order/overlapping/omitted segments; split UTF-8
boundaries; extreme offsets and exhausted budgets. These are executable source
checks, not linked state evaluation or completion of existing adapter delivery.
