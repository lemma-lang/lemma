#!/usr/bin/env bash
#
# Guard for the pipeline, security, proof, performance, design and GitLab
# roles: PreToolUse hook, complementing path-guard.sh, which ignores these
# roles and would let them write anything.
#
# Same contract: JSON on stdin, code 0 to let through, code 2 to refuse
# with the message on stderr. Any other status is turned into a refusal by
# the EXIT trap. bash 3.2.

set -uo pipefail

refuse() { printf '%s\n' "$1" >&2; exit 2; }

# Fail closed: a crash exits with 1, which the harness does not treat as a
# refusal. A guard that breaks must refuse.
trap 'status=$?
if [ "$status" -ne 0 ] && [ "$status" -ne 2 ]; then
  printf "Role guard: internal error (status %s), refused as a precaution.\n" "$status" >&2
  exit 2
fi' EXIT

command -v jq >/dev/null 2>&1 || refuse "Role guard: \"jq\" not found, everything is refused until it is installed."

payload=$(cat)
printf '%s' "$payload" | jq -e . >/dev/null 2>&1 || refuse \
  "Role guard: unreadable PreToolUse event, refused as a precaution."

role=$(printf '%s' "$payload"  | jq -r '.agent_type // ""')
tool=$(printf '%s' "$payload"  | jq -r '.tool_name  // ""')
target=$(printf '%s' "$payload" | jq -r '.tool_input.file_path // ""')
cmd=$(printf '%s' "$payload"   | jq -r '.tool_input.command   // ""')
cwd=$(printf '%s' "$payload"   | jq -r '.cwd // ""')
cwd=${cwd%/}

# The main session is not a role: it orchestrates, and nothing bounds it.
[ -n "$role" ] || exit 0

# What the shell runs as commands, as opposed to the data it carries: a
# heredoc body (a commit message, an issue description) and a quoted string
# (a grep pattern, a -m message) are dropped, so that a commit saying "apply
# cargo fmt" or a `grep "cargo install"` is not mistaken for running it. A
# speed bump like the rest: `"cargo" fmt` slips through.
code_of() { # $1 = command
  printf '%s\n' "$1" | awk '{ print } /<</ { exit }' \
    | sed -E -e "s/([A-Za-z_][A-Za-z0-9_]*)=[\"']([^\"' ]*)[\"']/\\1=\\2/g" \
             -e "s/'[^']*'//g" -e 's/"[^"]*"//g'
}
code=$(code_of "$cmd")

# glab carries the account's write token: outside the issue and merge request
# roles, nobody uses it, except for pipeline validation by the release engineer.
if [ "$tool" = "Bash" ] && [ -n "$cmd" ]; then
  case "$role" in
    ticket-reader|ticket-writer|mr-writer|mr-reviewer) ;;
    *)
      case "$(printf '%s\n' "$code" | head -n 1)" in
        *"glab ci lint"*) [ "$role" = "release-engineer" ] || refuse \
"Command refused: glab is reserved for the issue roles, and you are $role." ;;
        *glab\ *|glab) refuse \
"Command refused: glab carries GitLab write permissions, and you are $role.
Whatever must change in an issue goes through the ticket-writer." ;;
      esac ;;
  esac
fi

