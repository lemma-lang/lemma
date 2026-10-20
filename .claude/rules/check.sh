#!/usr/bin/env bash
#
# Checks the shape rules on what changed since a reference.
#
#   .claude/rules/check.sh <reference>
#
# The coder and the tester pass HEAD (their pass, not committed yet), the
# reviewer the branch's merge-base. The working tree is compared with the
# reference, untracked files included.
#
# The rules only apply to the code written in the pass: a finding is kept only
# if it touches an added or modified line. It is "new" if its first line is,
# "touched" otherwise: a pre-existing forest the pass went through, which is
# information, not a task.
#
# Shape rules exist for Rust only (.rs). Every other changed file type is
# listed under "Not checked", so that "0 findings" never means "nothing looked".
# The ticket-key check applies to every file.
#
# Output: one finding per line, then what could not be checked. Exit code 0
# without findings, 1 with some, 2 if the check could not run.
#
# bash 3.2, that is what macOS ships.

set -uo pipefail

# Byte semantics: under a UTF-8 locale, cut and awk fail on a changed file that
# is not valid UTF-8 (a test fixture can be one on purpose) and lose its lines.
export LC_ALL=C

fail() { printf 'check.sh: %s\n' "$1" >&2; exit 2; }

[ $# -eq 1 ] || fail "usage: check.sh <reference> (HEAD for a pass, the merge-base for a review)"
ref=$1

here=$(cd "$(dirname "$0")" && pwd -P)
root=$(git rev-parse --show-toplevel 2>/dev/null) || fail "not inside a Git repository"
cd "$root" || fail "repository root unreachable: $root"
root=$(pwd -P)
git rev-parse --verify -q "$ref^{commit}" >/dev/null || fail "\"$ref\" names no commit"
command -v jq >/dev/null 2>&1 || fail "jq cannot be found"

tmp=$(mktemp -d) || fail "mktemp failed"
trap 'rm -rf "$tmp"' EXIT
not_checked=""

# --- Added lines: file, number, content --------------------------------------
# .claude/ holds the workflow's tooling, indexed by ticket key on purpose.
git diff -U0 --no-ext-diff --no-color --diff-filter=AMR "$ref" -- . ':(exclude).claude' \
  | awk '
      /^\+\+\+ / { f = substr($0, 7); next }
      /^@@/      { split($3, a, ","); n = substr(a[1], 2) + 0; next }
      /^\+/      { printf "%s\t%d\t%s\n", f, n, substr($0, 2); n++ }
    ' > "$tmp/added"
git ls-files -z --others --exclude-standard -- . ':(exclude).claude' \
  | while IFS= read -r -d '' f; do
      [ -f "$f" ] && awk -v f="$f" '{ printf "%s\t%d\t%s\n", f, FNR, $0 }' "$f"
    done >> "$tmp/added"

cut -f1 "$tmp/added" | sort -u > "$tmp/files"
if [ ! -s "$tmp/files" ]; then
  printf 'No line added or modified since %s.\n' "$ref"
  exit 0
fi

: > "$tmp/findings"   # file, start, end, rule, message

# --- ast-grep on Rust: unwrap, unsafe, nesting, chains, comments, test logic -
# Filtered to .rs before the call: with an empty list, xargs would still run
# `ast-grep scan` once, on the whole repository.
grep -E '\.rs$' "$tmp/files" > "$tmp/rs"
if [ -s "$tmp/rs" ]; then
  if command -v ast-grep >/dev/null 2>&1; then
    # ast-grep exits non-zero when an error-severity rule matches: that is a
    # finding, not a failure. Only another message on stderr is one.
    tr '\n' '\0' < "$tmp/rs" \
      | xargs -0 ast-grep scan -c "$here/sgconfig.yml" --json=stream > "$tmp/sg.json" 2> "$tmp/sg.err" \
      || grep -q 'found error level diagnostics' "$tmp/sg.err" \
      || fail "ast-grep failed: $(head -3 "$tmp/sg.err")"
    jq -r '[.file, .range.start.line + 1, .range.end.line + 1, .ruleId, .message] | @tsv' \
      "$tmp/sg.json" >> "$tmp/findings"
  else
    not_checked="${not_checked}- .rs ($(grep -c . "$tmp/rs") file(s)): ast-grep is missing
"
  fi
fi

# --- Every other file type: said, not silently passed -------------------------
grep -vE '\.rs$' "$tmp/files" \
  | awk -F/ '{ n = $NF; i = index(n, "."); print (i > 1 ? substr(n, i) : n) }' \
  | sed -E 's/^.*(\.[^.]+)$/\1/' | sort | uniq -c \
  | while read -r count ext; do
      printf -- '- %s (%s file(s)): no shape rules for this file type\n' "$ext" "$count"
    done > "$tmp/other"
[ -s "$tmp/other" ] && not_checked="${not_checked}$(cat "$tmp/other")
"

# --- Ticket key, in the content as in the file names -------------------------
# Both the prefix the project declares and the one in the branch name: a wrong
# TICKET_KEY must not hide the key the branch actually carries.
declared=""
[ -f .claude/project.conf ] \
  && declared=$(sed -n 's/^TICKET_KEY=[[:space:]]*\([A-Za-z][A-Za-z0-9]*\).*/\1/p' .claude/project.conf | head -1)
branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null | grep -oE '[A-Z][A-Z0-9]+-[0-9]+' | head -1)
branch=${branch%-*}
prefix=$(printf '%s\n%s\n' "$declared" "$branch" | grep . | sort -u | paste -sd '|' -)
[ "${prefix#*|}" = "$prefix" ] || prefix="($prefix)"
if [ -n "$prefix" ]; then
  awk -F '\t' -v re="(^|[^A-Za-z0-9])$prefix-[0-9]+" '
    $3 ~ re { printf "%s\t%d\t%d\tticket-key\tTicket key %s-... in the file\n", $1, $2, $2, p }
  ' p="$prefix" "$tmp/added" >> "$tmp/findings"
  grep -E "(^|[^A-Za-z0-9])$prefix-[0-9]+" "$tmp/files" \
    | while IFS= read -r f; do printf '%s\t1\t1\tticket-key\tTicket key in the file name\n' "$f"; done \
    >> "$tmp/findings"
else
  not_checked="${not_checked}- ticket key: no TICKET_KEY in .claude/project.conf and none in the branch name
"
fi

# --- Keep only what touches an added line ------------------------------------
awk -F '\t' '
  NR == FNR { added[$1 "\t" $2] = 1; next }
  {
    keep = 0
    for (l = $2; l <= $3; l++) if (($1 "\t" l) in added) { keep = 1; break }
    if (!keep) next
    state = (($1 "\t" $2) in added) ? "new" : "touched"
    printf "%s:%d\t%s\t%s\t(%s)\n", $1, $2, $4, $5, state
  }
' "$tmp/added" "$tmp/findings" | awk '!seen[$0]++' | sort -t: -k1,1 -k2,2n > "$tmp/kept"

n=$(grep -c . "$tmp/kept")
[ "$n" -eq 0 ] || cat "$tmp/kept"
printf '%s finding(s) on the lines changed since %s.\n' "$n" "$ref"
[ -z "$not_checked" ] || printf 'Not checked:\n%s' "$not_checked"
[ "$n" -eq 0 ]
exit $?
