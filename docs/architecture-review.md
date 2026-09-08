# Language/core architecture review — 2026-09-07

Scope: Agent A's draft compiler and state-core specification boundary. This
review was requested by the owner during LC01 implementation. It does not claim
review of B's implementation, C's adapters or the temporal owner's work.

## Findings and disposition

| Priority | Evidence in the initial draft | Change / decision |
|---|---|---|
| High | Handwritten byte scanner, duplicated keyword spelling tables, and custom JSON escape/surrogate decoding | Replace with Logos declarative token definitions and serde_json string decoding. Grammar-specific checks remain explicit. |
| High | Every token span counted characters again from the line start; long lines could cost quadratic work | Syntax/tokens now use compact byte spans. Immutable Source indexes line starts and cumulative multibyte excess once; coordinate queries use binary search. |
| High | Parenthesized primary returned its inner node, losing grouping bytes from surrounding expression spans | Preserve Group nodes with exact delimiter-inclusive spans. Lowering may later erase grouping while preserving derivation loci. |
| Medium | A recursive AST printer would duplicate grammar and risk stack exhaustion on long operator chains | Format the validated original token stream by changing whitespace. Preserve comments, literal spellings and grouping. Test structural round trips and idempotence. |
| Medium | Each fallible parser frame carried a 152-byte diagnostic on the successful path | Box diagnostics on the failure path; retain structured source coordinates. |
| Medium | Public expression lookup indexed an arena directly and could panic for an out-of-range handle | Checked lookup returns Option; ExprId is documented as local to a unit. ParsedUnit owns immutable syntax; no AST mutation/deserialization is exposed. |
| High | Operation fixture observations reused an ID for different populations; invocations reused IDs across cases; frame references omitted exact content/version | Corrected with case-scoped identities and byte-bound artifact references. Producer audit checked 23 files / 7 cases and six stale-digest/content-reuse controls. C independently reviews/consumes this contract; model binding remains unavailable. |
| High, next stage | A generic successful parse could be mistaken for a linked executable subject | Keep ParsedUnit as syntax-only. The CLI reports parsed, never healthy/verified. Linking needs a separate constructor/type with checked model/import/source bindings. |
| High | Extracted body offsets could be applied to the original document without proving correspondence | Implemented a bounded SourceMap validator with exact SHA-256 sources, complete UTF-8 segments, explicit layout deletion rules and discontiguous original locations. Registered wire/authority bindings remain separate. |
| High, next stage | Full source refs, numeric IR revision mappings, dependency closure and runtime identities are not represented by parser metadata alone | Parser identity/revision labels are diagnostic metadata, not a registered interchange SourceRef. FS02/FS05 must define validated refs before evidence/cache reuse. No invented hashes/revisions or permissive fixture reader. |
| High, next stage | Imported names currently have declaration/clause-wide syntax loci; precise unresolved-name diagnostics need individual reference loci | Completed in LC01 follow-up: names and import literals now carry their own exact token spans. This supplies diagnostic locations without re-parsing source; resolution still requires the reviewed linker contract. |

## Boundaries retained

One Rust package is sufficient for the current consumers. Source, tokenization,
syntax, parsing, formatting and diagnostics are modules. Binding/lowering stay
modules until independent consumers justify a crate. No empty evaluator/backend
crates are created to match a diagram.

Logos recognizes tokens; the parser handles declaration structure and uses a
Pratt precedence table for expressions. Typed dispatch expresses grammar rules.
Conditional control flow itself is not a defect: matching keywords is lexing,
while selecting a parse production is parsing. A parser generator is not needed
to fix the scanner. Reconsider grammar generation if recovery, ambiguity or
multiple independently maintained syntax implementations justify it.

The syntax arena avoids recursive destruction and traversal for long left and
unary chains. Recursive grammar paths and delimiter depth are bounded, as are
source bytes, tokens, syntax nodes and formatted output. Exceeding any budget
produces incompleteness; no parser recovery creates a partial success. Current
limits are implementation ceilings, not model bounds or evaluation fuel rules.

The next pipeline is `ParsedUnit -> LinkedPackage -> qualified projection`.
Those arrows are validation boundaries, not interchangeable serializations.
The existing semantic-core owns domain declaration meaning and the existing
contract IR owns its strict executable binder. LC02 resolves references against
that model boundary; LC04 lowers only qualified features into the existing IR.
The inspected IR lacks identity-bearing references; recursive record encoding
would change meaning and is explicitly rejected. No new model authority or
parallel executable binder is warranted.

Static linked packages exclude populations and invocation state. Runtime inputs
must independently validate type/universe/object identity, closure, observations,
frames and invocation correspondence before evaluation. Unknown versions and
unavailable bindings refuse or remain incomplete. A Boolean is only available
after successful input validation and completed evaluation. B owns the portable
result envelope; A must not introduce a competing truth/result protocol.

Quire extraction remains opaque. C's adapter must map extracted body bytes to
the exact original document, using segments for indentation/normalization.
The native compiler retains its own original source bytes and reports half-open
byte spans; it does not add a second expression compiler to Markdown extraction.

## Validation and limits

Regression coverage targets operator precedence/associativity, exact grouping
spans, UTF-8/CRLF coordinates, JSON escapes, recognized unsupported forms versus
malformed input, resource exhaustion, comment-preserving parse/format round trips,
long flat chains on a bounded stack and a deterministic malformed corpus.
These checks qualify syntax behavior only. They are not typechecking, evaluator
conformance, a proof of parser safety or completed LC01–LC05 acceptance.

Library basis: [Logos 0.16.1](https://docs.rs/logos/0.16.1/logos/) generates a
recognizer from token definitions and exposes original byte spans;
[serde_json](https://docs.rs/serde_json/1.0.151/serde_json/fn.from_str.html) supplies
the string codec. Neither library defines this language's semantic profile.
