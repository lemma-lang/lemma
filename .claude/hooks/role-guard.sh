#!/usr/bin/env bash
#
# Guard for the pipeline, security, proof, performance, design and GitHub
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

# gh carries the account's write token: outside the issue and pull request
# roles, nobody uses it, except the release-engineer, which reads the state of
# the workflow runs (gh run list|view|watch) and triggers nothing. The whole
# code part is examined, not only its first line, and gh is matched as a word:
# "high" or ".github/" is not a gh call.
gh_calls() { # $1 = code → number of gh invocations
  printf '%s\n' "$1" | grep -Eo '(^|[;&|(`/[:space:]])gh([[:space:]]|$)' | wc -l | tr -d ' '
}
if [ "$tool" = "Bash" ] && [ -n "$cmd" ]; then
  case "$role" in
    ticket-reader|ticket-writer|pr-writer|pr-reviewer) ;;
    *)
      if [ "$(gh_calls "$code")" -gt 0 ]; then
        if [ "$role" = "release-engineer" ] && [ "$(gh_calls "$code")" -eq 1 ] \
           && printf '%s\n' "$code" | grep -Eq '(^|[;&|(`/[:space:]])gh[[:space:]]+run[[:space:]]+(list|view|watch)([[:space:]]|$)'; then
          :
        elif [ "$role" = "release-engineer" ]; then
          refuse \
"Command refused: the release-engineer only reads workflow runs with gh
(gh run list, gh run view, gh run watch). It triggers, reruns or cancels nothing."
        else
          refuse \
"Command refused: gh carries GitHub write permissions, and you are $role.
Whatever must change in an issue goes through the ticket-writer."
        fi
      fi ;;
  esac
fi

# Every role shares the main checkout, and only the ticket-brancher moves its
# HEAD: a role that switches branch, stashes or hard-resets drags another
# role's uncommitted work onto the wrong branch, or throws it away. A restore
# of named files (git checkout <rev> -- <paths>) stays allowed.
git_sub='(^|[;&|(`[:space:]])git([[:space:]]+-[Cc][[:space:]]+[^[:space:]]+|[[:space:]]+--[a-z-]+(=[^[:space:]]+)?)*[[:space:]]+'
if [ "$tool" = "Bash" ] && [ -n "$cmd" ] && [ "$role" != "ticket-brancher" ]; then
  moves=0
  printf '%s\n' "$code" | grep -Eq "${git_sub}(switch|rebase|merge|pull|cherry-pick|revert|bisect)([[:space:]]|\$)" && moves=1
  printf '%s\n' "$code" | grep -Eq "${git_sub}stash([[:space:]]+(push|pop|apply|drop|clear|save|branch|create|store)([[:space:]]|\$)|[[:space:]]*(\$|[;&|)]))" && moves=1
  printf '%s\n' "$code" | grep -Eq "${git_sub}reset([[:space:]][^;&|]*)?[[:space:]]--(hard|keep|merge|soft)([[:space:]]|\$)" && moves=1
  if printf '%s\n' "$code" | grep -Eo "${git_sub}checkout([[:space:]][^;&|]*)?" \
     | grep -Evq '[[:space:]]--([[:space:]]|$)'; then moves=1; fi
  [ "$moves" -eq 0 ] || refuse \
"Command refused: it moves HEAD or rewrites the shared working tree (git switch,
git checkout <branch>, git stash, git reset --hard, rebase, merge, pull...),
and you are $role.
    $cmd
Other roles may have uncommitted work in this checkout. Stay on the branch you
were given; if the work needs another branch, say so in your report: the main
session gives you a worktree (.claude/tools/worktree.sh)."
fi

