# Rust verification remediation

Tracking: [Agent A #58 / LR02](https://github.com/agent-ix/quire-research/issues/58),
coordinated with [LC01](https://github.com/agent-ix/quire-spec-language/issues/2)
on the existing private draft PR7 branch. The specification is FR-012/NFR-005
and IT-004 at 11a9128. The scoped full review is under
[spec/reviews/rust-verification](../spec/reviews/rust-verification/base.md), and
[Plan-001](../plan/Plan-001-rust-fixture-audits/plan.md) records implementation.

## Executable-path and capability inventory

| Existing path | Classification before porting | Rust remediation / disposition |
| --- | --- | --- |
| tools/audit_review_fixtures.py | Domain-specific historical snapshot/frame/invocation wrapper checks and negative controls. Hashing is already shared. | Rust review/self-test modes; reuse native ByteDigest. Preserve historical tuple fields and exact bytes. |
| tools/audit_role_compositions.py | Domain-specific packet/source/model/run correspondence. Source correspondence and byte hashing already exist as native primitives. | Rust roles mode; share checked file/JSON intake, use original-byte hashing and native Source coordinates; no new evidence matcher. |
| tools/check_model_fixture.py --bytes-only | Domain-specific pinned fixture manifest and byte checks. | Rust model-bytes mode with the same five historical digests and producer pin. |
| tools/check_model_fixture.py producer mode | Rust can own orchestration, but the consumed TypeSpec/Node producer is a separate executable-language dependency. | Fresh execution remains unapproved; Rust model-producer refuses. Preserve original production/stale-lock evidence and producer pin. No Filament edits. |
| tools/check_rule_syntax.py | Domain-specific rule wrapper/case metadata; actual parsing is an existing shared native capability. | Rust rule-syntax calls the existing parser directly. No expression parser, temp source runner or evaluator duplicate. |
| CI self-test and model integrity steps | Owned executable verification logic currently invokes Python. | Replace with the Rust audit executable; keep explicit count/claim boundaries. |
| Cargo, Rust toolchain, GitHub runner/checkout action | Existing external build/CI tooling and host; checkout@v4 is not Rust-owned verification logic. | Inventory retained; this work grants no new host/tool language exemption or broader qualification. No manual CI dispatch. |
| Quoin/Quire specification and evidence workflows | Existing shared requirement/trace/method/evidence ownership. | Reuse installed workflows. Campaign explicitly retains Quoin for now; no local evidence-store framework or Quoin rewrite. |

The existing contract-IR exposes canonical/binding identities and a conformance
hash helper; neither is the schema/authority of these historical review wrappers.
Quire owns clause extraction/coverage/evidence relationships, not this fixture
packet's assertions. Inspection of the existing sources found no directly
reusable public implementation of these four complete audits. Native ByteDigest,
Source coordinates and parser APIs are the applicable shared capabilities.

Strict duplicate-key JSON intake is a reusable boundary concern, but this
change uses a private Serde visitor inside the audit target and does not publish
a competing shared wire contract. Any future reusable production reader belongs
to the separately reviewed shared interchange work. Artifact, model and portable
result authorities remain unchanged.

## Verification plan

Ten TC artifacts and a scoped matrix precede implementation. They cover the
real audit modes, stale/recomputed digest controls, selected source regions,
producer pin, unsupported syntax, malformed/missing/wrongly typed JSON, escaped
duplicate keys, path/symlink escape, OS argument encoding, and resource ceilings.
New Rust test functions carry resolving TC/AC tags. No Python verifier is kept
as a parity oracle; original fixture bytes and independently authored adverse
cases supply the checks.

Serde's [visitor interface](https://serde.rs/deserialize-map.html) permits
checking each decoded map entry. [thiserror](https://docs.rs/thiserror/2.0.20/thiserror/)
supplies standard Rust error traits, and
[tempfile](https://docs.rs/tempfile/3.27.0/tempfile/) isolates test copies.
Versions and MIT OR Apache-2.0 grants were inspected in the selected local
package manifests before adoption. The Cargo lock and included-artifact
inventory will record the actual resolved closure; existing grants are preserved.

## Current status

Specification and scoped review are authored; runtime migration and execution
evidence remain pending. Historical successful producer runs remain labeled
historical. Nothing here approves new TypeScript/Node execution, changes
Filament, accepts FS02/FS03/FS05, or completes the full compiler workflow.
