# Complete-value normative inputs

The files below `quire-specification/` are unmodified bytes of
[agent-ix/quire-specification](https://github.com/agent-ix/quire-specification)
at merged revision `a25c93c875231664d90f5d4b3f2ac51f1dc77a75` (QSpec PR #73, amending QSpec #68),
copied with `git show <revision>:<path>` at their repository-relative paths so
the lock's relative `artifact_path` values resolve unchanged. QSL #118 /
Plan-013 Task-048 consumes them; QSL #119 / Plan-013 Task-049 adds the
FR-143, FR-144, FR-145, FR-146 and FR-307 rules and the TC-188, TC-189,
TC-190, TC-191, TC-194 and TC-227 procedures, and consumes FR-149.

`complete-value-lock.json` is the authority for every qualification-catalog
artifact digest; `tests/complete_value_lock.rs` recomputes each vendored
artifact's raw-byte SHA-256 against that lock. The lock itself, the selection
vectors, the checked-package-v2 node-identity vectors and preimage schema, and
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

FR-148 IEEE profiles (TC-193) read the `quire.value.ieee754-2019-default/v1`
definition, the FR-148 rule and the TC-193 procedure; `tests/ieee_profiles.rs`
reads the procedure's vector ids.
