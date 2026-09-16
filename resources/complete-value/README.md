# Complete-value normative inputs

The files below `quire-specification/` are unmodified bytes of
[agent-ix/quire-specification](https://github.com/agent-ix/quire-specification)
at merged revision `4780a9e6119bb86ffb1322fe0a141ef3905b11ef` (QSpec #68),
copied with `git show <revision>:<path>` at their repository-relative paths so
the lock's relative `artifact_path` values resolve unchanged. QSL #118 /
Plan-013 Task-048 consumes them.

`complete-value-lock.json` is the authority for every qualification-catalog
artifact digest; `tests/complete_value_lock.rs` recomputes each vendored
artifact's raw-byte SHA-256 against that lock. The lock itself, the selection
vectors, the checked-package-v2 node-identity vectors and preimage schema, and
the TC-185/TC-186/TC-192 procedures are not in the catalog, so the same
test pins their SHA-256 values as read from the pinned revision.

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

The FR-148 IEEE inputs were checked against merged QSpec revision
`5aa00f35056c65948de93ad339540974d35c368a` (QSpec #71): the vendored
`value-ieee754-2019-default.md` and `FR-148-evaluate-ieee-floating-profiles.md`
bytes are identical there, and that revision's lock keeps the same
`ieee_profile` and `rule_fr_148` digests. The TC-193 procedure
`spec/test-cases/TC-193-ieee-exceptional-and-rounding-profiles.md` was copied
with `git show 5aa00f3:<path>` and is byte-identical at `4780a9e`; the same lock
test pins its SHA-256, and `tests/ieee_profiles.rs` reads its vector ids.
