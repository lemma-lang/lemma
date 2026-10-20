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
#      split over several lines is covered whole and a layout change is not a move;
#      a contract keyword outside those regions counts too.
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

# One line per function: its contract, from the fn line (or a lone `export`
# line above it) to the impl line, without comments and with whitespace
# collapsed, prefixed "contract:". Outside contracts, any line holding a
# contract keyword (reserved everywhere) is listed too, prefixed "outside:", so
# a clause written after an impl cannot slip by.
contracts_of() {
  awk '
    function collapse(text) {
      gsub(/[ \t]+/, " ", text); sub(/^ /, "", text); sub(/ $/, "", text)
      return text
    }
    function flush() {
      if (buf != "") print "contract: " collapse(buf)
      buf = ""
    }
    { line = $0; sub(/\/\/.*$/, "", line) }
    line ~ /^[ \t]*export[ \t]*$/ { flush(); buf = "export"; inside = 1; pending = 1; next }
    line ~ /^[ \t]*(export[ \t]+)?fn[ \t]/ { if (!pending) flush(); pending = 0; inside = 1 }
    inside && line ~ /^[ \t]*impl([ \t{]|$)/ { flush(); inside = 0; next }
    inside { buf = buf " " line; next }
    line ~ /(^|[^a-z0-9_])(export|fn|eff|req|ens|spec|equiv|allow)([^a-z0-9_]|$)/ {
      print "outside: " collapse(line)
    }
    END { flush() }
  '
}

# NUL-separated and without rename detection: a path is never split, quoted or
# hidden behind its new name, and a renamed file shows as deleted plus added.
changed_contracts=""
while IFS= read -r -d '' file; do
  if ! git cat-file -e "$ref:$file" 2>/dev/null && [ ! -f "$file" ]; then
    fail "\"$file\" is listed as changed but exists neither at $ref nor in the tree"
  fi
  before=$(git show "$ref:$file" 2>/dev/null | contracts_of)
  after=$(if [ -f "$file" ]; then contracts_of < "$file"; fi)
  [ "$before" = "$after" ] && continue
  delta=$(diff <(printf '%s\n' "$before") <(printf '%s\n' "$after") | grep -E '^[<>]' \
    | sed -e 's/^</  before /' -e 's/^>/  after  /')
  changed_contracts="$changed_contracts$file
$delta
"
done < <(git diff -z --name-only --no-renames "$ref" -- '*.lm'; git ls-files -z --others --exclude-standard -- '*.lm')
report "Contracts (fn line to impl line) changed since $ref:" "${changed_contracts%$'\n'}"

if [ "$moved" -eq 0 ]; then
  printf 'Frozen since %s: tests, wiring and contracts unchanged.\n' "$ref"
  exit 0
fi
printf 'Not frozen since %s: restore from it (git checkout %s -- <paths>) and relaunch the coder.\n' "$ref" "$ref"
exit 1
