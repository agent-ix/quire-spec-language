# Complete-value normative inputs

The files below `quire-specification/` are unmodified bytes of
[agent-ix/quire-specification](https://github.com/agent-ix/quire-specification)
at merged revision `d227270fbeb28289df6abba7e94173118345c028` (QSpec PR #86,
which renamed TC-196 R01's cycle listing to name types rather than
generalization records and added R07's second contending-redefiner shape),
copied with `git show <revision>:<path>` at their repository-relative paths so
the lock's relative `artifact_path` values resolve unchanged. QSL #118 /
Plan-013 Task-048 consumes them; QSL #119 / Plan-013 Task-049 adds the
FR-143, FR-144, FR-145, FR-146 and FR-307 rules and the TC-188, TC-189,
TC-190, TC-191, TC-194 and TC-227 procedures, and consumes FR-149.

`complete-value-lock.json` is the authority for every qualification-catalog
artifact digest; `tests/complete_value_lock.rs` recomputes each vendored
artifact's raw-byte SHA-256 against that lock. The lock itself, the selection
vectors, the checked-package-v2 node-identity vectors and preimage schema, the
`positive-nominal-identities` CheckedPackage V2 fixture whose identity preimage
FR-307 resolution reads, and
the FR-143/FR-144/FR-145/FR-146/FR-307 rules and the
TC-185/TC-186/TC-187/TC-188/TC-189/TC-190/TC-191/TC-192/TC-193/TC-194/TC-227
procedures are not in the catalog, so the same
test pins their SHA-256 values as read from the pinned revision.

FR-142 dimensions, units and quantities (TC-187) read the checked-package-v2
dimension/unit node vectors and schema, and the `quire.value.compound-unit/v1`
definition, schema and vectors. The compound-unit files are catalog artifacts,
so their digests come from the lock. TC-233 is not vendored: it qualifies the
published compound-unit artifacts themselves rather than this implementation.

`unicode-17.0.0/` holds the six Unicode-hosted artifacts that
`value-text-unicode-17.md` names, downloaded unmodified from their
authoritative `unicode.org` URLs (UAX #15 revision 57, the four Unicode 17.0.0
UCD files and `license.txt`). `tests/unicode_17_text.rs` reads each SHA-256 from
that definition's table and checks the vendored bytes against it, then checks
the `unicode-normalization` tables against these UCD files and runs the complete
`NormalizationTest.txt` corpus. Their redistribution terms are the Unicode
license in `unicode-17.0.0/license.txt`; `.gitattributes` keeps their bytes
exactly as downloaded.

The standard's document and reusable-artifact license remains deferred under
its [license decision](https://github.com/agent-ix/quire-specification/blob/main/LICENSE-DECISION.md).
This private snapshot applies no new license to those documents, and the
compiler's AGPL license does not relicense them. A changed interpretation
requires re-vendoring from a newer merged revision, never editing these bytes.

## Re-vendoring

[`VENDOR.json`](VENDOR.json) is the pin for this directory: the `quire-specification`
files above and their commit (currently `d227270fbeb28289df6abba7e94173118345c028`),
and the six `unicode-17.0.0/` files, each pinned by origin URL and digest since
they have no git revision to read from. `cargo xtask revendor --tree
complete-value --qspec-clone <path to a local quire-specification checkout>`
(`make revendor TREE=complete-value QSPEC_CLONE=<path>`) reads every listed
`quire-specification` path with `git show <commit>:<path>` against that clone
and writes exactly those bytes; it never resolves "latest" and never fetches
the Unicode files over the network, so it only ever verifies their recorded
digest against what is already here. `cargo xtask revendor-check`
(`make revendor-check`, and `cargo test --workspace` via
`xtask/tests/revendor.rs`) verifies every vendored byte against `VENDOR.json`
offline and flags any file present here that `VENDOR.json` does not mention.
A new pin replaces the vendored bytes wholesale via `revendor`, never by hand:
edit the affected source's `commit` (and its path list, for a `unicode-17.0.0/`
file its `url`/`sha256`) in `VENDOR.json`, then re-run `revendor`; it removes
any file the new pin no longer lists rather than leaving it behind as a stray
file.

FR-148 IEEE profiles (TC-193) read the `quire.value.ieee754-2019-default/v1`
definition, the FR-148 rule and the TC-193 procedure; `tests/ieee_profiles.rs`
reads the procedure's vector ids.
