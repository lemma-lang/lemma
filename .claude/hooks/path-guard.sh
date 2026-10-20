#!/usr/bin/env bash
#
# Path guard — PreToolUse hook.
#
# Refuses a write depending on the role requesting it. This is what turns into
# a barrier what was only a sentence in a prompt: the coder *cannot* touch the
# tests anymore, the reviewer *cannot* fix what it reviews anymore.
#
# Contract: the event arrives as JSON on stdin, with among others
#   .agent_type   the name of the calling role, absent for the main session
#   .tool_name    Write | Edit | NotebookEdit | Bash
#   .tool_input   .file_path for Write/Edit, .command for Bash
#   .cwd          the session's current directory — *not* the repository root
#
# Two bases, never to be confused:
#   .cwd      resolves a relative path, because that is what the tool does;
#   $project  anchors the allowed directories (.claude/...), because that is
#             where the tree the rules talk about lives.
# Exit: code 0 to let through, code 2 to refuse — the message carried on
# stderr is returned to the model, so it must tell it what to do instead.
# Any other exit code would let the tool call through: the EXIT trap below
# turns every unexpected status into a refusal.
#
# bash 3.2, that is what macOS ships.

set -uo pipefail

refuse() { printf '%s\n' "$1" >&2; exit 2; }

# Fail closed: a crash (unset variable, missing command) exits with 1, which
# the harness treats as "not blocking". A guard that breaks must refuse.
trap 'status=$?
if [ "$status" -ne 0 ] && [ "$status" -ne 2 ]; then
  printf "Path guard: internal error (status %s), refused as a precaution.\n" "$status" >&2
  exit 2
fi' EXIT

command -v jq >/dev/null 2>&1 || refuse \
  "Path guard: \"jq\" cannot be found, the guard cannot apply.
Every write is refused as long as it is missing — this is deliberate: a guard
that is absent without anyone knowing would be worse than a noisy guard.
Install jq, or remove the hook from .claude/settings.json knowingly."

payload=$(cat)

# An unreadable event must not silently disarm the guard: without this check,
# a parse error yields an empty role, an empty role passes for the main
# session, and everything is allowed without anything reporting it.
printf '%s' "$payload" | jq -e . >/dev/null 2>&1 || refuse \
"Path guard: unreadable PreToolUse event, write refused as a precaution.
A guard that opens on an input it does not understand guards nothing."

role=$(printf '%s' "$payload"  | jq -r '.agent_type // ""')
tool=$(printf '%s' "$payload"  | jq -r '.tool_name  // ""')
target=$(printf '%s' "$payload" | jq -r '.tool_input.file_path // ""')
cmd=$(printf '%s' "$payload"   | jq -r '.tool_input.command   // ""')
cwd=$(printf '%s' "$payload"   | jq -r '.cwd // ""')
cwd=${cwd%/}

# The main session is not a role: it orchestrates, restores, commits.
[ -n "$role" ] || exit 0

# --- The anchor: where the .claude/ the rules talk about lives ---------------
# 1. The Git toplevel of the cwd, when it holds a .claude/. This repository
#    commits its .claude/, so the toplevel is the right tree, worktrees
#    included, and a .claude/ created by accident in a parent directory (a
#    session once launched from ~/Documents/gitlab) cannot move the anchor.
# 2. Otherwise the *highest* ancestor holding a .claude/, below the home
#    directory (~/.claude is the user's configuration, never a project): the
#    first one could be the stray .claude/ that a misplaced write just created
#    deep in a subdirectory.
# 3. Otherwise CLAUDE_PROJECT_DIR, which settings.json uses to locate this
#    script — a fallback only, it says nothing about the tree being written.
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

# Without an anchor, no directory rule makes sense. We refuse rather than
# fall back on the cwd: that very fallback is what made reports get written
# into a stray .claude/ deep in a subdirectory.
[ -n "$project" ] || refuse \
"Path guard: unable to locate the project root from \"$cwd\".
Neither the Git toplevel, nor any parent directory, nor CLAUDE_PROJECT_DIR holds
a .claude/. Refused: a guard that does not know where the allowed directories
are guards nothing."

