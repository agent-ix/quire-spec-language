#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Write run.json and compile.json for one generated ConfigVersion case.
#
#   examples/config-version/requests.sh /tmp/config-version/healthy-parent ParentOrder
#
# Both requests select the case's 1-draft unit and its domain model by digest;
# run.json checks CLAUSE for the object keyed "child" in the current snapshot.
set -eu
dir=$1
clause=${2:-ParentOrder}
ref() {
  digest=$(shasum -a 256 "$dir/$1" | cut -d' ' -f1)
  printf '{"file":"%s","digest":"sha256:%s","authority":"agent-ix","identity":"example:%s","revision_namespace":"fixture","revision":"1","document":"%s","formal_revision":1}' \
    "$1" "$digest" "$1" "$2"
}
models="[{\"format\":\"semantic-ir/2.0.0\",\"source\":$(ref spine-model.semantic-ir.json SpineModel)}]"
program="{\"source\":$(ref spine-unit.native SpineUnit)}"
self='{"population":"ix://example/config-version/config_history","key":"child"}'
printf '{"format":"native-run/1","request":{"models":%s,"program":%s,"clause":{"selection":{"clause":{"name":"%s","observation":"current","self":%s,"anchor":{"kind":"handler","name":"validate"}}},"observations":{"current":"spine-snapshot.json"}}}}\n' \
  "$models" "$program" "$clause" "$self" > "$dir/run.json"
printf '{"format":"native-compile/1","request":{"models":%s,"program":%s}}\n' \
  "$models" "$program" > "$dir/compile.json"
