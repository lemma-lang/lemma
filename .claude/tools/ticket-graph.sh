#!/usr/bin/env bash
#
# Shows which open tickets can be started, from GitHub's "blocked by"
# relationships.
#
#   .claude/tools/ticket-graph.sh [--all]
#
# One line per open issue: its key, the open issues blocking it, how many open
# issues wait on it downstream (directly or through others), and the ticket
# branch if one exists. Ready tickets (no open blocker) come first, those that
# unblock the most first. Tracking issues are left out; --all keeps them.
#
# A blocker written only in prose ("Depends on: ADR 0006") is not a GitHub
# relationship: read the ticket before starting it.
#
# Read-only: gh issue list and git branch.
#
# bash 3.2, that is what macOS ships.

set -uo pipefail

fail() { printf 'ticket-graph.sh: %s\n' "$1" >&2; exit 2; }

show_tracking=false
case "${1:-}" in
  "") ;;
  --all) show_tracking=true ;;
  *) fail "usage: ticket-graph.sh [--all]" ;;
esac

command -v gh >/dev/null 2>&1 || fail "gh cannot be found"
command -v jq >/dev/null 2>&1 || fail "jq cannot be found"
root=$(git rev-parse --show-toplevel 2>/dev/null) || fail "not inside a Git repository"
key=$(sed -n 's/^TICKET_KEY=//p' "$root/.claude/project.conf" 2>/dev/null)
[ -n "$key" ] || fail "TICKET_KEY not found in .claude/project.conf"

issues=$(gh issue list --state open --limit 500 --json number,title,labels,blockedBy,blocking) \
  || fail "gh issue list failed (gh auth status?)"
branches=$(git -C "$root" branch -a --format='%(refname:short)' | sed -E 's#^origin/##' | sort -u)

printf '%s' "$issues" | jq -r --arg key "$key" --arg branches "$branches" \
  --argjson show_tracking "$show_tracking" '
  (map({key: (.number | tostring),
          value: [.blocking.nodes[] | select(.state == "OPEN") | .number]}) | from_entries) as $blocks
  | def downstream($n; $seen):
      reduce ($blocks[$n | tostring] // [])[] as $m ($seen;
        if index([$m]) then . else downstream($m; . + [$m]) end);
  ($branches | split("\n")) as $branch_list
  | map(select($show_tracking or ([.labels[].name] | index("tracking") | not)))
  | map({number, title,
         blockers: [.blockedBy.nodes[] | select(.state == "OPEN") | .number],
         waiting: (downstream(.number; []) | length),
         branch: (.number as $n | [$branch_list[] | select(startswith("\($key)-\($n)-"))] | first // "")})
  | sort_by((.blockers | length > 0), -.waiting, .number)
  | "Ready (no open blocker):"
  , (.[] | select(.blockers | length == 0)
      | "  \($key)-\(.number)  unblocks \(.waiting)\(if .branch != "" then "  [branch \(.branch)]" else "" end)  \(.title)")
  , ""
  , "Blocked:"
  , (.[] | select(.blockers | length > 0)
      | "  \($key)-\(.number)  by \(.blockers | map("#\(.)") | join(", "))  unblocks \(.waiting)\(if .branch != "" then "  [branch \(.branch)]" else "" end)  \(.title)")
'