# Cargo subcommands that rewrite the tree (formatting, automatic fixes,
# dependency edits) without naming a single path. Only the coder and the
# tester write source; every other role observes, and a reviewer that runs
# `cargo fmt` or `cargo clippy --fix` has fixed what it was reviewing.
# The issue and merge request roles are left to their allowlist below, which
# already decides every command they may run. Subagents that are not
# orchestration roles (general-purpose, a fork of the main session) act for
# the main session and are not bounded here.
if [ "$tool" = "Bash" ] && [ -n "$cmd" ]; then
  case "$role" in
    planner|reviewer|auditor|debugger|committer|ticket-brancher|release-engineer|\
    devsecops|proof-engineer|perf-engineer|language-designer)
      writes=0
      if printf '%s' "$code" | grep -Eq 'cargo([[:space:]]+\+[^[:space:]]+)?[[:space:]]+fmt' \
         && ! printf '%s' "$code" | grep -Eq -- '--check'; then writes=1; fi
      printf '%s' "$code" | grep -Eq 'cargo([[:space:]]+\+[^[:space:]]+)?[[:space:]]+(fix|add|remove|rm|install)([[:space:]]|$)' && writes=1
      printf '%s' "$code" | grep -Eq -- '(^|[[:space:]])--(fix|allow-dirty|allow-staged)([[:space:]=]|$)' && writes=1
      if printf '%s' "$code" | grep -Eq 'cargo([[:space:]]+\+[^[:space:]]+)?[[:space:]]+update' \
         && ! printf '%s' "$code" | grep -Eq -- '--dry-run'; then writes=1; fi
      if [ "$role" != "perf-engineer" ] && printf '%s' "$code" | grep -Eq '(^|[;&|[:space:]])rustfmt[[:space:]]' \
         && ! printf '%s' "$code" | grep -Eq -- '--check'; then writes=1; fi
      [ "$writes" -eq 0 ] || refuse \
"Command refused: it rewrites source files or dependencies (cargo fmt without
--check, cargo fix, --fix, cargo add/remove/update/install), and you are $role.
    $cmd
Run the read-only form (cargo fmt --all --check, cargo clippy without --fix,
cargo update --dry-run) and report what should change: the coder applies it." ;;
  esac
fi

case "$role" in
  release-engineer|devsecops|proof-engineer|perf-engineer|language-designer|\
  ticket-reader|ticket-writer|mr-writer|mr-reviewer) ;;
  *) exit 0 ;;
esac

# The issue and merge request roles only have the shell for glab, git reads
# and, for the mr-reviewer, the checks: an allowlist of commands, judged on
# the first line (the rest may be the body of a heredoc).
cmd_issues=""
if [ "$tool" = "Bash" ]; then
  case "$role" in ticket-reader|ticket-writer|mr-writer|mr-reviewer) cmd_issues="$cmd" ;; esac
fi
if [ -n "$cmd_issues" ]; then
  head_line=$(printf '%s\n' "$cmd_issues" | head -n 1)
  case "$head_line" in
    *'&&'*|*'||'*|*';'*|*'`'*) refuse \
"Command refused: chaining (&&, ||, ;, backquote) is not allowed, and you are $role.
Run a single command per call." ;;
  esac
  without_heredoc=$(printf '%s' "$head_line" | sed -e "s/\\\$(cat <<'EOF'//g")
  case "$without_heredoc" in
    *'$('*) refuse \
"Command refused: only the substitution \"\$(cat <<'EOF' … EOF)\" is allowed, and you are $role." ;;
  esac
  # A "glab api" without a method or field is a GET; -f/-F would turn it into a POST.
  case "$head_line" in
    "glab api projects/:id/issues/"*)
      case "$head_line" in
        *" -X"*|*" --method"*|*" -f "*|*" -F "*|*" --field"*|*" --raw-field"*|*" --input"*) ;;
        *) exit 0 ;;
      esac ;;
  esac
  case "$role" in
    ticket-reader)
      case "$head_line" in
        "glab issue view "*|"glab issue list"*|"glab label list"*|"glab milestone list"*|\
        "git log"*|"git show"*|"git branch"*|"git status"*) exit 0 ;;
      esac ;;
    mr-reviewer)
      # The force refusal comes BEFORE the allowlist: otherwise "glab mr merge --yes"
      # would exit through the exit 0 of the "glab mr merge" pattern without ever being examined.
      case "$head_line" in
        *"--force"*|*"--yes"*|*" -y"*|*"--no-confirm"*) refuse \
"Command refused: no force or auto-confirm option, and you are mr-reviewer.
The merge is submitted to the user, it is neither forced nor pre-confirmed." ;;
        *"--fix"*|*"--allow-dirty"*|*"--allow-staged"*|*"--bless"*) refuse \