# Commands that rewrite the tree (formatting, automatic fixes, dependency
# edits) without naming a single path. Only the coder and the tester write
# source; every other role observes, and a reviewer that runs `cargo fmt`,
# `just fmt` or `cargo clippy --fix` has fixed what it was reviewing.
# The issue and pull request roles are left to their allowlist below, which
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
      printf '%s' "$code" | grep -Eq -- '(^|[[:space:]])--(fix|allow-dirty|allow-staged|write-changes)([[:space:]=]|$)' && writes=1
      if printf '%s' "$code" | grep -Eq 'cargo([[:space:]]+\+[^[:space:]]+)?[[:space:]]+update' \
         && ! printf '%s' "$code" | grep -Eq -- '--dry-run'; then writes=1; fi
      if [ "$role" != "perf-engineer" ] && printf '%s' "$code" | grep -Eq '(^|[;&|[:space:]])rustfmt[[:space:]]' \
         && ! printf '%s' "$code" | grep -Eq -- '--check'; then writes=1; fi
      printf '%s' "$code" | grep -Eq '(^|[;&|(`[:space:]])just[[:space:]]+(fmt|setup)([[:space:]]|$)' && writes=1
      if printf '%s' "$code" | grep -Eq '(^|[;&|(`[:space:]])taplo[[:space:]]+(fmt|format)([[:space:]]|$)' \
         && ! printf '%s' "$code" | grep -Eq -- '--check'; then writes=1; fi
      printf '%s' "$code" | grep -Eq '(^|[;&|(`[:space:]])typos([[:space:]]+[^;&|]*)?[[:space:]]-w([[:space:]]|$)' && writes=1
      [ "$writes" -eq 0 ] || refuse \
"Command refused: it rewrites source files or dependencies (cargo fmt, just fmt
or taplo fmt without --check, cargo fix, --fix, typos -w, just setup,
cargo add/remove/update/install), and you are $role.
    $cmd
Run the read-only form (just check, cargo fmt --all --check, cargo clippy
without --fix, cargo update --dry-run) and report what should change: the
coder applies it." ;;
  esac
fi

case "$role" in
  release-engineer|devsecops|proof-engineer|perf-engineer|language-designer|\
  ticket-reader|ticket-writer|pr-writer|pr-reviewer) ;;
  *) exit 0 ;;
esac

# The issue and pull request roles only have the shell for gh, git reads
# and, for the pr-reviewer, the checks: an allowlist of commands, judged on
# the first line (the rest may be the body of a heredoc).
cmd_issues=""
if [ "$tool" = "Bash" ]; then
  case "$role" in ticket-reader|ticket-writer|pr-writer|pr-reviewer) cmd_issues="$cmd" ;; esac
fi
if [ -n "$cmd_issues" ]; then
  head_line=$(printf '%s\n' "$cmd_issues" | head -n 1)
  # A second line outside a heredoc is a second command the allowlist below
  # would never see.
  if [ "$(printf '%s\n' "$code" | grep -c '[^[:space:]]')" -gt 1 ]; then refuse \
"Command refused: one command per call, on one line, and you are $role.
Pass a multi-line text with --body-file - and a heredoc."
  fi
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
  # Padded with spaces so that a flag is matched as a whole word: " -s " is the
  # squash flag, not the "-s" inside "--state".
  padded=" $head_line "
  # Options that skip a confirmation, bypass the branch protection, delete
  # what was written, or hand the command to a browser or an editor that the
  # harness does not show. Checked BEFORE the allowlist: otherwise
  # "gh pr merge 3 --admin" would exit through the exit 0 of the
  # "gh pr merge" pattern without ever being examined.
  case "$padded" in
    *" --admin "*|*" --admin="*|*" --auto "*|*" --auto="*|*" --disable-auto "*|\
    *" --yes "*|*" -y "*|*" --force "*|*" --no-confirm "*|*" --delete-last "*|\
    *" --web "*|*" -w "*|*" --editor "*) refuse \
