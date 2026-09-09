# Native source correspondence for Contract IR

FR-014 defines a local Rust correspondence API over existing Source and Contract
IR source types at dependency revision 690bde7f2dc58662cf9ff0595c2c0e3b17107c6f.
The public module is `formal_source`:

```rust
FormalSource::new(source: Source, identity: quire_contract_ir::SourceIdentity) -> FormalSource
FormalSource::source(&self) -> &Source
FormalSource::identity(&self) -> &quire_contract_ir::SourceIdentity
FormalSource::to_ir(&self, source: &Source, span: Span)
    -> Result<quire_contract_ir::SourceSpan, Box<Diagnostic>>
FormalSource::to_native(&self, span: &quire_contract_ir::SourceSpan)
    -> Result<Span, Box<Diagnostic>>
```

`new` is an explicit assignment made by the caller. Both inputs already have
validated constructors. The caller owns choosing the appropriate formal document
and positive revision; native opaque labels are never parsed, hashed into a
revision, normalized or replaced. The binding retains an inexpensive immutable
Source clone, including its native labels, original path and byte digest.
There is no global registry, conflict resolution across bindings or wire format.
Later package assembly must retain these bindings and enforce its own inventory
uniqueness. Constructing a binding supplies no semantic or provenance verdict.

Forward mapping requires exact equality of native labels, path and byte digest
with the stored Source. Path is a local request guard, not a portable identity.
Offsets are checked through the existing Source coordinate API before conversion
to the actual IR SourceLocation and SourceSpan constructors. No duplicated UTF-8
index is maintained. CR counts as a scalar; LF advances the line. Empty spans,
including an empty document's zero boundary and EOF, are valid.

Reverse mapping first checks the formal identity, then both byte offsets and
their recomputed line/scalar-column coordinates against the retained source.
An IR SourceSpan constructor only checks structural monotonicity and cannot
validate its coordinates against bytes it does not own. This bridge supplies
that check. Invalid spans never become a guessed or clamped source position.

Both directions return `invalid_source_map` in phase `source_map` on a mapping
failure, with native diagnostic coordinates at bound byte zero, no related
declarations and no Boolean result. If an IR constructor returns a diagnostic,
that structured diagnostic remains available in `upstream`. The existing source
ceiling bounds reachable coordinates well within the IR numeric widths; checked
conversions also reject oversized incoming IR offsets. Mapping does not allocate
in proportion to the source or start threads, processes, callbacks or filesystem
work. It uses the source's existing indexed coordinate lookup. The API adds no
separate resource budget because each request maps exactly two endpoints.

The binding is immutable and reusable after refusals. Independently loaded
sources with identical labels, path and digest are equivalent forward requests;
Arc pointer identity is not the correspondence rule. Source::read_verified
remains responsible for checking an independently selected digest during intake.
SourceMap still owns extraction layout and discontiguous original regions.
Composing these APIs requires callers to retain each exact original segment;
this API neither combines those regions nor replaces the extraction contract.

Qualification uses actual pinned IR types and Rust assertions, including an
independent coordinate oracle and deliberately misleading constructor-valid IR
locations. There are no new dependencies or non-Rust qualification helpers.
Type/definedness checking, native model semantic roles, lowering and evaluation
remain subsequent LC02–LC05 work.