# --- What counts as a test path ----------------------------------------------
# The defaults cover the widespread conventions. This repository sets its own
# patterns in .claude/test-paths.txt — they replace the defaults rather than
# adding to them, so that one can also *restrict*.
default_patterns='tests.rs
*/test/*
*/tests/*
*/__tests__/*
*_test.*
*-test.*
*.test.*
*.spec.*
*-spec.*
test_*.py
conftest.py'

patterns="$default_patterns"
if [ -f "$project/.claude/test-paths.txt" ]; then
  custom=$(grep -v '^[[:space:]]*\(#\|$\)' "$project/.claude/test-paths.txt" 2>/dev/null)
  [ -n "$custom" ] && patterns="$custom"
fi

# A relative path is resolved by the tool from the cwd, never from the
# root — the guard must judge the file that will actually be written.
# A leading "./" is stripped: without that the prefix comparison would refuse
# a perfectly legitimate path. We go no further — canonicalizing ".." is a
# whole piece of work; we refuse them and say so, that is honest and done.
absolute() { # $1 = path as the tool received it
  path="$1"
  while :; do case "$path" in ./*) path=${path#./} ;; *) break ;; esac; done
  case "$path" in /*) printf '%s' "$path" ;; *) printf '%s/%s' "$cwd" "$path" ;; esac
}

# Patterns read like .gitignore: one without a "/" is matched against the file
# name, wherever the file sits (`tests.rs`); one with a "/" is matched against
# the path relative to the project, with and without a leading "/", so that
# `*/fuzz/*` also matches `fuzz/` at the root. Never against the absolute
# path: a project cloned under a directory named `tests` would make every file
# a test.
is_a_test() { # $1 = absolute path
  path="$1"
  rel=${path#"$project"/}
  name=${path##*/}
  while IFS= read -r pattern; do
    [ -n "$pattern" ] || continue
    # shellcheck disable=SC2254
    case "$pattern" in
      */*) case "$rel"  in $pattern) return 0 ;; esac
           case "/$rel" in $pattern) return 0 ;; esac ;;
      *)   case "$name" in $pattern) return 0 ;; esac ;;
    esac
  done <<EOF
$patterns
EOF
  return 1
}

under_dir() { # $1 = absolute path, $2 = expected directory, relative to the project
  case "$1" in "$project/$2"*) return 0 ;; esac
  return 1
}

# The files that decide what the build and the CI accept: the workflows, lints,
# formatting, commit rules, dependency policy, the justfile the CI runs. They belong to the release-engineer; a
# coder that loosens one turns a red check green without touching the code.
is_build_policy() { # $1 = path relative to the project
  case "$1" in
    .github/*|cog.toml|deny.toml|clippy.toml|.clippy.toml|rustfmt.toml|.rustfmt.toml|\
    rust-toolchain|rust-toolchain.toml|.cargo/config.toml|justfile|taplo.toml|.taplo.toml|\
    _typos.toml|typos.toml|.typos.toml|.editorconfig) return 0 ;;
  esac
  return 1
}

# The refusal must be verifiable by whoever receives it. ".claude/reviews/X.md
# is outside .claude/reviews/" is an absurdity from the model's point of view: it
# retries with another relative variant, and the report lands beside the mark
# again. So we show both absolute paths, the one that would have been written
# and the right one.
outside_notice() { # $1 = path received, $2 = absolute path, $3 = expected directory, $4 = who you are
  printf 'Write refused: "%s" is outside %s, and you are %s.\n' "$1" "$project/$3" "$4"
  [ "$1" = "$2" ] || printf 'This path is relative: it would be written to "%s".\n' "$2"
  printf 'Expected path: %s%s\n' "$project/$3" "${2##*/}"
}

# --- Direct writes -----------------------------------------------------------

if [ "$tool" = "Write" ] || [ "$tool" = "Edit" ] || [ "$tool" = "NotebookEdit" ]; then
  [ -n "$target" ] || exit 0
  abs=$(absolute "$target")
  rel=${abs#"$project"/}

  case "$abs" in
    */../*|*/..) refuse \
"Write refused: \"$target\" contains \"..\", and the guard does not interpret it.
A path that walks up can leave your zone without looking like it, and the guard
refuses rather than guess. Give the path again in plain form, from the project
root: $project/…" ;;
  esac

  # Some writes would let a role rewrite the guard's own settings, and no role
  # makes them, whoever it is: this block runs before the per-role rules, so
  # it also covers the roles this script otherwise leaves to role-guard.sh.
  # `test-paths.txt` replaces the test patterns: a coder that puts a pattern
  # matching nothing in it can then rewrite every test.
  case "$abs" in
    */.claude/test-paths.txt) refuse \