"Command refused: no option that bypasses a protection, skips a confirmation,
deletes a comment or opens a browser or an editor (--admin, --auto, --yes,
--force, --delete-last, --web, --editor), and you are $role.
What goes out is shown to the user as is, neither forced nor pre-confirmed." ;;
  esac
  case "$head_line" in
    "gh issue create"*|"gh issue comment"*|"gh issue edit"*|"gh pr create"*|"gh pr comment"*|"gh pr edit"*)
      case "$padded" in *" -e "*) refuse \
"Command refused: -e opens an editor the harness does not show, and you are $role.
Pass the text with --body-file - and a heredoc." ;;
      esac ;;
  esac
  # A "gh api" without a method or field is a GET; -f/-F would turn it into a POST.
  # Short flags are matched with an attached value and inside a group
  # (-fbody=x, -if), the way gh parses them.
  case "$head_line" in
    "gh api repos/{owner}/{repo}/issues/"*|"gh api repos/{owner}/{repo}/pulls/"*|\
    "gh api repos/{owner}/{repo}/milestones"*)
      if ! printf '%s' "$padded" | grep -Eq -- '[[:space:]]-[A-Za-z]*[fFX]|[[:space:]]--(method|field|raw-field|input)'; then
        exit 0
      fi ;;
  esac
  case "$role" in
    ticket-reader)
      case "$head_line" in
        "gh issue view "*|"gh issue list"*|"gh label list"*|\
        "git log"*|"git show"*|"git branch"*|"git status"*) exit 0 ;;
      esac ;;
    pr-reviewer)
      case "$padded" in
        *" --fix"*|*" --allow-dirty"*|*" --allow-staged"*|*" --bless"*|*" --write-changes"*) refuse \
"Command refused: this option rewrites files, and you are pr-reviewer.
You observe the branch; what must change goes back to the coder." ;;
      esac
      # The repository only accepts squash and rebase merges, and the merge is
      # tied to the head commit the checks ran on.
      case "$head_line" in
        "gh pr merge "*)
          case "$padded" in
            *" --merge "*|*" -m "*) refuse \
"Command refused: the repository accepts no merge commit, and you are pr-reviewer.
Merge with --squash or --rebase, the method the caller gave you." ;;
          esac
          case "$padded" in
            *" --squash "*|*" -s "*|*" --rebase "*|*" -r "*) ;;
            *) refuse \
"Command refused: gh pr merge needs an explicit --squash or --rebase, and you are pr-reviewer.
Without the caller's method, do not choose: ask." ;;
          esac
          case "$padded" in
            *" --match-head-commit "*|*" --match-head-commit="*) ;;
            *) refuse \
"Command refused: gh pr merge needs --match-head-commit <sha>, the commit your
checks ran on, and you are pr-reviewer. Without it, a push made after your
review would be merged unchecked." ;;
          esac ;;
      esac
      case "$head_line" in
        "gh pr view"*|"gh pr list"*|"gh pr diff"*|"gh pr checks"*|"gh pr comment "*|"gh pr merge "*|\
        "gh run list"*|"gh run view"*|"gh run watch"*|\
        "git log"*|"git diff"*|"git show"*|"git branch"*|"git status"*|"git merge-base"*|\
        "git rev-parse"*|"git symbolic-ref"*|"git fetch"*|\
        "just check"*|"just fmt-check"*|"just lint"*|"just test"*|"just doc"*|"just deny"*|\
        "just spell"*|"just rules"*|\
        "cargo fmt"*"--check"*|"cargo clippy"*|"cargo test"*|"cargo nextest run"*|"cargo build"*|\
        "cargo check"*|"cargo doc"*|"RUSTDOCFLAGS=\"-D warnings\" cargo doc"*|"RUSTDOCFLAGS='-D warnings' cargo doc"*|\
        "cargo deny check"*|"taplo fmt"*"--check"*|"typos"|"typos "*|"cog check"*|"cog verify"*) exit 0 ;;
      esac ;;
    pr-writer)
      case "$head_line" in
        "gh pr create"*)
          case "$padded" in
            *" --dry-run "*) refuse \
