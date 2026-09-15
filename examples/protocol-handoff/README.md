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

Build and run the Rust example with a new output directory:

```console
CARGO_PROFILE_RELEASE_STRIP=symbols cargo run --locked --offline --release --example native_protocol_handoff -- /tmp/quire-native-handoff
```

Run the named producer test with its actual stripped release test executable:

```console
CARGO_PROFILE_RELEASE_STRIP=symbols cargo test --locked --offline --release --no-default-features --example native_protocol_handoff stripped_release_producer_keeps_original_owners_and_compensations -- --ignored --test-threads=1
```

This test uses a fresh temporary output directory and checks original source and
declaration owners, joined receive/choice provenance and Full/Partial compensation records after the producer's
independent reader succeeds. It is ignored in ordinary test runs because the
executable must fit the producer's ELF binary limit. It does not exercise B's
acceptance interface.

This recipe supports Linux ELF executables; Mach-O and PE executables are refused.
The producer reads its actual `current_exe()` bytes, identifies an ELF version-1
binary, and selects their raw digest. This example lowers binary input to 16 MiB
within the artifact byte-work ceiling; larger binaries fail before output.
An unstripped debug executable will commonly exceed that bound. All compiler
stages retain their own default limits; no limit is disabled to accommodate a
build. Existing output directories are refused. Publication is not atomic: an
I/O failure may leave a partial directory. Retry with a new path, or inspect and
remove the incomplete output before reusing its path. No source file or synthetic
producer string stands in for the executable bytes.

The output directory contains:

- `compiled-protocol.json`: exact canonical bytes from native emission.
- `compiled-protocol.ref.json`: the existing `ix.artifact-ref/3-draft` external seal.
- `expected.json`: this example's independent selections, using the existing
  wire records and `Expected` fields, plus relative dependency/model filenames.
- `predicates.native`, `state.native`, `temporal.native`, `workflow.native` and
  `model-source.json`: the complete original sources.
- `dependencies/`: exact selected model, binary, contract, definition and rule
  bytes; `expected.json` maps each reference to its file and direct prerequisites.
- `SHA256SUMS`: a deterministic complete inventory of every other generated
  handoff member, using normalized handoff-relative paths.

The immutable checked-in consumer copy is addressed by
`protocol_artifact::handoff::PUBLISHED_V1_HANDOFF`; its version-explicit member
constants prevent consumers from guessing filenames. Selecting the pinned crate
and this owner-published directory is separate from offering the package bytes:
decoding `Selection` remains inert until the caller constructs and invokes the
strict public reader.

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

New example source uses AGPL-3.0-only. Embedded standard documents retain their
[original provenance and deferred licensing](../../resources/native-v1/README.md);
the compiler license does not relicense them or authorize public distribution.