"Command refused: this option rewrites files, and you are mr-reviewer.
You observe the branch; what must change goes back to the coder." ;;
      esac
      case "$head_line" in
        "glab mr view"*|"glab mr list"*|"glab mr diff"*|"glab mr note "*|"glab mr approve "*|\
        "glab mr merge "*|"glab ci status"*|\
        "git log"*|"git diff"*|"git show"*|"git branch"*|"git status"*|"git merge-base"*|\
        "git rev-parse"*|"git symbolic-ref"*|"git fetch"*|\
        "cargo fmt"*"--check"*|"cargo clippy"*|"cargo test"*|"cargo build"*|"cargo check"*|\
        "cargo doc"*|"RUSTDOCFLAGS=\"-D warnings\" cargo doc"*|"RUSTDOCFLAGS='-D warnings' cargo doc"*|\
        "cargo deny check"*|"cog check"*|"cog verify"*) exit 0 ;;
      esac ;;
    mr-writer)
      case "$head_line" in
        "glab mr view"*|"glab mr list"*|"glab mr create "*|"glab mr update "*|"glab mr note "*|\
        "git log"*|"git diff"*|"git show"*|"git branch"*|"git status"*|"git merge-base"*|\
        "git rev-parse"*) exit 0 ;;
      esac ;;
    ticket-writer)
      case "$head_line" in
        "glab issue view "*|"glab issue list"*|"glab issue create "*|"glab issue update "*|"glab issue note "*|\
        "glab label list"*|"glab label create "*|"glab milestone list"*|\
        "glab api -X POST projects/:id/issues/"*/links" "*|\
        "glab api -X DELETE projects/:id/issues/"*/links/*|"glab api -X PUT projects/:id/issues/"*/notes/*) exit 0 ;;
      esac ;;
  esac
  refuse \
"Command refused: \"$head_line\" is not among the commands allowed for $role.
See the list in your prompt. Closing, reopening, assigning or deleting an issue is up to the user."
fi

# Same anchor as path-guard.sh, so that both guards judge the same tree: the
# Git toplevel when it holds a .claude/, otherwise the highest ancestor holding
# one below the home directory, otherwise CLAUDE_PROJECT_DIR.
anchor() {
  local top d found=""
  if [ -n "$cwd" ] && [ -d "$cwd" ]; then
    top=$(git -C "$cwd" rev-parse --show-toplevel 2>/dev/null) || top=""
    if [ -n "$top" ] && [ -d "$top/.claude" ]; then printf '%s' "${top%/}"; return 0; fi
  fi
  d="$cwd"
  while [ -n "$d" ] && [ "$d" != "/" ]; do
    if [ -d "$d/.claude" ] && [ "$d" != "${HOME%/}" ]; then found="$d"; fi
    d=$(dirname "$d")
  done
  if [ -z "$found" ] && [ -n "${CLAUDE_PROJECT_DIR:-}" ] && [ -d "${CLAUDE_PROJECT_DIR%/}/.claude" ]; then
    found=${CLAUDE_PROJECT_DIR%/}
  fi
  printf '%s' "$found"
}
project=$(anchor)
[ -n "$project" ] || refuse "Role guard: project root not found from \"$cwd\", refused as a precaution."

zone_of() { # $1 = role → readable description of its zone
  case "$1" in
    release-engineer)  echo ".gitlab-ci.yml, .gitlab/, ci/, cog.toml, deny.toml, clippy.toml, rustfmt.toml, rust-toolchain.toml, .cargo/config.toml, renovate.json" ;;
    devsecops)         echo ".claude/security/" ;;
    proof-engineer)    echo ".claude/proofs/, and the .lm sources under examples/ (specs only, by rule)" ;;
    perf-engineer)     echo ".claude/perf/, benches/ and crates/*/benches/" ;;
    language-designer) echo "docs/ (ADRs in docs/adr/, docs/design.md)" ;;
    mr-writer)         echo ".claude/tmp/ only (description drafts)" ;;
    ticket-writer)     echo ".claude/tmp/ only (description drafts)" ;;
    mr-reviewer)       echo ".claude/reviews/mr/" ;;
    ticket-reader)     echo "nothing: no file" ;;
  esac
}

