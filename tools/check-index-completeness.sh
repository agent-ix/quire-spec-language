#!/usr/bin/env bash
# QSL-168: fail when an FR or TC artifact exists under spec/ but has no row
# in the master index (spec/spec.md's Requirements table for FR, and the
# `Test ID` column of every spec/**/tests.md TestMatrix's Test Case Summary
# table for TC). Without this check the master index falls behind silently,
# every new spec PR is "consistent" with the gap as it stands, and the gap
# compounds indefinitely (QSL-168).
#
# This intentionally checks only ID presence, not row content, direction or
# accuracy -- it is a completeness net, not a content reviewer.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fail=0

check_kind() {
	local kind="$1" index_grep="$2"
	local artifacts indexed missing

	artifacts="$(find spec -name "${kind}-*.md" -print0 |
		xargs -0 -n1 basename |
		sed -E "s/^(${kind}-[0-9]+)-.*/\1/" |
		sort -u)"

	indexed="$(eval "$index_grep" | sort -u)" || true

	missing="$(comm -23 <(printf '%s\n' "$artifacts") <(printf '%s\n' "$indexed"))"

	if [ -n "$missing" ]; then
		echo "check-index-completeness: ${kind} artifact(s) missing from the master index:" >&2
		echo "$missing" | sed 's/^/  /' >&2
		fail=1
	fi
}

# FR: every `[FR-NNN](...)` link row in spec/spec.md's `## Requirements` table.
check_kind "FR" \
	"grep -ohE '^\\| \\[FR-[0-9]+\\]' spec/spec.md | grep -ohE 'FR-[0-9]+'"

# TC: every `| TC-NNN | ...` row in any spec/**/tests.md Test Case Summary table.
check_kind "TC" \
	"grep -rohE '^\\| TC-[0-9]+' spec/tests.md spec/*/tests.md 2>/dev/null | grep -ohE 'TC-[0-9]+'"

if [ "$fail" -ne 0 ]; then
	echo "check-index-completeness: FAILED -- add the missing row(s) to spec/spec.md and/or spec/tests.md (or the owning subsystem tests.md)." >&2
	exit 1
fi

echo "check-index-completeness: every FR and TC artifact under spec/ has an index row."
