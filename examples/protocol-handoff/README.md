# Native protocol producer example

`native_protocol_handoff` compiles the authored [predicate](predicates.body.native),
[state](state.body.native), [temporal](temporal.body.native) and
[workflow](workflow.body.native) source units and the [located rule model](model.json)
through the public model frontend, namespace, binding, type, definedness and
family-admission APIs. It emits through
`protocol_artifact::native::emit` and checks the unchanged bytes with
`protocol_artifact::read` against selections derived from the original inputs.
The output seal is selected by the producer after emission; this local reader
check does not independently authenticate that seal or exercise tamper controls.
The units retain cross-unit calls, bounded `sum`/`size` queries, exact rational types and
object/reference population requirements. The `Wide` and `Exact` scalars declare the
full signed-64 integer domain and the exact-rational domain at the denominator
ceiling that `ir::RationalType::new` admits, and the `Bounded` predicate types and
literalises `-9223372036854775808`, `9223372036854775807` and
`rational(9223372036854775807, 9223372036854775806)` and
`rational(-9223372036854775808, 9223372036854775807)` so a consumer sees those
endpoints as emitted `M::Wide`/`M::Exact` values rather than inferring them.
The second rational exercises the actual denominator ceiling `i64::MAX`, which
the pinned Contract IR `RationalType::new` accepts; the first exercises the
maximum numerator without reduction collapsing the pair. The negative pair
also exercises the minimum numerator. No declared domain or compiler limit is
weakened to admit these values.
Adding those two scalars renumbers the name-ordered model export table; consumers
must re-derive every `"export"` index rather than reuse a frozen one. Two `Notice` records received by
`Service` on the `Decisions` channel become available after the explicit
all-branch join. `Outcome` selects complementary Boolean guards over their
distinct `ready` facts, retaining both authored event branches and original
receive anchors. The producer proves the conservative Boolean partition; it
does not supply concrete messages or choose a runtime branch.
The actual `Workflow::apply` operation
and its pre/post contracts retain their model and source owners. Both `Full` and
`Partial` compensation obligations pair with `Main::Applied`: `Full` requires the
receipt sum to equal its captured target and names `Main::Committed`; `Partial`
requires a positive sum below its captured target and selects commit `never`.
Their registration, activation, retry anchors and recovery predicates remain
distinct static requirements; the example supplies no runtime
observations, population-completeness claims or recovery results.

The producer is the library API
`quire_spec_language::protocol_artifact::handoff::{write_v1, write_v2}`, behind
the `handoff-writer` feature (source:
[`src/protocol_artifact/handoff/writer.rs`](../../src/protocol_artifact/handoff/writer.rs)).
It embeds this directory's recipe inputs at compile time, so a downstream crate
calls it in-process -- as a git dependency with
`features = ["handoff-writer"]` -- without building these examples or this
crate's dev-dependencies. The two examples are thin command-line callers of it.

Build and run the Rust example with a new output directory:

```console
CARGO_PROFILE_RELEASE_STRIP=symbols cargo run --locked --offline --release --features handoff-writer --example native_protocol_handoff -- /tmp/quire-native-handoff
```

Run the named producer test:

```console
cargo test --locked --offline --features handoff-writer --example native_protocol_handoff stripped_release_producer_keeps_original_owners_and_compensations
```

This test uses a fresh temporary output directory and checks original source and
declaration owners, joined receive/choice provenance and Full/Partial compensation records after the producer's
independent reader succeeds. It does not exercise B's acceptance interface.

The producer identifies itself by a digest over its own source text
(`src/protocol_artifact/handoff/writer.rs`, embedded at compile time via
`include_bytes!`) and records only that digest as `Producer.binary` -- the
bytes are never retained, written to a fixture file, or supplied as a
dependency's exact-byte content, so no size ceiling applies to them, and the
value is the same across a debug or release build, stripped or not, and
across any toolchain: anyone with this repository can independently
recompute it (`sha256sum src/protocol_artifact/handoff/writer.rs`). All compiler
stages retain their own default limits; no limit is disabled to accommodate a
build. Existing
output directories are refused. Publication is not atomic: an I/O failure may
leave a partial directory. Retry with a new path, or inspect and remove the
incomplete output before reusing its path.