"Command refused: gh pr create --dry-run may still push the branch, and you are pr-writer." ;;
          esac
          case "$padded" in
            *" --head "*|*" -H "*|*" --head="*) ;;
            *) refuse \
"Command refused: gh pr create needs --head <branch>, and you are pr-writer.
Without it, gh offers to push the current branch: you push nothing." ;;
          esac ;;
      esac
      case "$head_line" in
        "gh pr view"*|"gh pr list"*|"gh pr create "*|"gh pr edit "*|"gh pr comment "*|\
        "git log"*|"git diff"*|"git show"*|"git branch"*|"git status"*|"git merge-base"*|\
        "git rev-parse"*) exit 0 ;;
      esac ;;
    ticket-writer)
      case "$padded" in
        *" --add-assignee"*|*" --remove-assignee"*|*" --assignee"*|*" -a "*) refuse \
"Command refused: assigning an issue is up to the user, and you are ticket-writer." ;;
      esac
      # The last method flag wins: a second one would turn the PATCH into a DELETE.
      case "$head_line" in
        "gh api -X PATCH "*)
          if printf ' %s' "${head_line#gh api -X PATCH }" | grep -Eq -- '[[:space:]]-[A-Za-z]*X|[[:space:]]--method'; then
            refuse "Command refused: a single method, PATCH, on an issue comment, and you are ticket-writer."
          fi ;;
      esac
      case "$head_line" in
        "gh issue view "*|"gh issue list"*|"gh issue create "*|"gh issue edit "*|"gh issue comment "*|\
        "gh label list"*|"gh label create "*|\
        "gh api -X PATCH repos/{owner}/{repo}/issues/comments/"*) exit 0 ;;
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
    release-engineer)  echo ".github/ (workflows, dependabot.yml), cog.toml, deny.toml, clippy.toml, rustfmt.toml, rust-toolchain.toml, .cargo/config.toml, justfile, taplo.toml, _typos.toml, .editorconfig" ;;
    devsecops)         echo ".claude/security/" ;;
    proof-engineer)    echo ".claude/proofs/, and the .lm sources under examples/ (specs only, by rule)" ;;
    perf-engineer)     echo ".claude/perf/, benches/ and crates/*/benches/" ;;
    language-designer) echo "docs/ (ADRs in docs/adr/, docs/design.md)" ;;
    pr-writer)         echo ".claude/tmp/ only (description drafts)" ;;
    ticket-writer)     echo ".claude/tmp/ only (description drafts)" ;;
    pr-reviewer)       echo ".claude/reviews/pr/" ;;
    ticket-reader)     echo "nothing: no file" ;;
  esac
}

in_zone() { # $1 = role, $2 = path relative to the project
  case "$1" in
    release-engineer)
      case "$2" in
        .github/*|cog.toml|deny.toml|clippy.toml|.clippy.toml|rustfmt.toml|.rustfmt.toml|\
        rust-toolchain|rust-toolchain.toml|.cargo/config.toml|justfile|taplo.toml|.taplo.toml|\
        _typos.toml|typos.toml|.typos.toml|.editorconfig) return 0 ;;
      esac ;;
    devsecops)
      case "$2" in .claude/security/*) return 0 ;; esac ;;
    proof-engineer)
      case "$2" in .claude/proofs/*|examples/*.lm) return 0 ;; esac ;;
    perf-engineer)
      case "$2" in .claude/perf/*|benches/*|crates/*/benches/*) return 0 ;; esac ;;
    language-designer)
      case "$2" in docs/*) return 0 ;; esac ;;
    pr-reviewer)
      case "$2" in .claude/reviews/pr/*) return 0 ;; esac ;;
    pr-writer|ticket-writer)
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
  # The code part only: `grep -n "cargo install" .github/workflows/ci.yml` installs nothing.
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