"Write refused: \"$abs\" tells the guard what counts as a test, and no role
modifies it. If the patterns look wrong to you, say which ones and why in your
report: the user fixes them." ;;
  esac
  # The same goes for the rest of the setup: the guards themselves, the
  # settings that declare them, the role prompts, the shape rules, the policy,
  # project.conf (the ticket-brancher sources it with `.` and would run it),
  # and CLAUDE.md / AGENTS.md, which the main session loads as its instructions.
  case "$abs" in
    */.claude/settings.json|*/.claude/settings.local.json|*/.claude/hooks/*|\
    */.claude/agents/*|*/.claude/skills/*|*/.claude/tools/*|\
    */.claude/rules|*/.claude/rules/*|\
    */.claude/orchestration.md|*/.claude/project.conf|\
    "$project/CLAUDE.md"|"$project/AGENTS.md") refuse \
"Write refused: \"$abs\" belongs to the setup that runs the roles and their
guards, or to the instructions the main session runs under, and no role
modifies it. If it looks wrong to you, say what and why in your report: the
user fixes it." ;;
  esac
  case "$abs" in
    */.claude/*)
      case "$project/" in "${abs%%/.claude/*}"/?*) refuse \
"Write refused: \"$abs\" would create a .claude/ above the project
($project). No role writes there. If you need a file outside the project, say
so in your report." ;;
      esac ;;
  esac

  case "$role" in
    coder)
      is_a_test "$abs" && refuse \
"Write refused: \"$abs\" is a test file, and you are the coder.
This rule holds the whole TDD loop together — the tests are the specification,
and a coder who adjusts them turns a wrong test green without anyone
seeing it. If you believe this test is wrong, stop and report it, citing what
is off: it is the tester who will be relaunched to fix it."
      under_dir "$abs" ".claude/plans/" && refuse \
"Write refused: \"$abs\" is the plan, and you are the coder.
The plan is your instruction, not your scratchpad — a coder who rewrites it makes
the gap between what was planned and what was done disappear, and the
final review has nothing left to compare against. If the plan seems
wrong or incomplete to you, stop and say how: it is the planner who will be
relaunched."
      is_build_policy "$rel" && refuse \
"Write refused: \"$rel\" decides what the build and the CI accept (lints,
formatting, commits, dependencies), and you are the coder. Loosening it would
turn a red check green without fixing the code. If it must change, say what
and why in your report: the release-engineer applies it."
      case "$rel" in benches/*|*/benches/*) refuse \
"Write refused: \"$rel\" is a benchmark, and you are the coder. Benchmarks are
the measuring instrument of the perf-engineer; a coder who tunes them measures
its own work. Report what the benchmark should cover instead." ;;
      esac
      case "$rel" in docs/adr/*|docs/design.md) refuse \
"Write refused: \"$rel\" records the language design and its decisions, and
you are the coder. The language-designer keeps it. Say in your report what the
change alters in the design: it will be updated in the same ticket." ;;
      esac
      ;;
    tester)
      is_a_test "$abs" || refuse \
"Write refused: \"$abs\" is not a test file, and you are the tester.
You do not write production code, not even one line, not even \"so that it
compiles\" — and not the \`#[cfg(test)] mod tests;\` line either, nor a
dev-dependency: say in your report which declaration your tests need, the
main session adds it before the red is confirmed."
      ;;
    planner)
      under_dir "$abs" ".claude/plans/" || refuse \
"$(outside_notice "$target" "$abs" ".claude/plans/" "the planner")
Your product is a plan that others will execute. What you wanted to fix
along the way belongs in your plan, as a step."
      ;;
    debugger)
      under_dir "$abs" ".claude/diagnostics/" || refuse \
"$(outside_notice "$target" "$abs" ".claude/diagnostics/" "the debugger")
You establish a cause, you do not fix it — a debugger that repairs becomes the role
that gets called when the normal path gets stuck. If a line of instrumentation is
needed, say which one and where: someone will add it and relaunch you."
      ;;
    reviewer)
      under_dir "$abs" ".claude/reviews/" || refuse \
"$(outside_notice "$target" "$abs" ".claude/reviews/" "the reviewer")
A reviewer who fixes becomes the judge of its own work on the next
pass, and the independent check disappears without anyone
noticing. Write the finding out in full: someone else will apply
the fix in ten seconds."
      ;;
  esac
  exit 0
fi

# --- Writes through the shell ------------------------------------------------
# A speed bump, not a wall — and that must be known. Listing the ways to
# write a file under Unix is a lost cause from the start; this filter
# only catches the obvious forms. The real guarantee remains the diff of the
# test files against their reference commit, which the exit gate
# checks.

[ "$tool" = "Bash" ] && [ -n "$cmd" ] || exit 0

# What the shell runs as commands, as opposed to the data it carries: a
# heredoc body (a commit message, an issue description) and a quoted string
# (a grep pattern, a -m message) are dropped, so that a commit saying "apply
# cargo insta accept" is not mistaken for running it. An environment
# assignment keeps its value (INSTA_UPDATE="always" becomes INSTA_UPDATE=always).
# A speed bump like the rest: `"cargo" insta accept` slips through.
code_of() { # $1 = command
  printf '%s\n' "$1" | awk '{ print } /<</ { exit }' \
    | sed -E -e "s/([A-Za-z_][A-Za-z0-9_]*)=[\"']([^\"' ]*)[\"']/\\1=\\2/g" \
             -e "s/'[^']*'//g" -e 's/"[^"]*"//g'
}
code=$(code_of "$cmd")

# ast-grep rewrites a whole tree in a single command (-U, --update-all), or
# file by file without any path showing in the command (-i, --interactive).
# No role has a use for it: they search, and those that write go through
# Edit, where each path is judged. Mass rewriting stays with the main
# session, the one that runs cleanups.
# Only the call itself is examined, up to the first |, ; or &: otherwise a
# downstream "| grep -i" passed for the interactive option.
if printf '%s' "$code" \
      | grep -Eo '(^|[;&|(`[:space:]])(ast-grep|sg)[[:space:]][^|;&]*' \
      | grep -Eq '[[:space:]](-[A-Za-z]*[Ui][A-Za-z]*|--update-all|--interactive)([[:space:]=]|$)'; then
  refuse \
"Command refused: ast-grep with -U or -i rewrites files without the guard
seeing which ones, and you are $role.
    $cmd
Without -U, \"ast-grep -p … -r …\" shows the diff it would make: use it to
look, then apply with Edit file by file if your role writes. A mass rewrite
is done in the main session."
fi

# Blessing snapshots rewrites the expected outputs — test files — in one
# command that names none of them, and it carries no write token (no ">", no
# "mv") for the filter further down to see. Only the tester, who owns the
# snapshots, accepts them.
if [ "$role" != "tester" ]; then
  blesses=0
  printf '%s' "$code" | grep -Eq 'cargo[[:space:]]+insta[[:space:]]+(accept|review|reject)' && blesses=1
  printf '%s' "$code" | grep -Eq 'cargo[[:space:]]+insta[[:space:]]+test.*(--accept|--review|--force-update-snapshots)' && blesses=1
  printf '%s' "$code" | grep -Eq '(INSTA_FORCE_PASS|INSTA_FORCE_UPDATE|UPDATE_EXPECT|UPDATE_GOLDEN|BLESS)=' && blesses=1
  printf '%s' "$code" | grep -Eq '(TRYBUILD|SNAPSHOTS)=overwrite|--bless([[:space:]]|$)' && blesses=1
  if printf '%s' "$code" | grep -Eq 'INSTA_UPDATE=' \
     && ! printf '%s' "$code" | grep -Eq 'INSTA_UPDATE=no([[:space:]]|$)'; then
    blesses=1
  fi
  [ "$blesses" -eq 0 ] || refuse \
"Command refused: it accepts or rewrites snapshot / golden files, and you are $role.
    $cmd
Expected outputs are tests: only the tester accepts a new snapshot, after
checking it. Run the tests without the update option and report the diff."
fi

if [ "$role" = "coder" ]; then
  # `2>&1`, `>&2`, `1>&2` duplicate a descriptor: no file is written.
  # Leaving them in the examined string made the outer filter bite on the
  # chevron, and every *read* command ending in `2>&1` and mentioning a
  # test path was refused: running a test file, displaying it, searching
  # it. We strip them before examination, the real redirection to a file
  # (`> f`, `>> f`, `&> f`) remaining intact. Same for `> /dev/null`, and for
  # Rust's `->` and `=>`, which a search for a signature or a match arm
  # carries without writing anything.
  # The write token is looked for in the code part only (a ">" inside a grep
  # pattern writes nothing); the target, in the whole command, since a path
  # written to can be quoted.
  strip_reads() { sed -e 's/[0-9]*>&[0-9-]*//g' -e 's#[0-9&]*>[[:space:]]*/dev/null##g' -e 's/->//g' -e 's/=>//g'; }
  # A setup script in command position is run, not written (the coder runs check.sh
  # before handing in); dropping its name leaves a real write into .claude/ in view.
  strip_runs() {
    awk '{ if (sub(/\\$/, "")) printf "%s ", $0; else print }' \
      | sed -E 's#(^|[;(]|&&|\|\||[^>]\||[^>]&)([[:space:]]*)(([A-Za-z_][A-Za-z0-9_]*=[A-Za-z0-9_./~$:-]*[[:space:]]+)*)("?[A-Za-z0-9_./~$-]*/)?\.claude/(rules|tools|hooks/tests)/[A-Za-z0-9_.-]+\.sh"?([[:space:];&|)]|$)#\1\2\3\7#g'
  }
  cmd_examined=$(printf '%s' "$cmd" | strip_reads | strip_runs)
  code_examined=$(printf '%s' "$code" | strip_reads)
  case "$code_examined" in
    *sed\ -i*|*tee\ *|*\>*|*mv\ *|*cp\ *|*rm\ *|*truncate*|*patch\ *)
      # The plan must remain *readable* by the coder — the role tells it to
      # open it. So we only target the destination position: after a
      # redirection, or as an argument of a command that writes or destroys.
      # `cp` is deliberately absent: it can only copy the plan
      # elsewhere, never damage it.
      case "$cmd_examined" in
        *\>*.claude/plans/*|*sed\ -i*.claude/plans/*|*tee\ *.claude/plans/*|\
        *truncate*.claude/plans/*|*patch\ *.claude/plans/*|\
        *mv\ *.claude/plans/*|*rm\ *.claude/plans/*)
          refuse \
"Command refused: it appears to write to the plan, and you are the coder.
    $cmd
Going through the shell does not change the rule. If the plan seems wrong to you,
report it instead of rewriting it — it is the planner who will fix it."
          ;;
        *test-paths.txt*)
          refuse \
"Command refused: it appears to write to .claude/test-paths.txt, which tells
the guard what counts as a test, and you are the coder.
    $cmd
If the patterns look wrong to you, say so in your report: the user fixes them."
          ;;
        *.claude/settings.json*|*.claude/settings.local.json*|*.claude/hooks/*|\
        *.claude/agents/*|*.claude/skills/*|*.claude/tools/*|*.claude/rules*|\
        *.claude/orchestration.md*|*.claude/project.conf*|*CLAUDE.md*|*AGENTS.md*)
          refuse \
"Command refused: it appears to write to the setup that runs the roles and
their guards, and you are the coder.
    $cmd
If it looks wrong to you, say what and why in your report: the user fixes it."
          ;;
        *.github/*|*cog.toml*|*deny.toml*|*clippy.toml*|*rustfmt.toml*|\
        *rust-toolchain*|*.cargo/config.toml*|*justfile*|*taplo.toml*|*typos.toml*|*.editorconfig*)
          refuse \
"Command refused: it appears to write to a build or CI policy file, and you are the coder.
    $cmd
If it must change, say what and why in your report: the release-engineer applies it."
          ;;
      esac
      while IFS= read -r pattern; do
        [ -n "$pattern" ] || continue
        # A relative path carries no leading "/": `cp a tests/x.rs` must be
        # caught by `*/tests/*` as surely as `cp a ./tests/x.rs`.
        bare=${pattern#\*/}
        # shellcheck disable=SC2254
        case "$cmd_examined" in $pattern*|*\ $pattern*|*/$pattern*|$bare*|*\ $bare*)
          refuse \
"Command refused: it appears to write to a test path, and you are the coder.
    $cmd
Going through the shell does not change the rule. If you believe a test is wrong,
report it instead of working around it — it is the tester who will fix it."
          ;;
        esac
      done <<EOF
$patterns
EOF
      ;;
  esac
fi

exit 0
