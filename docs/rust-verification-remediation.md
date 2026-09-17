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
| tools/check_rule_syntax.py | Domain-specific rule wrapper/case metadata; actual parsing is an existing shared native capability. | Rust rule-syntax calls the existing parser directly. No expression parser, temp source runner or evaluator duplicate. |
| CI self-test step | Owned executable verification logic previously invoked Python. | Replaced with the Rust audit executable; explicit count/claim boundaries retained. Hosted events are manual-dispatch only. |
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
unsupported syntax, malformed/missing/wrongly typed JSON, escaped
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
inventory records the actual resolved closure; existing grants are preserved.

## Current status

The Python helpers are removed and the CI audit command invokes Rust. Model
intake comes from domain packages (FR-056), so no model fixture audit exists. Six audit unit tests and four default command/file integration tests pass;
the 21 preexisting compiler tests also pass. All three explicitly selected private
packet tests passed with independent corruptions. The six audit unit tests pass
in the optimized release profile as well. Formatter, strict all-feature Clippy
and a separate-target locked build pass locally.

The selected standard packet revision is
36293bae7f5bcb7ca3b2389ed166e525dc9dba87. Review reports 23 files/seven cases/six
controls; roles reports 17 artifacts/four regions; rule-syntax reports 50 parsed/one unsupported.
Integration tests compare all original fixture bytes before/after execution;
neither repository's historical fixtures changed.

Quire resolves all 13 trace attributes on the new Rust tests:
FR-012 is 9/9 backed, TC-001–TC-003 and TC-005–TC-009 are backed, and TC-010 is a declared Manual
inspection with no source symbol. This proves binding, not behavioral coverage
percentages. Old compiler trace debt remains outside Plan-001.

### TC-010 inspection and local CI policy

Inspected every owned file under tools/, tests/, src/, Cargo manifests and the
single .github/workflows/ci.yml. There are zero remaining Python executable
helpers or CI invocations and no embedded Node/shell verifier. The audit contains
no child-process invocation. Test harness processes invoke only the real Rust
binary; its environment has an empty PATH. Four named historical paths above
are an inventory of removed files.

The owner requested local CI until stable. NFR-002/IT-004 amendments at 1649ef7
received all eight review addenda at 2e5cd9a before the workflow change at
8cf5571. The compiler workflow has only workflow_dispatch; the specification
repository has no workflows. No hosted run was dispatched or used to qualify
this remediation. The shared ix-trace-rs dependency is private: a future hosted
runner needs access to that repository before the test/lint commands can run.
Local locked builds have that access/cache; hosted credential setup is not
claimed by local results.

This scope does not accept
FS02/FS03/FS05 or complete the full compiler workflow. The root code and gap
reviews record the actual evaluated implementation revision and limitations.
