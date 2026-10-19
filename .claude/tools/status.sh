#!/usr/bin/env bash
#
# Where the repository stands, computed rather than remembered: what a session
# reads first when it resumes.
#
#   .claude/tools/status.sh
#
# Current branch and its distance to origin/main, uncommitted changes, the
# worktrees, the open pull requests with their checks, the last CI run on main,
# and the ticket states kept in .claude/state/ with their "Where we are" line.
#
# Read-only, apart from `git fetch origin`; without network it says so and
# compares against the last fetched origin/main.
#
# bash 3.2, that is what macOS ships.

set -uo pipefail

fail() { printf 'status.sh: %s\n' "$1" >&2; exit 2; }

root=$(git rev-parse --show-toplevel 2>/dev/null) || fail "not inside a Git repository"
cd "$root" || fail "repository root unreachable: $root"

section() { printf '\n== %s\n' "$1"; }

section "Branch"
git fetch -q origin 2>/dev/null || printf 'git fetch origin failed: origin/main is the last fetched one.\n'
branch=$(git branch --show-current)
printf '%s at %s\n' "${branch:-detached HEAD}" "$(git log -1 --format='%h %s')"
if git rev-parse --verify -q origin/main >/dev/null; then
  counts=$(git rev-list --left-right --count origin/main...HEAD)
  printf 'origin/main: %s behind, %s ahead (origin/main at %s)\n' \
    "${counts%%	*}" "${counts##*	}" "$(git log -1 --format='%h' origin/main)"
fi
local_main=$(git rev-parse -q --verify main 2>/dev/null) && [ "$local_main" != "$(git rev-parse -q --verify origin/main)" ] \
  && printf 'local main differs from origin/main: refresh it before basing a branch on it.\n'

section "Working tree"
changes=$(git status --short)
if [ -n "$changes" ]; then printf '%s\n' "$changes"; else printf 'clean\n'; fi

section "Worktrees"
git worktree list

section "Open pull requests"
if command -v gh >/dev/null 2>&1; then
  gh pr list --state open --json number,title,headRefName,isDraft,statusCheckRollup --jq '
    if length == 0 then "none" else .[] |
      "#\(.number) \(.headRefName)\(if .isDraft then " (draft)" else "" end): \(.title)  checks: \(
        [.statusCheckRollup[] | (.conclusion // .status // .state)] | group_by(.) | map("\(length) \(.[0])") | join(", ")
        | if . == "" then "none reported" else . end)" end' \
    || printf 'gh pr list failed (gh auth status?)\n'

  section "Last CI run on main"
  gh run list --branch main --workflow CI --limit 1 --json headSha,conclusion,status,createdAt,url \
    --jq '.[] | "\(.headSha[0:7]) \(.conclusion // .status) \(.createdAt) \(.url)"' \
    || printf 'gh run list failed\n'
else
  printf 'gh not found: pull requests and CI not shown.\n'
fi

section "Ticket states (.claude/state/)"
found=false
for state in .claude/state/*.md; do
  [ -e "$state" ] || continue
  found=true
  where=$(grep -m 1 '^Where we are:' "$state" || printf 'Where we are: (not stated)')
  printf '%s (%s)\n  %s\n' "$(basename "$state" .md)" \
    "$(date -r "$state" '+%Y-%m-%d %H:%M')" "$where"
done
$found || printf 'none\n'
