#!/usr/bin/env bash
# Prepares a worktree usable by the roles.
#
# The roles, the guards, the settings and the policy are committed under
# .claude/, so a worktree gets them from its own checkout. What is *not*
# committed is the living memory (plans, state, reviews, diagnostics, proof,
# perf and security reports), and a worktree without it would give a ticket
# two states. Those directories are shared by symbolic link.
#
# Builds share one Cargo target directory: without it, every worktree would
# rebuild llvm-sys, inkwell and z3 from scratch. See `shared_target` below.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
shelter="$root/../$(basename "$root")-worktrees"
hook="$root/.claude/tools/worktree-local.sh"

# Local, untracked memory (see .gitignore). Linked one by one: .claude/ itself
# is tracked, so it already exists in the worktree.
memory="plans state reviews diagnostics security proofs perf tmp"

usage() {
  cat <<'END'
Usage:
  worktree.sh create <branch> [base]   creates the worktree and makes it usable by the roles
  worktree.sh remove <branch>          removes the worktree (refuses if it holds work)
  worktree.sh list                     lists the worktrees

The worktree checks out its own .claude/ (roles, guards, settings), and shares
with the main tree, by symbolic link, the local memory: .claude/plans, state,
reviews, diagnostics, security, proofs, perf, tmp, plus settings.local.json
and .env when they exist. Commits live in the branch, not in the worktree.

Cargo builds of every worktree go to the main tree's target directory
(override with WORKTREE_TARGET_DIR): the LLVM- and Z3-linked crates are built
once. The price: two worktrees building at the same time wait for each other
on Cargo's build-directory lock.

What is specific to the project goes in .claude/tools/worktree-local.sh, if it
exists and is executable. It is called with: create|remove <branch> <worktree-path>.
END
}

link() {
  local target=$1 source=$2
  [ -e "$source" ] || return 0
  if [ -e "$target" ] || [ -L "$target" ]; then return 0; fi
  ln -s "$source" "$target"
}

# Cargo reads .cargo/config.toml in the current directory *and in every
# parent*, merging them. The committed .cargo/config.toml sets no target-dir,
# so a config in the shelter directory, above every worktree, points them all
# at one target directory without touching a tracked file and without an
# environment variable (which would not survive from one agent command to the
# next). CARGO_TARGET_DIR, when set, still wins.
shared_target() {
  local dir=$1 target=${WORKTREE_TARGET_DIR:-$root/target}
  mkdir -p "$dir/.cargo"
  if [ ! -f "$dir/.cargo/config.toml" ]; then
    printf '# Written by .claude/tools/worktree.sh: worktrees below share one build directory.\n[build]\ntarget-dir = "%s"\n' \
      "$target" > "$dir/.cargo/config.toml"
  fi
}

project_hook() {
  [ -x "$hook" ] || return 0
  echo "--- project hook: $(basename "$hook") $* ---"
  "$hook" "$@"
}

create() {
  local branch=${1:?branch name expected}
  local base=${2:-origin/main}
  local path="$shelter/$branch"

  [ -e "$path" ] && { echo "already exists: $path" >&2; exit 1; }
  mkdir -p "$shelter"
  shared_target "$shelter"

  if git show-ref --verify --quiet "refs/heads/$branch"; then
    git worktree add "$path" "$branch"
  else
    git fetch --quiet origin || true
    git worktree add -b "$branch" "$path" "$base"
  fi

  # A base older than the committed setup checks out no .claude/: no roles,
  # no guards. Say so plainly rather than failing on the first link.
  if [ ! -d "$path/.claude" ]; then
    echo "the base predates the committed setup: $path has no .claude/, so no roles and no guards." >&2
    echo "Start from a base that contains .claude/, or remove it: $0 remove $branch" >&2
    exit 1
  fi

  # The memory directories are created in the main tree first: a link to a
  # missing directory would let the worktree grow its own, and the ticket two states.
  for dir in $memory; do
    mkdir -p "$root/.claude/$dir"
    link "$path/.claude/$dir" "$root/.claude/$dir"
  done
  link "$path/.claude/settings.local.json" "$root/.claude/settings.local.json"
  link "$path/.env" "$root/.env"

  project_hook create "$branch" "$path"

  verify "$path"
  echo
  echo "ready: $path"
  echo "to work in it: cd $path"
}

verify() {
  local path=$1 missing=0 expected dir
  for expected in .claude/agents .claude/hooks/path-guard.sh .claude/hooks/role-guard.sh \
                  .claude/settings.json .claude/test-paths.txt CLAUDE.md; do
    if [ -e "$path/$expected" ]; then
      echo "ok      $expected"
    else
      echo "MISSING $expected (does the base predate the committed setup?)" >&2
      missing=1
    fi
  done
  for dir in $memory; do
    if [ -L "$path/.claude/$dir" ]; then
      echo "shared  .claude/$dir"
    else
      echo "MISSING .claude/$dir link" >&2
      missing=1
    fi
  done
  echo "target  $(sed -n 's/^target-dir = "\(.*\)"/\1/p' "$shelter/.cargo/config.toml" 2>/dev/null)"
  return $missing
}

remove() {
  local branch=${1:?branch name expected}
  local path="$shelter/$branch"
  local dir symlink
  [ -e "$path" ] || { echo "unknown: $path" >&2; exit 1; }

  project_hook remove "$branch" "$path"

  # The links go before git: git worktree remove must not reach the main
  # tree's shared memory through them.
  for dir in $memory; do
    if [ -L "$path/.claude/$dir" ]; then rm "$path/.claude/$dir"; fi
  done
  for symlink in .claude/settings.local.json .env; do
    if [ -L "$path/$symlink" ]; then rm "$path/$symlink"; fi
  done
  git worktree remove "$path"
  echo "removed: $path"
}

case "${1:-}" in
  create) shift; create "$@" ;;
  remove) shift; remove "$@" ;;
  list)   git worktree list ;;
  *)      usage; exit 1 ;;
esac
