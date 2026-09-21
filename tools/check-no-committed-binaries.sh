#!/usr/bin/env bash
# Fail when a tracked file is executable/binary content, or exceeds the size
# ceiling. Two stripped x86-64 ELF executables (5.2 MB and 5.8 MB) were
# committed to artifacts/compiled-protocol-{v1,v2}/dependencies/0.bin,
# behind dependencies/*.bin filenames indistinguishable from the plain-text
# JSON members alongside them, and a size ceiling
# (`examples/protocol-handoff/producer.rs`'s old `BINARY_BYTES`) was raised
# to 16 MiB to let them through instead of being asked whether they should
# exist. QSL-169.
#
# This check detects content by its actual bytes, not by filename or
# extension: an extension allow-list is trivially defeated by naming a file
# "0.bin", ".dat", or nothing at all, which is exactly how the two
# executables above got in undetected.
#
# Honest limits: this catches every native executable format (ELF/PE/Mach-O
# magic) plus ZIP-family and gzip containers, and otherwise falls back to a
# NUL-byte scan of the first 64 KiB -- which is what a compiled binary of any
# other format reliably contains. It is not a general "is this binary" oracle:
# a NUL-free, ASCII-headed binary format would pass. Do not add detection for
# base64/PEM/hex-armored content -- that is printable by design, flagging it
# is a judgement call rather than a detection failure, and a gate that fires
# on armored text gets switched off by the first person it inconveniences.
#
# No allow-list carve-outs: a tracked file that trips this check is the next
# thing to remove, not an exception to add here.
#
# Uses GNU `stat -c%s`; a BSD/macOS `stat` will fail here (not silently).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# The largest legitimate tracked text artifact today is
# artifacts/compiled-protocol-v2/mutations/binding-surplus.json at 223,978
# bytes, one of ~30 near-identical mutation-corpus JSON fixtures in that
# family. 1 MiB (1,048,576 bytes) is ~4.7x that measured largest file -- real
# headroom for that fixture family to grow, while sitting more than 23x below
# the two ELF executables (5.2 MB, 5.8 MB) this gate exists to catch. It's a
# multiple of a measured artifact, not a round guess.
#
# Wired into `make ci` and the CI workflow (Makefile, .github/workflows/ci.yml).
# Raising the ceiling or adding a path carve-out to let a tracked file through
# would recreate exactly the trap this script exists to close.
max_bytes=1048576

fail=0

is_executable_magic() {
	case "$1" in
	7f454c46) return 0 ;;                                  # ELF
	4d5a*) return 0 ;;                                      # PE/DOS (MZ)
	feedface | cefaedfe | feedfacf | cffaedfe) return 0 ;;   # Mach-O 32/64-bit
	cafebabe | bebafeca) return 0 ;;                         # Mach-O fat binary
	504b0304) return 0 ;;                                    # ZIP/JAR/APK/OOXML (docx, xlsx, ...)
	1f8b*) return 0 ;;                                       # gzip
	esac
	return 1
}

while IFS= read -r -d '' file; do
	# A submodule gitlink or other non-blob entry has no working-tree file.
	[ -f "$file" ] || continue

	size="$(stat -c%s "$file")"
	if [ "$size" -gt "$max_bytes" ]; then
		echo "check-no-committed-binaries: $file is $size bytes, over the ${max_bytes}-byte ceiling" >&2
		fail=1
		continue
	fi
	[ "$size" -eq 0 ] && continue

	magic="$(head -c 4 "$file" | od -An -tx1 | tr -d ' \n')"
	if is_executable_magic "$magic"; then
		echo "check-no-committed-binaries: $file has an executable magic ($magic)" >&2
		fail=1
		continue
	fi

	# General binary-content check: a NUL byte anywhere in a sample of the
	# file is the standard heuristic (also used by `file`/git) for "this is
	# not text", independent of what the file claims to be via its name.
	sample_bytes="$(head -c 65536 "$file" | wc -c)"
	stripped_bytes="$(head -c 65536 "$file" | tr -d '\000' | wc -c)"
	if [ "$sample_bytes" != "$stripped_bytes" ]; then
		echo "check-no-committed-binaries: $file contains a NUL byte in its first 64 KiB (binary content)" >&2
		fail=1
	fi
done < <(git ls-files -z)

if [ "$fail" -ne 0 ]; then
	echo "check-no-committed-binaries: FAILED -- remove the file(s) above; do not add an extension or path carve-out." >&2
	exit 1
fi

echo "check-no-committed-binaries: every tracked file is at most ${max_bytes} bytes and has no executable/binary content."
