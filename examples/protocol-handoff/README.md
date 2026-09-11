# Native protocol producer example

`native_protocol_handoff` compiles the authored [native body](workflow.body.native)
and [located rule model](model.json) through the public model frontend, namespace,
binding, type, definedness and family-admission APIs. It emits through
`protocol_artifact::native::emit` and checks the unchanged bytes with
`protocol_artifact::read` against selections derived from the original inputs.
The output seal is selected by the producer after emission; this local reader
check does not independently authenticate that seal or exercise tamper controls.
The example includes an exact rational domain, object/reference population
requirements, and predicate, state, temporal and protocol declarations. It
supplies no runtime observations or population-completeness claims.

Build and run the Rust example with a new output directory:

```console
CARGO_PROFILE_RELEASE_STRIP=symbols cargo run --locked --offline --release --example native_protocol_handoff -- /tmp/quire-native-handoff
```

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
- `workflow.native` and `model-source.json`: the complete original sources.
- `dependencies/`: exact selected model, binary, contract, definition and rule
  bytes; `expected.json` maps each reference to its file and direct prerequisites.

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
The broader recovery, decision-proof and producer-authority obligations remain
as specified in FR-042/TC-121.

New example source uses AGPL-3.0-only. Embedded standard documents retain their
[original provenance and deferred licensing](../../resources/native-v1/README.md);
the compiler license does not relicense them or authorize public distribution.
