# quire-spec-language

`complete` exposes the edition `1-draft` authoring boundary. Its declarative
grammar table and one bounded interpreter produce a byte-exact CST with typed
token kinds, named production nodes and a separate recovery stream. The module
also provides revision-checked incremental edits, deterministic formatting and
editor snapshots, the closed 176-capability complete bundle, canonical package
identity and located diagnostics over the crate's existing code type. These APIs
establish source and package admission only; the authority-bound diagnostic
catalog and typed causes, checked extension compatibility, checked-package
manifests, later checking, runtime and backend stages remain separate typed
operations.
Public source/editor calls receive only an exact-reference `ProfileCatalog`.
Typed definition/model construction additionally requires a constructor-private
`ReaderAuthority`; Task-054 owns issuance from the concrete checked reader, so
raw bytes or caller-asserted capabilities cannot mint package authority here.

`parse_native` and `parse_native_source` select the authored edition and return
either a historical unit or a typed, source-located `1-draft` syntax unit.
The composed path extends the existing Logos recognizer and Pratt parser across
predicates, state, temporal and choreography forms. [Issue #35](https://github.com/agent-ix/quire-spec-language/issues/35)
tracks this work under [FR-035](spec/functional/FR-035-parse-composed-native-units.md);
[package linking](spec/functional/FR-036-link-composed-native-packages.md) continues
with `linking::composed::admit_namespace`: exact source inventory, package-wide
native names, typed declaration references and cycle/dependent refusals under
explicit work limits. `linking::composed::binding::bind` then resolves exact
supplied definition/rule closures, native model exports, lexical/capture scopes
and protocol structural references, preserving dependency-local refusals and
unfinished work. Its `NamesResolved` result precedes expression/profile checking
and complete runtime-role derivation. `checking::composed::admit_types` now checks
shared values across predicates, state, temporal and protocol declarations. It
retains exact nominal types, profiles, query shapes, capture provenance and
dependency-local refusals, with pending proof/runtime obligations. Its `Typed`
disposition establishes type admission only; exact authored formal sources supply
the existing IR rational normalizer. `checking::composed::proofs::discharge`
then checks supported guarded values through the existing IR prover and exact
authored `CheckBindings`, preserving actual goals and dependent refusals.
Ordered queries retain their original binder, collection and body through
definedness and native emission. Sum admission checks every prefix in supported
integer or denominator-one rational domains; broader rational sum transfer
remains explicitly unsupported. General family admission and runtime validation
remain open. Existing `parse`, format, CLI and checked-package APIs retain their
historical profile; parsing alone grants no semantic admission or execution.

Embedded normative resources retain their [original provenance and licensing](resources/native-v1/README.md).

The `protocol_artifact` library module supplies typed wire records, a bounded
canonical encoder and a parser-free reader under
[compiler #40](https://github.com/agent-ix/quire-spec-language/issues/40).
`read` checks independently selected artifact/source/dependency references,
actual admitted model exports, typed local handles and structural control edges.
`encode_candidate` returns transport bytes; its result and reader admission
remain separate from native compilation authority. `protocol_artifact::native::admit`
consumes the actual composed proof report and independently selected source,
definition and model inputs. It derives a constructor-private
`FamilyAdmission` from supported complete native families;
`native::emit` accepts only that authority and returns canonical bytes.
See the
[wire contract](docs/compiled-protocol-v1.md) for the exact encoding, identity
domains and limits. The [numeric component](spec/functional/FR-038-encode-exact-protocol-numbers.md)
uses tagged canonical decimal strings for signed-64 integers and reduced
rationals, with typed validation errors. Real source-to-reader tests exercise
native predicate, state, temporal and protocol emission, including static
compensation registration, activation, retries and full/partial recovery
requirements. The [Rust producer recipe](examples/protocol-handoff/README.md)
combines four source units, queries, population/reference roles and actual model
operations, retaining the executable bytes and independently derived selectors.
`protocol_artifact::handoff::PUBLISHED_V1_HANDOFF` now addresses that committed
`/1` handoff directly from the pinned crate. Version-explicit member constants
name its offer, external reference, `Selection` record and checksum inventory;
consumers need no environment variable, producer execution or repository-layout
guess. Its `Service` and `Provider` roles retain distinct admitted model
authorities so a strict consumer linker can enforce duplicate-authority refusal
without rejecting the positive corpus. The existing unversioned
`PUBLISHED_HANDOFF` family continues to address the unchanged `/2` corpus.
General dynamic choice/progress proofs, first-class relationship exports,
runtime recovery and B's public consumer acceptance remain open; the supported
boundary is stated in the wire contract.

`protocol_artifact::checked_predicate` and
`protocol_artifact::temporal_subject` publish the FR-051 native-owner handoffs.
Each module embeds immutable schema bytes and their SHA-256 digest, derives one
canonical static document from a strict `compiled-protocol/2` admission, and
reads it only by independently re-deriving the same package-bound document.
The readers are bounded and fail closed on changed, reordered, duplicated,
unknown, or trailing data. Their constructor-private views expose source,
clause, expression, binding, Boolean-type, profile, activation, clock, history,
and reachable-leaf authority; they expose no truth, observation, result, parser,
evaluator, callback, trust flag, or Boolean coercion. See the
[checked native handoff contract](docs/checked-native-handoffs.md).

`protocol_artifact::native_temporal::{request,result}` publishes the FR-052
formula-wide owner boundary over a strict FR-051 temporal subject. Canonical
requests bind complete explicit leaf valuations and independent progress,
closure and completeness evidence; canonical results obtain truth, settlement
and support only from the existing native evaluator. Strict result reading
re-evaluates the exact request and direct correction relation before admitting
bytes. This surface neither consumes nor emits a checked-leaf Protocol result.
See the [native temporal owner contract](docs/native-temporal-owner.md).

Private native compiler for `ix:native`, edition `0-draft`, profile
`state-finite/0-draft`. It parses and formats source with located diagnostics.
The library links exact imports and scoped names against supplied Contract IR
formal environments. Its native checker establishes contextual types and guarded
definedness through the existing IR prover under explicit runtime input obligations.

The native model admission API now checks explicit scalar, object and operation
roles against supplied IR declarations and exact source coordinates, then emits
a bounded deterministic model artifact. Its source-derived Rust fixture uses
original parsed JSON occurrences. `link_native` now selects those artifacts,
resolves explicit references and operations, and rejects conflicting model
inventory identities. `LinkedPackage::binding_profile`,
`LinkedModel::native_model` and `LinkedClause::operation` expose that
correspondence. Model admission and linkage are qualified under Task-008.

`model_source::read(formal_source, model_source::FORMAT, limits)` now exposes the
existing rule-model frontend to library callers. It derives located IR declarations
and native roles from `native-rule-model/1` source; `ModelDraft::admit` then runs
the existing model checks. Errors retain the original source and typed cause.
The [example model](tests/fixtures/native-rule-model.json) shows the admitted source
shape. Run `cargo test --test model_source` for fixed-artifact, failure and runtime
cases. Test helpers now call this production frontend.

Explicit `model_source::FORMAT_V2` selects `native-rule-model/2` and admits exact
rational domains through `native-state-model/2`, retaining the actual IR bounds,
units and source locations. Composed model exports consume these types; historical
linking refuses a selected `/2` model. This supplies the model prerequisite for
[composed value checking](spec/functional/FR-040-check-composed-values.md), whose
type admission and supported guarded proofs are available through the composed
checker. Complete runtime obligations remain open.

`checking::check` consumes a native linked package, exact `CheckBindings` and
caller-lowered `CheckLimits`. It returns the original source/AST with native
types, authored clause identities, discharged proof goals and required input
populations, observations and operation frames. Stable and lexical guard keys
preserve captured pre/post values; bounded proof expansion precedes actual IR
checks. The 24 checker tests qualify Task-009. The public library contract is in
[native-model-checking.md](docs/native-model-checking.md); review and handoff
are tracked in [Plan-005](plan/Plan-005-native-checking/index.md).

`runtime::Snapshot::new` and `runtime::Invocation::new` now construct immutable
inputs with exact bytes/digests, distinct reference types, structured errors
and caller-lowered byte/node/entry/depth limits. Flat arenas preserve duplicate
entries and shared values for later model-aware diagnosis. Twenty-one public
API tests and a role-separation compile-fail doctest cover construction; its
qualification is tracked in [Plan-006](plan/Plan-006-native-runtime/index.md).
See [native-runtime-inputs.md](docs/native-runtime-inputs.md) for the contract.

`runtime::validate` now checks exact artifact/clause/model bindings, supplied
typed values, finite reference closure, operation captures, population deltas
and immutable frames. Its constructor-private context retains the checked
package and immutable input; failed reports retain located defects and fresh
budget usage. Task-013 is qualified at 45ed1b4 by SR-097 with 35 public API tests.
See [the review](reviews/26-09-09-native-runtime-validation.md) for scope and evidence.

`runtime::evaluate` executes the original native AST over a borrowed validated
context. Reports retain concrete truth or an explicit incomplete/invariant-failure
outcome, exact reference costs and original implication events. Borrowed values
preserve pre/post captures, ordered sequence occurrences and object identities.
Twenty-nine public tests and two private invariant controls cover reference
execution, including all small functional graphs and independent budget vectors.
See [native-runtime-evaluation.md](docs/native-runtime-evaluation.md).

Five public pipeline tests now exercise healthy, violating, refused and incomplete
parent/aggregate cases, operation captures/frames and immutable retries through
actual model admission, parsing, linking, checking, construction, validation and
evaluation. SR-099 records their passing review, and Plan-006 records the
completed qualification/handoff with its final SR-100 gap audit. Backend projection
and Quire integration remain downstream work. Construction establishes structure and byte
correspondence; checking establishes static judgments conditional on valid input;
validation establishes input conditions before predicate execution.

`package::NativePackage::new` now exports a checked package's complete selected
models, authored clauses, resolutions, runtime obligations and projection
dispositions with separate byte and native static identities. Serde owns JSON
encoding, with independently measured derivation/canonical/output passes.
Task-016 is qualified at c195950 / SR-111, with fixed canonical/artifact vectors,
complete correspondence/capture controls, static dependency mutations and
runtime-independent identity checks. `NativePackage::read_verified` now verifies
selected bytes, closed JSON and external bindings, then repeats actual
parse/link/check before comparing every serialized claim. Reconstructed packages
execute the existing parent, aggregate and operation workflows, preserving
refusal, incomplete outcomes and fresh retry budgets. Qualification is tracked in
[Plan-007](plan/Plan-007-native-packages/index.md).
See [the package contract](docs/native-linked-packages.md).

`lowering::lower` derives complete Boolean executable projections through the
existing strict IR binder, retaining original native authority and explicit
model/read/observation correspondence. Object and graph expressions refuse.
`lowering::lower_for` with `ProjectionTarget::IntegerIrV1` additionally exports
bounded integer arithmetic and comparisons accepted by the strict IR binder.
Use `quire-spec lower <compile.json> --target integer-ir/v1` for this target;
the pinned codegen emits executable numeric Rust for obligation-free comparisons.
The `standalone_fixtures`
example emits `integer-healthy` and `integer-violating` requests for `amount < 7`.
Their native runs return true and false; both export the same integer projection.
`ProjectionTarget::StateScalarIrV1` additionally projects primitive self fields
and `pre` expressions. `NativeProjection::inputs` supplies their actual values
from a context validated against that exact checked package, retaining snapshot,
invocation and object provenance. For the concrete update example, run
`quire-spec lower <unchanged-version/compile.json> --target state-scalar-ir/v1`.
This binds `self.versionNumber = pre(self.versionNumber)` with separate pre/post
inputs; native population and frame validation still precedes materialization.
IT-010 runs the ConfigVersion state comparison through compiled generated Rust,
all four generated strategy populations and cargo-kani 0.67.0, and checks the
shared bounded corpus against `runtime::execute`. Values outside `0..=1000`
produce no Boolean oracle verdict, while object/graph expressions refuse at the
earliest source-owned boundary without a substitute IR expression.
The LC04 backend qualification uses pinned existing codegen and actual generated
Rust. Its named `boolean-oracle/v1` fixture compiles a generated proptest strategy,
checks every Boolean assignment against native evaluation and compares LLVM 3.1.0
source probes with native implication events. The pinned reusable coverage reader
still explicitly refuses LLVM 3.1.0; that downstream capability is not inferred
from the fixture-specific check. See [Plan-008](plan/Plan-008-native-lowering/plan.md).

CLI command and source identity/revision labels must be UTF-8; invalid encoding
returns usage exit 2. File operands remain OS paths. JSON paths are display text,
which may contain replacement characters; exact labels and the actual byte
digest carry the separate source correspondence. See the
[native diagnostic catalog](docs/native-error-codes.md) for stable outcomes.

Library callers can use `format::format_with_limit(unit, output_bytes)` to lower
the inclusive 1 MiB output-content ceiling. It counts every emitted byte,
including the final newline, and refuses before an append would exceed the
limit. Existing `format::format(unit)` retains the default ceiling. Allocator
capacity is not an exact content-byte accounting promise.

## Run and check

The [ConfigVersion example](examples/config-version/README.md) supplies the named
parent-order, cycle, identity and recorded-update workflow, with 13 native and
Markdown request sets and explicit refused/incomplete cases.

Rust 1.98.1 is pinned in rust-toolchain.toml. Cargo.lock pins dependencies. The
native CLI needs no Node or JVM. Default native commands need no optional features;
the Quire consumer is enabled explicitly with `quire-extraction`.
Use `--target-dir target` where a machine config points Cargo outside the checkout.

Backend parity additionally requires Rust 1.98.1's `llvm-tools-preview` component
and cargo-llvm-cov 0.9.0. Missing tools fail the test. Run the named LC04 parity
gate with `cargo test --locked --offline --target-dir target -j 1 --test
native_backend generated_proptest_and_old_profile_activation_match_reference --
--exact --test-threads=1`. The fixture verifies generated truth and activation
for `boolean-oracle/v1` while also requiring the pinned reusable reader to retain
its explicit LLVM 3.1.0 refusal.

On the shared desktop, run Cargo phases one at a time with `nice -n 10` and
`-j 1`, and run tests with `-- --test-threads=1`. Check for competing builds
before starting and reuse existing worktree target caches. Do not start another
phase until the previous command has completed.

```sh
cargo run --locked --target-dir target -- parse test:parent fixture:1 tests/fixtures/parent.native
cargo run --locked --target-dir target -- format test:parent fixture:1 tests/fixtures/parent.native
cargo fmt --all -- --check
cargo clippy --locked --target-dir target --all-targets --no-default-features -- -D warnings
cargo test --locked --target-dir target --no-default-features
cargo build --locked --no-default-features --target-dir target/clean
cargo run --locked --target-dir target --bin fixture-audit -- self-test
```

Run these checks locally while the repository stabilizes. Hosted CI exposes
only `workflow_dispatch` and has a ten-minute job timeout. Pushing commits or
opening a pull request does not request CI; hosted execution requires a separate
explicit dispatch.
Tests also need read access to the pinned private `agent-ix/ix-trace-rs` Git
dependency (or its exact cached revision). A future hosted runner needs that
access configured; local results do not qualify hosted credentials.

CLI arguments are `parse|format`, source identity, source revision, and file path.
`parse` emits JSON with status `parsed`; `format` writes source to stdout and
preserves comments/token spellings while normalizing whitespace to LF. Neither
command changes a file. Callers must assign the formatted document its appropriate
new revision before binding it into evidence. Diagnostics contain original byte,
line and Unicode scalar column positions. Exit 0 is success, 1 refusal, 2 usage/I/O
failure and 3 resource exhaustion (incomplete). These are local syntax outcomes,
not the portable verification result envelope.

The library returns immutable ParsedUnit syntax or a boxed Diagnostic. Source
identity/revision are opaque diagnostic labels; they do not implement the future
interchange source reference or numeric IR revision mapping. Source::read_verified
checks a selected byte digest; parse_source retains that verified immutable source. Source byte spans are half-open; located lines/columns are one-based.

Default ceilings are 1 MiB source, 100,000 tokens, 50,000 syntax nodes and 64
nested delimiters/parser frames. Callers can lower these ceilings. Formatting
also bounds output to 1 MiB and can return incomplete if expansion exceeds it.
Long flat expressions use an arena; grouping and all original source bytes are
retained. Balanced reserved forms refuse as unsupported; malformed tokens and
delimiters are syntax errors. No recovery manufactures a successful partial unit.

The [source correspondence API](docs/source-correspondence.md) checks exact
body-to-original segments with explicit layout transformations and returns
original document locations. Maps preserve all corresponding byte regions and
refuse changed bytes, foreign source bindings, malformed segments and budget
exhaustion. Wire decoding and existing-repository extraction remain separate.

`mapped::compile` carries a verified map and one authored `ClauseBinding` through
the existing parse/link/check/package stages. Its immutable `MappedPackage`
exposes the native package for validation, evaluation and lowering while keeping
the original document available for source locations. Failures preserve the
native diagnostic or package path; the API does not manufacture source wrappers
or change extraction availability. Run `cargo test --test mapped` for the mapped
parent workflow and stage refusals.

With `quire-extraction`, `quire_source::compile(original, context, selection,
models, limits)` calls Quire's actual pinned Rust extractor and compiles the
selected native body. Supply a digest-verified original Source, a loaded Quire
SemanticContext and explicit authored/body/formal identities. The consumer checks
source coordinates and exact bytes, including Quire's omitted final LF on CRLF
input, before using the existing source map. It retains Quire's original clause
population, availability and diagnostics on both success and refusal. Input is
bounded to 1 MiB and 4096 lines before extraction. Actual healthy/violating/refused
cases run with `cargo test --features quire-extraction --test quire_source`;
[the tests](tests/quire_source.rs) show context and identity construction.
C's existing-repository CLI/wire adoption remains separate from this working
compiler-side Rust integration.

The standalone `run` command can select Markdown with `--features quire-extraction`.
Its optional `program.extraction.body` record supplies `identity`, `revision`,
`document` and `formal_revision` for the derived native body; `program.source`
selects the original Markdown bytes and exactly one clause binding selects its
heading. This uses a validated clause-only Quire context for the selected package,
with no installed archetype schemas. The result's `extraction` member retains
original identity, producer observations and verified byte-map segments; ordinary
source, diagnostic and event coordinates refer to the native body.

Generate runnable Markdown fixtures with the existing `standalone_fixtures`
example, then run `cargo run --locked --features quire-extraction -- run
/tmp/native-workflow/markdown-healthy/request.json`. The sibling
`markdown-violating` and `markdown-refused` requests reproduce false and frame
refusal. These requests support source compilation through `run`; combining them
with selected-package execution or `compile`/`lower` export refuses explicitly.
Default builds reject the extraction field; ordinary native requests are unchanged.

`runtime::execute(package, input, selection, limits, poll)` now runs native
validation and evaluation in one call. The returned report retains the exact
package and offered artifacts even on validation failure; `truth()` returns a
Boolean only after completed evaluation. `outcome()` preserves the original
stage diagnostics, measured work and implication events. The existing separate
`validate` and `evaluate` APIs remain available. Run
`cargo test --test runtime_execution` for aggregate, operation and stopped/retried
requests. The standalone `quire-spec run <request-file>` command now reads selected
model/program/runtime files and calls these same APIs. See
[the runnable workflow](docs/native-standalone.md) for healthy, violating,
operation and refused examples, JSON results and exit codes.

[Matrix status checks](docs/matrix-status.md) records the exact CLI/module stack
that reads both authored status columns, its false-completion controls and the
remaining installed-tool limitation. Coverage binding does not execute tests.

`Snapshot::read_verified` and `Invocation::read_verified` now read selected
`native-state-input/1` bytes through closed Serde decoding and the existing
structural constructors. They preserve original bytes/digests and refuse stale
selections, incompatible envelopes, malformed fields and exhausted budgets.
Run `cargo test --test runtime_reading` for round trips and actual execution of
reread inputs. Model-aware validation still occurs during `runtime::execute`.

The [formal source bridge](docs/formal-source-binding.md) retains exact native
source under an explicitly supplied Contract IR source identity. Its forward
and reverse mappings check byte, line and scalar-column correspondence,
including constructor-valid IR spans whose coordinates disagree with the bytes.
Native revision labels remain opaque. This API supplies location correspondence
for later checking; the caller owns assigning the formal identity.

## Design and status

The [requirements index](spec/spec.md) covers LC01–LC05 through discrete Quoin
catalog artifacts. [Authoring status](docs/spec-workflow.md) records the exact
skills, validation results and outstanding review work. These requirements are
drafts; the completed native readiness reviews and local results are recorded
in the authoring status. The owner adopted specification PR8 at
`e897f810a7356d4ce8fd19026221ebda7b65596f` for internal LC02 implementation.
[LC02's test matrix](spec/model-linking/tests.md) records qualified linking and
planned type checking. The new
[formal linker API](docs/formal-environment-linking.md) owns the exact parsed
source and borrows immutable models; CLI parse/format still perform syntax work
only. Accepted Contract IR ADR-0054 at
`690bde7f2dc58662cf9ff0595c2c0e3b17107c6f` closes #54 and removes the former
Filament-reader prerequisite: generic compilation uses the existing public
DeclarationEnvironment/check_expression and executable binder APIs. A concrete
archetype projection is specified only for a clause that needs its semantics.

See [architecture review](docs/architecture-review.md) for concrete fixes and
remaining linking/identity work. The current parser uses declarative Logos tokens
and a Pratt expression parser; JSON escapes use serde_json. Binding/lowering will
remain modules and use the existing model/IR authorities. No second domain model,
backend binder or Markdown expression parser is created here.

New implementation and newly authored implementation fixtures use
[AGPL-3.0-or-later](LICENSE). [Dependency inventory](docs/dependencies.md) preserves
third-party grants. [License decision](LICENSE-DECISION.md) records owner approval
and deferred standard-artifact terms. No public release is authorized.

Private tracking: [LC01](https://github.com/agent-ix/quire-spec-language/issues/2),
[compiler epic](https://github.com/agent-ix/quire-spec-language/issues/1).
The shared contracts and LC02–LC05 acceptance remain separate work.

## Review fixture audits

The Rust `fixture-audit` binary checks optional private state-core review,
role and rule-syntax packets under
[Agent A #58](https://github.com/agent-ix/quire-research/issues/58).

```sh
cargo run --locked --target-dir target --bin fixture-audit -- review /path/to/specification/proposals/state-core/fixtures
cargo run --locked --target-dir target --bin fixture-audit -- roles /path/to/specification/proposals/state-core/fixtures
cargo run --locked --target-dir target --bin fixture-audit -- rule-syntax /path/to/specification/proposals/state-core
QUIRE_STATE_CORE=/path/to/specification/proposals/state-core cargo test --locked --target-dir target --test fixture_audit -- --ignored
```

Review checks 23 artifact files, seven invocation cases and six independent
negative controls. The second command checks the optional private FS02 packet's exact
artifacts and source regions. It is bookkeeping, not an independent
semantic-reference matcher. These optional checks run separately from the native
runtime.

Rule-syntax wraps the FS03 rule examples in native source units and checks
their syntax with the existing parser library. It checks the selected rule/profile digests,
then expects 50 parsed expressions and one explicit unsupported refusal. It
does not interpret the abstract type environments or execute their 51 authored
typing/evaluation expectations. The owner adopted the separately digest-bound
FS03 refinements for internal implementation; that decision does not rewrite
the original profile bytes or establish executed typing/evaluation evidence.

The final command runs the three explicitly selected private-packet integration
tests, including independent corruptions. The normal Rust test suite needs no
private sibling repository. Audit intake bounds files to 8 MiB, aggregate reads
to 64 MiB, files/decoded values to 10,000, and JSON nesting to 64 values; it refuses
escaped duplicate keys, invalid fields, foreign paths and exhausted budgets.
Fixture trees must stay immutable during a run. See the
[audit error catalog](docs/audit-error-codes.md) and
[remediation evidence](docs/rust-verification-remediation.md).

New tests use the shared `ix_trace_rs::trace` macro with canonical attributes
such as `#[trace("TC-006", "FR-012-AC-6")]`. Quire's declared grammar binds the
IDs; the macro checks argument shape. `#[cfg(test)]` controls compilation.
Read [license decision](LICENSE-DECISION.md) before adding code or normative artifacts. Publication requires a fresh review; do not flip private history public.

## Implementation language — owner directive

Executable implementation, including production and qualification tools/tests, is Rust. New TypeScript choices require explicit owner approval; surface all other executable languages. Quoin remains in its current implementation for now: contain its spread through versioned structured interfaces, do not rewrite it wholesale. Filament changes are excluded. See AGENTS.md and the language gate on the owning issue. Normative prose/schema need not be executable Rust.
