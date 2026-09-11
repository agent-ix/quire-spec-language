# Selected native definition resources

These original bytes come from the native definitions and selected rules in
[quire-specification PR #15](https://github.com/agent-ix/quire-specification/pull/15).
Standard-relative paths and document IDs retain that repository's meaning;
compiler requirement tooling scans the local `spec/` tree separately.
[FR-036](../../spec/functional/FR-036-link-composed-native-packages.md) owns their
use in exact definition and rule closure.

The standard's document and reusable-artifact license remains deferred under its
[license decision](https://github.com/agent-ix/quire-specification/blob/main/LICENSE-DECISION.md).
This private snapshot applies no new license to those documents. The compiler's
AGPL license does not relicense the standard or authorize its public release.

`external/quire-spec-language/` holds historical first-party compiler sources
explicitly selected by the [diagnostic definition](proposals/quire-v1/definitions/native-diagnostics.md).
Here `external` means external to the standard repository. Original notices
remain intact, including the Rust source's AGPL-3.0-only notice; copying does
not add a license grant to the accompanying documentation.

The [closed registry](../../src/linking/composed/definition_source.rs) interprets
this selected snapshot. Supplied content must match its embedded original bytes;
the registry tests compare those bytes with the retained resource files.
Updating current compiler diagnostics does not update this historical selection.
A changed interpretation requires an explicit registry/definition change, not
synchronizing these resources with current source files. Git and the source PR
retain provenance; there is no separate per-file checksum inventory.