The output directory contains:

- `compiled-protocol.json`: exact canonical bytes from native emission.
- `compiled-protocol.ref.json`: the existing `ix.artifact-ref/3-draft` external seal.
- `expected.json`: this example's independent selections, using the existing
  wire records and `Expected` fields, plus relative dependency/model filenames.
- `predicates.native`, `state.native`, `temporal.native`, `workflow.native` and
  `model-source.json`: the complete original sources.
- `dependencies/`: exact selected model, contract, definition and rule bytes;
  `expected.json` maps each reference to its file and direct prerequisites. The
  producer's own binary is not among them: `expected.json`'s `producer.binary`
  records its identity as a digest.
- `SHA256SUMS`: a deterministic complete inventory of every other generated
  handoff member, using normalized handoff-relative paths.

The immutable checked-in consumer copy is addressed by
`protocol_artifact::handoff::PUBLISHED_V1_HANDOFF`; its version-explicit member
constants prevent consumers from guessing filenames. It omits `dependencies/`:
this repository does not commit exact-byte dependency fixtures, so the checked-in
`expected.json`/`expected-v2.json` selections carry an empty `dependencies` list.
A fresh run of the producer still writes its own `dependencies/` directory to
its (uncommitted) output path, exactly as described above. The checked-in
copies are therefore not admissible, and their `Producer.binary` digest is
informational (the producer source when they were taken); a consumer that must
admit a handoff calls `write_v1`/`write_v2` into its own directory. Selecting the pinned
crate and this owner-published directory is separate from offering the package bytes:
decoding `Selection` remains inert until the caller constructs and invokes the
strict public reader.

The version-1 `Flow` uses distinct admitted object authorities for its `Service`
and `Provider` roles; only `Service` owns `Workflow::apply`. The version-2
handoff predates this consumer-corpus correction, so its producer recipe reads
the historical model and workflow inputs from `model-v2-frozen.json` and
`workflow-v2-frozen.body.native` rather than the corrected `/1` inputs.

The baseline is the accepted registered Edition artifact, identity `ix:native`,
semantic revision `1-draft.2`, from [standard PR #15](https://github.com/agent-ix/quire-specification/pull/15).
The registry supplies its exact bytes and the selected definition/rule closure;
the source headers derive their imports from those selections. Semantic revision
namespaces stay separate from opaque source-artifact revisions. The contract
selection is the actual [compiled protocol contract](../../docs/compiled-protocol-v1.md)
embedded in this executable. These are product inputs, not C-owned evidence
acceptance or proof of an authenticated/reproducible binary build.

For consumption, B constructs its accepted inventory as Rust values from
independently selected original inputs and referenced bytes before examining an
offered payload. Neither that payload nor `expected.json` authorizes its own
selectors. The sidecar is an inspection aid confined to this example recipe;
it introduces no production request or manifest format. Its model source record
retains original native identity/path, formal document/revision and bytes so a
Rust caller can reconstruct the model with `Source::read`, `FormalSource::new`,
`model_source::read(..., model_source::FORMAT_V2, ...)` and `ModelDraft::admit`,
then check the resulting artifact against the selected model reference. No
alternate model reader is needed.

This example exercises A's producer and reader. FR-042-AC-10 and B's IT-001 stay
incomplete until B's real public Rust admission/linking interface consumes these
unchanged bytes and retains the selectors. No B implementation is supplied here.
General dynamic choice/progress proofs, first-class D relationship/related-instance
exports and runtime recovery remain open as specified in FR-042/TC-121. Ordinary
object/reference exports do not supply those relationship authorities.

New example source uses AGPL-3.0-only. No third-party standard-document bytes
are embedded here: the registered-definition registry
(`linking::composed::definition_source`) resolves each definition to a
synthetic placeholder derived from its own identity string, never real
document content, and this example commits no `dependencies/` fixture files.