in_zone() { # $1 = role, $2 = path relative to the project
  case "$1" in
    release-engineer)
      case "$2" in
        .gitlab-ci.yml|.gitlab/*|ci/*|cog.toml|deny.toml|clippy.toml|.clippy.toml|rustfmt.toml|.rustfmt.toml|\
        rust-toolchain|rust-toolchain.toml|.cargo/config.toml|renovate.json) return 0 ;;
      esac ;;
    devsecops)
      case "$2" in .claude/security/*) return 0 ;; esac ;;
    proof-engineer)
      case "$2" in .claude/proofs/*|examples/*.lm) return 0 ;; esac ;;
    perf-engineer)
      case "$2" in .claude/perf/*|benches/*|crates/*/benches/*) return 0 ;; esac ;;
    language-designer)
      case "$2" in docs/*) return 0 ;; esac ;;
    mr-reviewer)
      case "$2" in .claude/reviews/mr/*) return 0 ;; esac ;;
    mr-writer|ticket-writer)
      case "$2" in .claude/tmp/*) return 0 ;; esac ;;
  esac
  return 1
}

if [ "$tool" = "Write" ] || [ "$tool" = "Edit" ] || [ "$tool" = "NotebookEdit" ]; then
  [ -n "$target" ] || exit 0
  path="$target"
  while :; do case "$path" in ./*) path=${path#./} ;; *) break ;; esac; done
  case "$path" in /*) abs="$path" ;; *) abs="$cwd/$path" ;; esac

  case "$abs" in */../*|*/..) refuse \
"Write refused: \"$target\" contains \"..\". Give the path again in plain form from $project/." ;;
  esac

  case "$abs" in
    "$project"/*) rel=${abs#"$project"/} ;;
    *) refuse "Write refused: \"$abs\" is outside the project $project, and you are $role." ;;
  esac

  in_zone "$role" "$rel" || refuse \
"Write refused: \"$rel\" is outside your zone, and you are $role.
Your zone: $(zone_of "$role")
Whatever must change elsewhere is described in your report, along with the role that will apply it."
  exit 0
fi

# Through the shell, only a speed bump: the advisory roles have nothing to
# write outside their directory, we stop the obvious forms. A redirection or a
# tee into the role's own report directory is allowed: a solver log or a
# benchmark run saved next to the report.
if [ "$tool" = "Bash" ] && [ -n "$cmd" ]; then
  case "$role" in
    devsecops)         own=".claude/security/" ;;
    proof-engineer)    own=".claude/proofs/" ;;
    perf-engineer)     own=".claude/perf/" ;;
    language-designer) own="" ;;
    *) exit 0 ;;
  esac
  # The code part only: `grep -n "cargo install" .gitlab-ci.yml` installs nothing.
  cmd_examined=$(printf '%s' "$code" | sed -e 's/[0-9]*>&[0-9-]*//g' \
    -e 's#[0-9&]*>[[:space:]]*/dev/null##g' -e 's/->//g' -e 's/=>//g')
  if [ -n "$own" ]; then
    own_re=$(printf '%s' "$own" | sed -e 's/\./\\./g')
    cmd_examined=$(printf '%s' "$cmd_examined" | sed -E \
      -e "s#[0-9&]*>>?[[:space:]]*[^[:space:]]*${own_re}[^[:space:]]*##g" \
      -e "s#tee([[:space:]]+-a)?[[:space:]]+[^[:space:]]*${own_re}[^[:space:]]*##g")
  fi
  case "$cmd_examined" in
    *sed\ -i*|*tee\ *|*\>*|*mv\ *|*cp\ *|*rm\ *|*truncate*|*patch\ *|*install\ *|*\ add\ *|*sudo\ *)
      refuse \
"Command refused: it appears to write, install or escalate, and you are $role.
    $cmd
Your shell commands are for reading, building, running and measuring. Write
your report with the Write tool, in $(zone_of "$role"). If a measurement needs
root (sudo), say so in your report: the user runs it." ;;
  esac
fi

exit 0
