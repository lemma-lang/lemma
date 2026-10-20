#!/usr/bin/env bash
#
# Checks that the tests and the specs are still frozen since the red tests
# commit (rule 4 of .claude/orchestration.md).
#
#   .claude/tools/frozen-check.sh <tests-commit>
#
# The working tree is compared with the commit, untracked files included:
#   1. no test file changed, added or removed (the patterns of
#      .claude/test-paths.txt);
#   2. no `#[cfg(test)]` or `mod tests;` wiring line removed;
#   3. no contract of a .lm function changed: everything from its fn line up to its
#      impl line (signature, types and bounds, spec, req, ens, eff, equiv), compared
#      with comments removed and whitespace collapsed, so a clause or a signature
#      split over several lines is covered whole and a layout change is not a move.
#
# Exit code 0 when frozen, 1 when something moved, 2 if the check could not run.
#
# bash 3.2, that is what macOS ships.

set -uo pipefail

fail() { printf 'frozen-check.sh: %s\n' "$1" >&2; exit 2; }

[ $# -eq 1 ] || fail "usage: frozen-check.sh <tests-commit>"
ref=$1

root=$(git rev-parse --show-toplevel 2>/dev/null) || fail "not inside a Git repository"
cd "$root" || fail "repository root unreachable: $root"
git rev-parse --verify -q "$ref^{commit}" >/dev/null || fail "\"$ref\" names no commit"
patterns_file=.claude/test-paths.txt
[ -r "$patterns_file" ] || fail "$patterns_file cannot be read"

# test-paths.txt semantics as Git pathspecs: a pattern without "/" matches the
# file name anywhere, a leading "*/" also matches at the root.
pathspecs=()
while IFS= read -r pattern; do
  case "$pattern" in ''|'#'*) continue ;; esac
  pattern=${pattern#\*/}
  case "$pattern" in */\*) pattern="${pattern%\*}**" ;; esac
  pathspecs+=(":(glob)**/$pattern")
done < "$patterns_file"
[ "${#pathspecs[@]}" -gt 0 ] || fail "no pattern in $patterns_file"

moved=0
report() { # $1 = title, $2 = lines
  [ -n "$2" ] || return 0
  moved=1
  printf '%s\n%s\n\n' "$1" "$2"
}

changed_tests=$(git diff --stat "$ref" -- "${pathspecs[@]}")
untracked_tests=$(git ls-files --others --exclude-standard -- "${pathspecs[@]}")
report "Test files changed since $ref:" "$changed_tests"
report "Test files added since $ref, untracked:" "$untracked_tests"

removed_wiring=$(git diff "$ref" | grep -E '^-[[:space:]]*(#\[cfg\(test\)\]|mod tests;)')
report "Test wiring removed since $ref:" "$removed_wiring"

# One line per function: its contract, from the fn line to the impl line,
# without comments and with whitespace collapsed.
contracts_of() {
  awk '
    function flush() {
      if (buf == "") return
      gsub(/[ \t]+/, " ", buf); sub(/^ /, "", buf); sub(/ $/, "", buf)
      print buf; buf = ""
    }
    { line = $0; sub(/\/\/.*$/, "", line) }
    line ~ /^[ \t]*(export[ \t]+)?fn[ \t]/ { flush(); inside = 1 }
    inside && line ~ /^[ \t]*impl([ \t{]|$)/ { flush(); inside = 0; next }
    inside { buf = buf " " line }
    END { flush() }
  '
}

changed_contracts=""
for file in $(git diff --name-only "$ref" -- '*.lm'; git ls-files --others --exclude-standard -- '*.lm'); do
  before=$(git show "$ref:$file" 2>/dev/null | contracts_of)
  after=$( [ -f "$file" ] && contracts_of < "$file")
  [ "$before" = "$after" ] && continue
  delta=$(diff <(printf '%s\n' "$before") <(printf '%s\n' "$after") | grep -E '^[<>]' \
    | sed -e 's/^</  before:/' -e 's/^>/  after: /')
  changed_contracts="$changed_contracts$file
$delta
"
done
report "Contracts (fn line to impl line) changed since $ref:" "${changed_contracts%$'\n'}"

if [ "$moved" -eq 0 ]; then
  printf 'Frozen since %s: tests, wiring and contracts unchanged.\n' "$ref"
  exit 0
fi
printf 'Not frozen since %s: restore from it (git checkout %s -- <paths>) and relaunch the coder.\n' "$ref" "$ref"
exit 1
