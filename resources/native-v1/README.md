# Selected native definition resources

These original bytes come from the native definitions and selected rules in
[quire-specification PR #15](https://github.com/agent-ix/quire-specification/pull/15),
at its merge commit `4d6230eb8aa9766ff3017360962f2d6368d74cb3`. Standard-relative
paths and document IDs retain that repository's meaning; compiler requirement
tooling scans the local `spec/` tree separately.
[FR-036](../../spec/functional/FR-036-link-composed-native-packages.md) owns their
use in exact definition and rule closure.

The standard's document and reusable-artifact license remains deferred under its
[license decision](https://github.com/agent-ix/quire-specification/blob/main/LICENSE-DECISION.md).
This private snapshot applies no new license to those documents. The compiler's
AGPL license does not relicense the standard or authorize its public release.

`external/quire-spec-language/` holds historical first-party compiler sources
explicitly selected by the [diagnostic definition](proposals/quire-v1/definitions/native-diagnostics.md),
at this repository's own commit `bf9960e8d818e4d72182c4691c8aaf4e2cd7dffd`.
Here `external` means external to the standard repository. Original notices
remain intact, including the Rust source's AGPL-3.0-only notice; copying does
not add a license grant to the accompanying documentation.

## Re-vendoring

[`VENDOR.json`](VENDOR.json) is the pin: it records, per source, the exact
commit and the exact path list vendored from it (or, for an externally hosted
file, its origin URL and digest). `cargo xtask revendor --tree native-v1
--qspec-clone <path to a local quire-specification checkout>` (wrapped by
`make revendor TREE=native-v1 QSPEC_CLONE=<path>`) reads every listed path
with `git show <commit>:<path>` against that clone (or, for the
`external/quire-spec-language` selection, against this repository's own
history) and writes exactly those bytes; it never resolves "latest", never
reads a working tree, and never adds a path by scanning a source tree.
`cargo xtask revendor-check` (`make revendor-check`, and `cargo test
--workspace` via `xtask/tests/revendor.rs`) verifies the vendored bytes
against `VENDOR.json` offline and flags any file present here that
`VENDOR.json` does not mention.

To move a pin, edit the affected source's `commit` (and its path list) in
`VENDOR.json` by hand, then re-run `revendor`; it replaces the tree
wholesale, removing any file the new pin no longer lists rather than leaving
it behind. Reading the `external/quire-spec-language` selection needs this
repository's own history for that commit, so a shallow clone (for example
`git clone --depth 1`, or CI's default `actions/checkout@v4` checkout) must
first be made non-shallow (`git fetch --unshallow`, or an explicit `fetch-depth: 0`)
before running `revendor` for `native-v1` against it.

The [closed registry](../../src/linking/composed/definition_source.rs) interprets
this selected snapshot. Supplied content must match its embedded original bytes;
the registry tests compare those bytes with the retained resource files.
Updating current compiler diagnostics does not update this historical selection.
A changed interpretation requires an explicit registry/definition change, not
synchronizing these resources with current source files. A new pin replaces
these bytes wholesale via `revendor`, never by hand-editing them.
