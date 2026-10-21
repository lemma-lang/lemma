#!/usr/bin/env bash
#
# Feeds sample PreToolUse events to both guards, the way the harness does, and
# checks the outcome. Rerun after any change to the hooks, test-paths.txt or
# the wiring: it is the only way to tell a guard that protects from one that
# no longer exists.
#
#   .claude/hooks/tests/run.sh
#
# Each guard must exit with exactly 0 (allow) or 2 (refuse): any other status
# would let the tool call through, and is a failure here.
#
# bash 3.2.

set -uo pipefail

# Run from a git hook (pre-push), GIT_DIR points at the outer repository and
# overrides the scratch repositories these cases create.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX

here=$(cd "$(dirname "$0")" && pwd -P)
hooks=$(cd "$here/.." && pwd -P)
root=$(cd "$hooks/../.." && pwd -P)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

pass=0
failed=0

# $1 = role ("" for the main session), $2 = tool, $3 = path or command, $4 = cwd
event() {
  if [ "$2" = "Bash" ]; then
    jq -n --arg r "$1" --arg c "$3" --arg d "$4" \
      '{tool_name: "Bash", tool_input: {command: $c}, cwd: $d} + (if $r == "" then {} else {agent_type: $r} end)'
  else
    jq -n --arg r "$1" --arg t "$2" --arg f "$3" --arg d "$4" \
      '{tool_name: $t, tool_input: {file_path: $f, content: "x"}, cwd: $d} + (if $r == "" then {} else {agent_type: $r} end)'
  fi
}

# Runs both guards on a payload; prints "allow" or "deny", or "broken:<hook>:<status>".
verdict() { # $1 = payload, extra env in the caller
  local outcome=allow hook status
  for hook in path-guard.sh role-guard.sh; do
    printf '%s' "$1" | "$hooks/$hook" >/dev/null 2>"$tmp/stderr"
    status=$?
    case "$status" in
      0) ;;
      2) outcome=deny ;;
      *) printf 'broken:%s:%s' "$hook" "$status"; return ;;
    esac
  done
  printf '%s' "$outcome"
}

check() { # $1 = expected, $2 = description, $3 = payload
  local got
  got=$(verdict "$3")
  if [ "$got" = "$1" ]; then
    pass=$((pass + 1))
  else
    failed=$((failed + 1))
    printf 'FAIL  %-6s got %-6s %s\n' "$1" "$got" "$2"
  fi
}

case_() { # $1 = expected, $2 = role, $3 = tool, $4 = path or command, [$5 = cwd]
  local cwd=${5:-$root}
  check "$1" "${2:-main}: $3 $4" "$(event "$2" "$3" "$4" "$cwd")"
}

R=$root

# --- The TDD loop -------------------------------------------------------------
case_ allow ""     Write "$R/crates/lemma-codegen/src/tests.rs"
case_ deny  coder  Write "$R/crates/lemma-codegen/src/tests.rs"
case_ allow tester Write "$R/crates/lemma-codegen/src/tests.rs"
case_ allow coder  Write "$R/crates/lemma-codegen/src/lib.rs"
case_ deny  tester Write "$R/crates/lemma-codegen/src/lib.rs"
case_ deny  coder  Edit  "$R/crates/lemma-syntax/src/parser/tests.rs"
case_ deny  coder  Write "$R/crates/lemma-syntax/tests/parse.rs"
case_ allow tester Write "$R/crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Write "$R/crates/lemma-codegen/snapshots/dot.snap"
case_ deny  coder  Write "$R/crates/lemma-verify/testdata/vacuous.lm"
case_ deny  coder  Write "$R/fuzz/fuzz_targets/parse.rs"
case_ allow tester Write "$R/fuzz/fuzz_targets/parse.rs"
case_ deny  coder  Write "src/tests.rs" "$R/crates/lemma-codegen"
case_ deny  coder  Write "$R/crates/lemma-codegen/../lemma-codegen/src/tests.rs"
case_ deny  coder  Write "$R/.claude/plans/LEM-1.md"
case_ allow planner Write "$R/.claude/plans/LEM-1.md"
case_ deny  planner Write "$R/crates/lemma-syntax/src/lib.rs"
case_ allow reviewer Write "$R/.claude/reviews/LEM-1.md"
case_ deny  reviewer Edit "$R/crates/lemma-syntax/src/lib.rs"
case_ allow debugger Write "$R/.claude/diagnostics/z3-timeout.md"
case_ deny  debugger Write "$R/crates/lemma-verify/src/lib.rs"

# --- What the coder may not loosen -------------------------------------------
case_ deny  coder Write "$R/clippy.toml"
case_ deny  coder Edit  "$R/.github/workflows/ci.yml"
case_ deny  coder Write "$R/.github/dependabot.yml"
case_ deny  coder Edit  "$R/justfile"
case_ deny  coder Edit  "$R/taplo.toml"
case_ deny  coder Edit  "$R/_typos.toml"
case_ deny  coder Edit  "$R/.editorconfig"
case_ deny  coder Bash  "echo '  - run: true' >> .github/workflows/ci.yml"
case_ deny  coder Bash  "sed -i '' 's/check:/check: fmt/' justfile"
case_ deny  coder Edit  "$R/deny.toml"
case_ deny  coder Edit  "$R/.cargo/config.toml"
case_ deny  coder Write "$R/crates/lemma-codegen/benches/dot.rs"
case_ deny  coder Edit  "$R/docs/adr/0002-scope.md"
case_ deny  coder Edit  "$R/docs/design.md"
case_ allow coder Edit  "$R/README.md"
case_ allow coder Edit  "$R/examples/dot.lm"
case_ allow coder Edit  "$R/Cargo.toml"

# --- The setup itself ---------------------------------------------------------
case_ deny  coder Write "$R/.claude/test-paths.txt"
case_ deny  tester Write "$R/.claude/test-paths.txt"
case_ deny  release-engineer Edit "$R/.claude/hooks/path-guard.sh"
case_ deny  coder Edit  "$R/.claude/settings.json"
case_ deny  coder Edit  "$R/.claude/agents/coder.md"
case_ deny  coder Edit  "$R/CLAUDE.md"
case_ deny  language-designer Edit "$R/AGENTS.md"
case_ allow ""    Edit  "$R/.claude/hooks/path-guard.sh"

# --- Shell: the coder ---------------------------------------------------------
case_ deny  coder  Bash "cargo insta accept"
case_ deny  coder  Bash "cargo insta review"
case_ deny  coder  Bash "cargo insta test --accept"
case_ deny  coder  Bash "INSTA_UPDATE=always cargo test -p lemma-codegen"
case_ deny  coder  Bash "INSTA_FORCE_PASS=1 cargo test"
case_ deny  coder  Bash "UPDATE_EXPECT=1 cargo test"
case_ deny  coder  Bash "BLESS=1 cargo test"
case_ deny  coder  Bash "TRYBUILD=overwrite cargo test"
case_ allow coder  Bash "INSTA_UPDATE=no cargo test"
case_ allow tester Bash "cargo insta accept"
case_ allow tester Bash "INSTA_UPDATE=always cargo test -p lemma-codegen"
case_ deny  debugger Bash "cargo insta accept"
case_ allow coder  Bash "cargo test --workspace 2>&1 | tail -20"
# `>&` duplicates a descriptor only when a number or `-` follows; `>&word` writes a file.
case_ allow coder  Bash "echo done >&2"
case_ allow coder  Bash "cargo build 2>&- | tail -5"
case_ allow coder  Bash "cargo test 3>&1 1>&2 2>&3 | tail -5"
case_ deny  coder  Bash "echo pwned >&.claude/rules/check.sh"
case_ deny  coder  Bash "echo x >&crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "echo x >&2crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "p=/../crates/lemma-syntax/tests/parse.rs; echo x >&2\$p"
case_ deny  coder  Bash "echo x >&2\\/../crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "echo x >&2\"/../crates/lemma-syntax/tests/parse.rs\""
case_ deny  coder  Bash "echo x ->crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "echo a=>crates/lemma-syntax/tests/parse.rs"
case_ allow coder  Bash "grep -rn 'Some(x) => x' crates/lemma-syntax/tests/"
case_ deny  coder  Bash "echo x ->.claude/plans/LEM-1.md"
case_ deny  coder  Bash "echo x =>.claude/plans/LEM-1.md"
case_ deny  coder  Bash "git checkout -- \".\""
case_ deny  coder  Bash "git checkout abc123 -- '.'"
case_ deny  coder  Bash "\"\"gh issue list"
case_ deny  coder  Bash "git \"\"switch main"
case_ deny  coder  Bash "cargo insta \"\"accept"
case_ deny  devsecops Bash "echo x >&2\"deny.toml\""
case_ allow coder  Bash "grep -n 'fn parse() -> Ast' crates/lemma-syntax/src/tests.rs 2>/dev/null"
case_ allow coder  Bash "cargo fmt --all"
case_ deny  coder  Bash "echo x > crates/lemma-codegen/src/tests.rs"
case_ deny  coder  Bash "sed -i '' 's/a/b/' crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "cp /tmp/x.rs crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "echo x >> .claude/plans/LEM-1.md"
case_ deny  coder  Bash "echo '[build]' >> .cargo/config.toml"
# A setup script runs as a command of its own; any write next to a setup path is refused.
case_ allow coder  Bash ".claude/rules/check.sh HEAD"
case_ allow coder  Bash "LC_ALL=C .claude/rules/check.sh HEAD 2>&1 | tail -20"
case_ allow coder  Bash "$R/.claude/tools/frozen-check.sh abc123"
case_ deny  coder  Bash "sed -i '' 's/a/b/' crates/lemma-codegen/src/lib.rs && .claude/rules/check.sh HEAD"
case_ deny  coder  Bash ".claude/tools/status.sh > /tmp/status.txt"
case_ deny  coder  Bash "echo x >> .claude/rules/check.sh"
case_ deny  coder  Bash "cp /tmp/x.sh .claude/tools/status.sh"
case_ deny  coder  Bash "sed -i '' 's/a/b/' .claude/tools/frozen-check.sh"
case_ deny  coder  Bash ".claude/rules/check.sh HEAD > .claude/rules/report.txt"
case_ deny  coder  Bash "echo x | tee .claude/hooks/tests/run.sh"
case_ deny  coder  Bash "echo pwned >| .claude/rules/check.sh"
case_ deny  coder  Bash "echo pwned &>./.claude/tools/frozen-check.sh"
case_ deny  coder  Bash ">./.claude/tools/frozen-check.sh echo 'exit 0'"
case_ deny  coder  Bash ">$R/.claude/rules/check.sh cat <<'EOF'
exit 0
EOF"
case_ deny  coder  Bash "cp /tmp/evil.sh \\
.claude/rules/check.sh"
case_ deny  coder  Bash "echo x > \\
.claude/tools/status.sh"
case_ deny  coder  Bash "rm -f \\
{crates/lemma-syntax/tests/parse.rs,/.claude/rules/x.sh}"
case_ deny  coder  Bash ".claude/rules/check.sh.bak > .claude/rules/check.sh"
case_ deny  coder  Bash "xargs cp /tmp/evil.sh <<'EOF'
.claude/rules/check.sh
EOF"
case_ deny  coder  Bash "t=(.claude/rules/check.sh); cp /tmp/evil.sh \"\${t[0]}\""
case_ deny  coder  Bash "xargs cp /tmp/evil.sh <<\\EOF
.claude/rules/check.sh
EOF"
case_ deny  coder  Bash "xargs cp /tmp/evil.sh <<<x <<EOF
.claude/rules/check.sh
EOF"
case_ deny  coder  Bash "xargs cp /tmp/evil.sh <<A <<B
A
.claude/rules/check.sh
B"
case_ deny  coder  Bash "xargs -I{} cp /tmp/evil.sh {} <<EOF
	EOF
.claude/rules/check.sh
EOF"
case_ deny  coder  Bash "xargs -I{} cp /tmp/evil.sh {} <<EOF.x
EOF
.claude/rules/check.sh
EOF.x"
case_ deny  coder  Bash "printf '%s\\n' \"(.claude/rules/check.sh\" | tr -d '(' | xargs cp /tmp/evil.sh"
case_ deny  coder  Bash "echo x > crates/a.rs; .claude/rules/check.sh HEAD; cp /tmp/y .claude/rules/sgconfig.yml"
case_ deny  coder  Bash "ast-grep -p 'x.unwrap()' -r 'x?' -U crates/"
case_ allow coder  Bash "ast-grep -l rust -p '\$X.unwrap()' crates/ | grep -i lemma"

# --- Shell: read-only roles ---------------------------------------------------
case_ deny  reviewer Bash "cargo fmt --all"
case_ allow reviewer Bash "cargo fmt --all --check"
case_ deny  reviewer Bash "cargo clippy --fix --allow-dirty"
case_ allow reviewer Bash "cargo clippy --workspace --all-targets -- -D warnings"
case_ deny  auditor  Bash "cargo add serde"
case_ deny  devsecops Bash "cargo install cargo-audit"
case_ allow devsecops Bash "cargo deny check"
case_ allow devsecops Bash "cargo update --dry-run"
case_ deny  devsecops Bash "cargo update"
case_ allow devsecops Bash "cargo deny check 2>&1 | tee .claude/security/deny.log"
case_ deny  devsecops Bash "echo x > deny.toml"
case_ allow devsecops Bash "cargo deny check >&2"
case_ deny  devsecops Bash "echo x >&deny.toml"

# --- Prose is data: heredoc bodies and quoted strings are not commands ------------
case_ allow committer Bash $'git commit -F - <<\'EOF\'\nstyle: apply cargo fmt\n\nCo-Authored-By: Claude <noreply@anthropic.com>\nEOF'
case_ allow committer Bash "git commit -m \"ci: install cargo-deny with --locked\""
case_ deny  committer Bash "git commit -m \"style: format\" && cargo fmt --all"
case_ allow ticket-writer Bash $'gh issue create --title "Snapshot review" --body-file - <<\'EOF\'\nRun cargo insta accept only after reading each snapshot.\nEOF'
case_ allow devsecops Bash "grep -n \"cargo install\" .github/workflows/ci.yml"
case_ allow reviewer Bash "grep -rn \"cargo add\" crates"
case_ allow planner Bash "grep -rn 'gh issue' .github/workflows/ci.yml"
case_ allow planner Bash "grep -rn high crates"
case_ allow planner Bash "ls .github/workflows"
case_ deny  coder Bash "INSTA_UPDATE=\"always\" cargo test"
case_ allow coder Bash "INSTA_UPDATE='no' cargo test"
case_ allow coder Bash "grep -n \"a > b\" crates/lemma-syntax/src/tests.rs"
case_ deny  coder Bash "echo x > \"crates/lemma-syntax/src/tests.rs\""

# --- A heredoc ends at its terminator: the lines after it are commands --------
case_ deny  coder  Bash $'true <<EOF\nEOF\necho x > crates/lemma-syntax/src/tests.rs'
case_ deny  coder  Bash $'cat <<-EOF\n\tx\n\tEOF\ncp /tmp/evil.sh .claude/rules/check.sh'
case_ deny  coder  Bash $'cat <<A <<\'B\'\na\nA\nb\nB\necho x > crates/lemma-syntax/src/tests.rs'
case_ deny  coder  Bash $'echo hi # <<X\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'echo "<<X"\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'echo $((1<<X))\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'true <<EOF\nEOF\ngh issue close 3'
case_ deny  devsecops Bash $'cat <<EOF\nx\nEOF\necho x > deny.toml'
case_ deny  proof-engineer Bash $'cat <<EOF\nx\nEOF\necho x > crates/lemma-verify/src/lib.rs'
case_ deny  perf-engineer Bash $'cat <<EOF\nx\nEOF\necho x > crates/lemma-jit/src/lib.rs'
case_ deny  language-designer Bash $'cat <<EOF\nx\nEOF\necho x > crates/lemma-syntax/src/lib.rs'
case_ deny  ticket-reader Bash $'gh issue view 3 <<EOF\nEOF\ngh issue close 3'
case_ deny  ticket-writer Bash $'gh issue comment 3 --body-file - <<\'EOF\'\nx\nEOF\ngh issue close 3'
case_ deny  pr-writer Bash $'gh pr comment 7 --body-file - <<\'EOF\'\nx\nEOF\ngh pr merge 7 --squash'
case_ deny  coder  Bash $'cat <<E\"O\"F\nx\nEOF\necho x > crates/lemma-syntax/src/tests.rs\nE'
case_ deny  coder  Bash $'cat <<\'EO\'F\nx\nEOF\necho x > crates/lemma-syntax/src/tests.rs\nEO'
case_ deny  coder  Bash $'cat <<E\\OF\nx\nEOF\necho x > crates/lemma-syntax/src/tests.rs\nE'
case_ deny  coder  Bash $'cat <<E/F\nx\nE/F\necho x > crates/lemma-syntax/src/tests.rs\nE'
case_ deny  coder  Bash $'cat <<-$X\nx\n\necho x > crates/lemma-syntax/src/tests.rs\n$X'
case_ deny  coder  Bash $'echo \\<<X\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'echo $(( (1) << X ))\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'echo ${x:-<<X}\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'echo $[1<<X]\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'(true)#<<X\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'echo \"a\n<<X\n\"\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'git commit -m \"$(cat <<\'EOF\'\nx\nEOF\n)\"\necho x > crates/lemma-syntax/src/tests.rs'
case_ deny  coder  Bash $'echo $\'\\\' <<X \'\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'cat <<E\\\nOF\nEOF\necho x > crates/lemma-syntax/src/tests.rs\nE'
case_ deny  coder  Bash $'echo ${x:-\"}\"}\necho \"<<X \"\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'echo \"$(case x in a) echo \"<<X \";; esac)\"\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'cat <<X\nX\necho x > crates/lemma-syntax/src/tests.rs\nX'
case_ deny  coder  Bash $'cat <<EOF\n$(echo x > crates/lemma-syntax/src/tests.rs)\nEOF'
case_ allow coder  Bash $'cat <<\'EOF\'\n$(echo x > crates/lemma-syntax/src/tests.rs)\nEOF'
case_ allow coder  Bash $'cat <<EOF\ncargo insta accept\nEOF'
case_ deny  committer Bash $'git commit -m "$(cat <<\'EOF\'\n)\n$(gh issue close 3)\nEOF\n)"'

# --- Paths are compared collapsed and without case, links are writes ----------
case_ deny  coder  Write "$R/.claude//hooks/path-guard.sh"
case_ deny  coder  Write "$R/./.github/workflows/ci.yml"
case_ deny  coder  Write "$R//justfile"
case_ deny  coder  Write "$R/./docs/adr/0002-scope.md"
case_ deny  coder  Write "$R/crates/lemma-diagnostics/src/Tests.rs"
case_ deny  coder  Write "$R/.claude/Hooks/path-guard.sh"
case_ deny  coder  Write "$R/Justfile"
case_ deny  coder  Write "$R/Claude.md"
case_ deny  coder  Write "$R/crates/lemma-syntax/./tests/parse.rs"
case_ deny  tester Write "$R/crates/lemma-syntax/src//lib.rs"
case_ deny  devsecops Write "$R/.claude/Security/../../deny.toml"
case_ deny  devsecops Write "$R//deny.toml"
case_ deny  devsecops Bash "echo x > .claude/security/../../deny.toml"
case_ deny  devsecops Bash "cargo deny check | tee .claude/security/../../deny.toml"
case_ deny  coder  Bash "ln -s ../../../justfile crates/x/src/h.rs"
case_ deny  coder  Bash "cd crates && ln ../justfile h.rs"
case_ deny  perf-engineer Bash "ln -sf ../../Cargo.toml .claude/perf/x"
case_ deny  coder  Write "$R/juſtfile"
case_ deny  coder  Write "$R/.cargo/conﬁg.toml"
case_ deny  coder  Write "$R/crates/lemma-syntax/src/teſts.rs"
case_ deny  coder  Write "$R/.claude/hookſ/path-guard.sh"
case_ deny  devsecops Write "$R/.claude/security/résumé.md"
case_ allow coder  Write "$R/crates/lemma-syntax/src/lib.rs"
case_ allow coder  Write "$R/crates/lemma-syntax/src/Lexer.rs"
case_ allow tester Write "$R/crates/lemma-syntax/src/tests.rs"
case_ allow devsecops Write "$R/.claude/security/x.md"
case_ allow devsecops Bash "cargo deny check 2>&1 | tee .claude/security/deny.log"
case_ allow coder  Bash "grep -rn println crates"
case_ allow coder  Bash "ls -ln crates"

# --- gh: who may call it at all ------------------------------------------------
case_ deny  coder Bash "gh issue list"
case_ deny  planner Bash "gh issue view 3"
case_ deny  committer Bash "git log --oneline -3 | gh issue comment 3 --body-file -"
case_ deny  reviewer Bash "/opt/homebrew/bin/gh pr list"
case_ deny  debugger Bash $'cargo test -p lemma-syntax\ngh issue close 3'
case_ deny  devsecops Bash "gh run list"
case_ allow release-engineer Bash "gh run list --workflow ci.yml"
case_ allow release-engineer Bash "gh run view 123 --log-failed"
case_ allow release-engineer Bash "gh run watch 123"
case_ deny  release-engineer Bash "gh run rerun 123"
case_ deny  release-engineer Bash "gh workflow run ci.yml"
case_ deny  release-engineer Bash "gh run list && gh run cancel 123"
case_ allow release-engineer Bash "actionlint"
case_ allow release-engineer Bash "actionlint .github/workflows/ci.yml"

# --- The shared checkout: only the ticket-brancher moves HEAD -------------------
case_ deny  release-engineer Bash "git switch ci/macos-llvm-link-conflict"
case_ deny  coder     Bash "git checkout main"
case_ deny  coder     Bash "git -C /tmp/x checkout -b other"
case_ deny  tester    Bash "git stash"
case_ deny  tester    Bash "git stash pop"
case_ deny  coder     Bash "cargo test; git stash push -m wip"
case_ deny  committer Bash "git reset --hard HEAD"
case_ deny  committer Bash "git rebase origin/main"
case_ deny  debugger  Bash "git pull --ff-only"
case_ deny  general-purpose Bash "git merge origin/main"
case_ allow ticket-brancher Bash "git switch -c LEM-9-dot-subset-grammar origin/main"
case_ allow coder     Bash "git checkout -- crates/lemma-syntax/src/lib.rs"
case_ allow tester    Bash "git checkout abc123 -- crates/lemma-syntax/src/tests.rs"
case_ allow coder     Bash "git stash list"
case_ allow committer Bash "git reset HEAD crates/lemma-syntax/src/lib.rs"
case_ allow reviewer  Bash "git merge-base origin/main HEAD"
case_ allow reviewer  Bash "git log --merges --oneline"
case_ allow committer Bash "git commit -m \"docs: explain git switch and git stash\""
case_ allow ""        Bash "git switch main"
case_ deny  tester    Bash "git stash -u"
case_ deny  tester    Bash "git stash -m \"wip\""
case_ deny  coder     Bash "git stash --keep-index"
case_ deny  coder     Bash "git stash -- crates/lemma-syntax/src/lib.rs"
case_ allow coder     Bash "git stash show -p"
case_ deny  coder     Bash "git -C \"\$wt\" reset --hard"
case_ deny  coder     Bash "git -C \"\$wt\" switch main"
case_ deny  coder     Bash "git checkout main --"
case_ deny  coder     Bash "git checkout -- ."
case_ deny  tester    Bash "git checkout abc123 -- ."
case_ deny  coder     Bash "git restore ."
case_ deny  coder     Bash "git restore --staged --worktree :/"
case_ deny  coder     Bash "git clean -fd"
case_ deny  coder     Bash "git clean --force"
case_ allow coder     Bash "git clean -n"
case_ allow coder     Bash "git restore crates/lemma-syntax/src/lib.rs"
case_ allow committer Bash "git restore --staged crates/lemma-syntax/src/lib.rs"
case_ allow coder     Bash "git -C \"\$wt\" status --short"

# --- Read-only roles: the just, taplo and typos forms that write ---------------
case_ allow reviewer Bash "just check"
case_ allow reviewer Bash "just fmt-check"
case_ allow reviewer Bash "just lint 2>&1 | tail -20"
case_ deny  reviewer Bash "just fmt"
case_ deny  auditor  Bash "just setup"
case_ deny  release-engineer Bash "taplo fmt"
case_ allow release-engineer Bash "taplo fmt --check"
case_ deny  planner Bash "typos -w"
case_ deny  planner Bash "typos --write-changes crates"
case_ allow planner Bash "typos"
case_ allow coder Bash "just fmt"
case_ allow coder Bash "just check"

# --- Issues: ticket-reader and ticket-writer ------------------------------------
case_ allow ticket-reader Bash "gh issue view 3 --comments"
case_ allow ticket-reader Bash "gh issue view 3 --json number,title,state,labels,milestone,blockedBy,subIssues"
case_ allow ticket-reader Bash "gh issue list --state all --search loop"
case_ allow ticket-reader Bash "gh label list"
case_ allow ticket-reader Bash "gh api repos/{owner}/{repo}/milestones"
case_ allow ticket-reader Bash "gh api repos/{owner}/{repo}/issues/3/comments"
case_ deny  ticket-reader Bash "gh api -X DELETE repos/{owner}/{repo}/issues/comments/9"
case_ deny  ticket-reader Bash "gh api repos/{owner}/{repo}/issues/3/labels -f labels[]=bug"
case_ deny  ticket-reader Bash "gh api graphql -f query=x"
case_ deny  ticket-reader Bash "gh api repos/{owner}/{repo}/issues/3/comments -fbody=x"
case_ deny  ticket-reader Bash "gh api repos/{owner}/{repo}/issues/3/comments -Fbody=@-"
case_ deny  ticket-reader Bash "gh api repos/{owner}/{repo}/issues/3/comments -iXPOST"
case_ deny  ticket-reader Bash "gh api repos/{owner}/{repo}/issues/3/labels --method=POST"
case_ allow ticket-reader Bash "gh api repos/{owner}/{repo}/issues/3/comments --paginate --jq '.[].id'"
case_ deny  ticket-reader Bash "gh issue comment 3 --body x"
case_ deny  ticket-reader Bash "gh issue view 3 && gh issue close 3"
case_ deny  ticket-reader Bash $'gh issue view 3\ngh issue close 3'
case_ deny  ticket-reader Bash "rm -rf crates"
case_ deny  ticket-reader Write "$R/.claude/tmp/x.md"
case_ allow ticket-writer Write "$R/.claude/tmp/LEM-3.md"
case_ allow ticket-writer Bash $'gh issue comment 3 --body-file - <<\'EOF\'\nClarified.\nEOF'
case_ allow ticket-writer Bash $'gh issue comment 3 --edit-last --body-file - <<\'EOF\'\nFixed.\nEOF'
case_ allow ticket-writer Bash "gh issue edit 3 --add-label perf --milestone v0.1"
case_ allow ticket-writer Bash "gh issue edit 3 --add-blocked-by 2"
case_ allow ticket-writer Bash "gh issue edit 3 --remove-blocked-by 2"
case_ allow ticket-writer Bash "gh issue edit 1 --add-sub-issue 3"
case_ allow ticket-writer Bash "gh issue create --title x --body-file .claude/tmp/LEM-3.md --blocked-by 2"
case_ allow ticket-writer Bash "gh label create perf --description x"
case_ allow ticket-writer Bash $'gh api -X PATCH repos/{owner}/{repo}/issues/comments/9 -F body=@- <<\'EOF\'\nFixed.\nEOF'
case_ deny  ticket-writer Bash "gh issue close 3"
case_ deny  ticket-writer Bash "gh issue reopen 3"
case_ deny  ticket-writer Bash "gh issue delete 3 --yes"
case_ deny  ticket-writer Bash "gh issue comment 3 --delete-last --yes"
case_ deny  ticket-writer Bash "gh issue edit 3 --add-assignee @me"
case_ deny  ticket-writer Bash "gh issue create --title x -a @me --body-file -"
case_ deny  ticket-writer Bash "gh issue create --title x -e"
case_ deny  ticket-writer Bash "gh issue comment 3 --web"
case_ deny  ticket-writer Bash "gh label create perf --force"
case_ deny  ticket-writer Bash "gh api -X DELETE repos/{owner}/{repo}/issues/comments/9"
case_ deny  ticket-writer Bash "gh api -X PATCH repos/{owner}/{repo}/issues/comments/9 -X DELETE"
case_ deny  ticket-writer Bash "gh api -X PATCH repos/{owner}/{repo}/issues/comments/9 --method DELETE"
case_ deny  ticket-writer Bash "gh issue comment 3 --body \"\$(git log -1)\""
case_ deny  ticket-writer Bash "gh pr create --title x --head LEM-3-x --body-file -"

# --- Pull requests: pr-writer ---------------------------------------------------
case_ allow pr-writer Bash $'gh pr create --title "LEM-3 Parse loop carries" --body-file - --base main --head LEM-3-loop-carries <<\'EOF\'\nCloses #3\nEOF'
case_ allow pr-writer Bash "gh pr create -t x -F .claude/tmp/pr.md -B main -H LEM-3-x"
case_ allow pr-writer Bash "gh pr list --head LEM-3-x --state all"
case_ allow pr-writer Bash "gh pr view 7"
case_ allow pr-writer Bash $'gh pr edit 7 --title "LEM-3 Parse loop carries" --body-file - <<\'EOF\'\nCloses #3\nEOF'
case_ allow pr-writer Bash $'gh pr comment 7 --body-file - <<\'EOF\'\nUpdated.\nEOF'
case_ allow pr-writer Bash "git log --oneline origin/main..HEAD"
case_ allow pr-writer Write "$R/.claude/tmp/pr-7.md"
case_ deny  pr-writer Write "$R/README.md"
case_ deny  pr-writer Bash "gh pr create --title x --body-file -"
case_ deny  pr-writer Bash "gh pr create --title x --body-file - --head LEM-3-x --dry-run"
case_ deny  pr-writer Bash "gh pr create --head LEM-3-x --web"
case_ deny  pr-writer Bash "gh pr create --head LEM-3-x -e"
case_ deny  pr-writer Bash "gh pr merge 7 --squash --match-head-commit abc"
case_ deny  pr-writer Bash "gh pr review 7 --approve"
case_ deny  pr-writer Bash "gh pr close 7"
case_ deny  pr-writer Bash "gh pr comment 7 --delete-last --yes"
case_ deny  pr-writer Bash "git push origin LEM-3-x"

# --- Pull requests: pr-reviewer -------------------------------------------------
case_ allow pr-reviewer Bash "gh pr view 7 --json number,isDraft,baseRefName,headRefOid,mergeable,mergeStateStatus,reviewDecision"
case_ allow pr-reviewer Bash "gh pr view 7 --comments"
case_ allow pr-reviewer Bash "gh pr checks 7"
case_ allow pr-reviewer Bash "gh pr diff 7 --name-only"
case_ allow pr-reviewer Bash "gh api repos/{owner}/{repo}/pulls/7/comments"
case_ allow pr-reviewer Bash "gh run view 123 --log-failed"
case_ allow pr-reviewer Bash $'gh pr comment 7 --body-file - <<\'EOF\'\nBlocked: the linux job fails.\nEOF'
case_ allow pr-reviewer Bash "gh pr merge 7 --squash --match-head-commit 0123abc --subject \"feat(syntax): parse loop carries (#7)\""
case_ allow pr-reviewer Bash "gh pr merge 7 --rebase --match-head-commit 0123abc"
case_ allow pr-reviewer Bash "gh pr merge 7 -s --match-head-commit=0123abc"
case_ deny  pr-reviewer Bash "gh pr merge 7 --squash --match-head-commit 0123abc --admin"
case_ deny  pr-reviewer Bash "gh pr merge 7 --squash --match-head-commit 0123abc --auto"
case_ deny  pr-reviewer Bash "gh pr merge 7 --squash --match-head-commit 0123abc --yes"
case_ deny  pr-reviewer Bash "gh pr merge 7 --merge --match-head-commit 0123abc"
case_ deny  pr-reviewer Bash "gh pr merge 7 -m --match-head-commit 0123abc"
case_ deny  pr-reviewer Bash "gh pr merge 7 --match-head-commit 0123abc"
case_ deny  pr-reviewer Bash "gh pr merge 7 --squash"
case_ deny  pr-reviewer Bash "gh pr merge 7 --disable-auto"
case_ deny  pr-reviewer Bash "gh pr review 7 --approve"
case_ deny  pr-reviewer Bash "gh pr close 7"
case_ deny  pr-reviewer Bash "gh run rerun 123"
case_ deny  pr-reviewer Bash "gh api -X PUT repos/{owner}/{repo}/pulls/7/merge"
case_ allow pr-reviewer Bash "just check"
case_ allow pr-reviewer Bash "just lint"
case_ deny  pr-reviewer Bash "just fmt"
case_ deny  pr-reviewer Bash "just setup"
# After an allowed prefix, the line holds plain words only: no pipe, redirection or $.
case_ deny  pr-reviewer Bash "just test 2>&1 | tail -30"
case_ deny  pr-reviewer Bash "just check fmt"
case_ deny  pr-reviewer Bash "just check setup"
case_ deny  pr-reviewer Bash "just check \"fmt\""
case_ deny  pr-reviewer Bash "git log -1 --format=%s | sh"
case_ deny  pr-reviewer Bash "git log > .claude/reviews/pr/x.md"
case_ deny  pr-reviewer Bash "git log --oneline & rm -rf crates"
case_ deny  pr-reviewer Bash "git log --format=\"'\" > README.md \"'\""
case_ deny  pr-reviewer Bash "git log -1 --format=%B%n#\\\" | sh -s \\\""
case_ deny  pr-reviewer Bash "git log <(rm -rf crates)"
case_ deny  pr-reviewer Bash "git log	--output=README.md"
case_ deny  pr-reviewer Bash "git fetch --upload-pack=touch /tmp/repo"
case_ deny  pr-reviewer Bash "git fetch --upl=touch /tmp/repo"
case_ deny  pr-reviewer Bash "git fetch \"--upload-pack=touch\" /tmp/repo"
case_ deny  pr-reviewer Bash "git diff --output=crates/lemma-syntax/src/lib.rs"
case_ deny  pr-reviewer Bash "git log \"--output=README.md\""
case_ deny  pr-reviewer Bash "git branch -D LEM-3-x"
case_ deny  pr-reviewer Bash "git branch -f main HEAD~3"
case_ deny  pr-reviewer Bash "git symbolic-ref HEAD refs/heads/x"
case_ deny  pr-reviewer Bash "git fetch origin +main:refs/heads/main"
case_ deny  pr-writer   Bash "git log --oneline | tee notes.md"
case_ deny  ticket-reader Bash "git log | sh"
case_ deny  pr-reviewer Bash $'gh pr comment 7 --body-file - <<\'EOF\'\nx\nEOF\nrm -rf crates'
case_ allow pr-reviewer Bash "git branch -a --contains HEAD"
case_ allow pr-reviewer Bash "git symbolic-ref --short refs/remotes/origin/HEAD"
case_ allow pr-reviewer Bash "git fetch origin"
case_ allow pr-reviewer Bash "git log --oneline origin/main..HEAD"
case_ allow pr-writer   Bash "gh pr view 7 --json title --jq .title"
case_ deny  pr-reviewer Bash "git log --out''put=README.md"
case_ deny  pr-reviewer Bash "git fetch --u''pl=touch /tmp/repo"
case_ deny  pr-reviewer Bash "git fetch-pack --ex''ec=touch /tmp/repo"
case_ deny  pr-reviewer Bash "git branch '-D' LEM-3-x"
case_ deny  pr-reviewer Bash "git branch --del LEM-3-x"
case_ deny  pr-reviewer Bash "git branch LEM-3-new"
case_ deny  pr-reviewer Bash "git fetch --prune-tags origin"
case_ deny  pr-reviewer Bash "git fetch -P origin"
case_ deny  pr-reviewer Bash "git fetch --set-upstream origin main"
case_ deny  pr-reviewer Bash "cargo test --config 'target.x.runner=[\"rm\",\"-rf\",\"crates\"]'"
case_ deny  pr-reviewer Bash "cargo build --config build.rustc-wrapper=/tmp/w"
case_ allow pr-reviewer Bash "git branch"
case_ allow pr-reviewer Bash "git branch -r"
case_ allow pr-reviewer Bash "git branch --list 'LEM-*'"
case_ deny  pr-reviewer Bash "git branch --list LEM-*"
case_ deny  pr-reviewer Bash "git log *"
case_ deny  pr-reviewer Bash "git fetch -pP origin"
case_ deny  pr-reviewer Bash "git fetch --depth=1 origin"
case_ deny  pr-reviewer Bash "cargo test -- --logfile README.md"
case_ deny  pr-reviewer Bash "cargo clippy -- --emit=metadata=README.md"
case_ deny  pr-reviewer Bash "cargo build --target-dir /tmp/elsewhere"
case_ deny  pr-reviewer Bash "cargo test --manifest-path /tmp/other/Cargo.toml"
case_ deny  pr-reviewer Bash "typos --dump-config README.md"
case_ deny  pr-reviewer Bash "git log --{output=README.md,stat}"
case_ deny  pr-reviewer Bash "cargo test {--target-dir=/tmp/x,-q}"
case_ deny  pr-reviewer Bash "git fetch --{upload-pack=touch,quiet} /tmp/repo"
case_ deny  pr-reviewer Bash "typos {--dump-config=README.md,-q}"
case_ deny  pr-reviewer Bash "git fetch -f origin tag v1"
case_ allow pr-reviewer Bash "gh api repos/{owner}/{repo}/pulls/7/comments"
case_ allow pr-reviewer Bash "git fetch -p origin"
case_ allow pr-reviewer Bash "cargo test -p lemma-diagnostics --test text"
case_ allow pr-reviewer Bash "git branch --merged main"
case_ allow pr-reviewer Bash "git log --format='%h %s' -5"
case_ allow pr-reviewer Bash "git fetch --prune origin"
case_ allow pr-reviewer Bash "cargo nextest run --workspace"
case_ allow pr-reviewer Bash "RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps"
case_ allow pr-reviewer Bash "cargo fmt --all --check"
case_ deny  pr-reviewer Bash "cargo fmt --all"
case_ deny  pr-reviewer Bash "cargo clippy --fix"
case_ allow pr-reviewer Bash "typos"
case_ deny  pr-reviewer Bash "typos -w"
case_ allow pr-reviewer Bash "cog check origin/main..HEAD"
case_ allow pr-reviewer Bash "git rev-parse HEAD"
case_ deny  pr-reviewer Bash "npm test"
case_ allow pr-reviewer Write "$R/.claude/reviews/pr/7.md"
case_ deny  pr-reviewer Write "$R/.claude/reviews/mr/7.md"
case_ deny  pr-reviewer Write "$R/.claude/reviews/LEM-3.md"

# --- Pull requests in a worktree: the same commands, aimed at a registered worktree
wt_main="$tmp/wt-main"
wt_copy="$tmp/wt-copy"
wt_stray="$tmp/wt-stray"
mkdir -p "$wt_main/.claude" "$wt_stray"
git -C "$wt_main" init -q
git -C "$wt_main" -c user.name=t -c user.email=t@t commit -qm base --allow-empty
git -C "$wt_main" worktree add -q --detach "$wt_copy"
case_ allow pr-reviewer Bash "git -C $wt_copy log --oneline origin/main..HEAD" "$wt_main"
case_ allow pr-reviewer Bash "git -C \"$wt_copy\" status" "$wt_main"
case_ allow pr-reviewer Bash "git -C ../wt-copy rev-parse HEAD" "$wt_main"
case_ allow pr-reviewer Bash "just $wt_copy/check" "$wt_main"
case_ allow pr-reviewer Bash "just $wt_copy/rules" "$wt_main"
case_ allow pr-writer   Bash "git -C $wt_copy log --oneline origin/main..HEAD" "$wt_main"
case_ deny  pr-reviewer Bash "git -C $wt_stray log" "$wt_main"
case_ deny  pr-reviewer Bash "just $wt_stray/check" "$wt_main"
case_ deny  pr-reviewer Bash "git -C $wt_copy push origin HEAD" "$wt_main"
case_ deny  pr-reviewer Bash "just $wt_copy/fmt" "$wt_main"
case_ deny  pr-reviewer Bash "git -C $wt_copy log && rm -rf $wt_copy" "$wt_main"
case_ deny  pr-writer   Bash "just $wt_copy/check" "$wt_main"
case_ allow pr-reviewer Bash "git -C $wt_copy diff --stat origin/main..HEAD -- crates/" "$wt_main"
mkdir -p "$tmp/wt-far/inside"
ln -s "$tmp/wt-far/inside" "$wt_copy/link"
case_ deny  pr-reviewer Bash "git -C $wt_copy/link/.. status" "$wt_main"
case_ deny  pr-reviewer Bash "just $wt_copy/link/../check" "$wt_main"
case_ deny  pr-reviewer Bash "git -C \"$wt_copy/	 log\" push origin HEAD" "$wt_main"
case_ deny  pr-reviewer Bash "git -C $wt_copy -C $wt_stray log" "$wt_main"
case_ deny  pr-reviewer Bash "git -C $wt_copy -c core.pager=sh log" "$wt_main"
git -C "$wt_main" worktree add -q --detach "$tmp/wt-a*[b]"
case_ allow pr-reviewer Bash "git -C \"$tmp/wt-a*[b]\" log" "$wt_main"
case_ deny  pr-reviewer Bash "git -C \"$tmp/wt-a*[b]\" push \"b\" log" "$wt_main"
case_ deny  pr-reviewer Bash "just /check" "$wt_main"
case_ deny  pr-reviewer Bash "git -C -L log" "$wt_main"

# --- Renamed roles: the old names are no longer GitHub roles ---------------------
case_ deny  mr-reviewer Bash "gh pr merge 7 --squash --match-head-commit 0123abc"
case_ deny  mr-writer Bash "gh pr create --title x --head LEM-3-x --body-file -"

# --- Zones of the CI, supply-chain, proof, perf and design roles ---------
case_ allow release-engineer Write "$R/clippy.toml"
case_ allow release-engineer Edit  "$R/cog.toml"
case_ allow release-engineer Edit  "$R/.cargo/config.toml"
case_ allow release-engineer Edit  "$R/.github/workflows/ci.yml"
case_ allow release-engineer Write "$R/.github/dependabot.yml"
case_ allow release-engineer Edit  "$R/justfile"
case_ allow release-engineer Edit  "$R/taplo.toml"
case_ allow release-engineer Edit  "$R/_typos.toml"
case_ allow release-engineer Edit  "$R/.editorconfig"
case_ allow release-engineer Edit  "$R/rust-toolchain.toml"
case_ deny  release-engineer Write "$R/.gitlab-ci.yml"
case_ deny  release-engineer Edit  "$R/Cargo.toml"
case_ deny  release-engineer Edit  "$R/crates/lemma-cli/src/main.rs"
case_ allow devsecops Write "$R/.claude/security/supply-chain.md"
case_ deny  devsecops Edit  "$R/deny.toml"
case_ allow proof-engineer Edit  "$R/examples/dot.lm"
case_ allow proof-engineer Write "$R/.claude/proofs/LEM-4.md"
case_ deny  proof-engineer Edit  "$R/crates/lemma-verify/src/lib.rs"
case_ deny  proof-engineer Write "$R/crates/lemma-verify/src/tests.rs"
case_ allow proof-engineer Bash "cargo test -p lemma-verify > .claude/proofs/run.log 2>&1"
case_ deny  proof-engineer Bash "echo x > crates/lemma-verify/src/lib.rs"
case_ allow perf-engineer Write "$R/crates/lemma-codegen/benches/dot.rs"
case_ allow perf-engineer Write "$R/.claude/perf/LEM-5.md"
case_ deny  perf-engineer Edit  "$R/crates/lemma-codegen/src/lib.rs"
case_ deny  perf-engineer Bash "sudo ./target/release/bench"
case_ allow perf-engineer Bash "cargo bench -p lemma-codegen 2>&1 | tee .claude/perf/run.txt"
case_ allow language-designer Write "$R/docs/adr/0003-memory-effects.md"
case_ allow language-designer Edit  "$R/docs/design.md"
case_ deny  language-designer Edit  "$R/README.md"

# --- Fail closed ----------------------------------------------------------------
check deny "malformed event" '{"agent_type": "coder", "tool_name": "Write",'

nojq="$tmp/nojq"
mkdir -p "$nojq"
for dir in /bin /usr/bin; do
  for tool in "$dir"/*; do
    name=${tool##*/}
    [ "$name" = jq ] || [ -e "$nojq/$name" ] || ln -s "$tool" "$nojq/$name"
  done
done
got=$(PATH="$nojq" verdict "$(event coder Write "$R/crates/lemma-codegen/src/lib.rs" "$R")")
if [ "$got" = deny ]; then pass=$((pass + 1)); else failed=$((failed + 1)); echo "FAIL  deny   got $got  jq missing from PATH"; fi

lost="$tmp/no-project"
mkdir -p "$lost"
payload=$(event coder Write "$lost/src/lib.rs" "$lost")
got=$(unset CLAUDE_PROJECT_DIR; verdict "$payload")
if [ "$got" = deny ]; then pass=$((pass + 1)); else failed=$((failed + 1)); echo "FAIL  deny   got $got  no .claude/ anywhere above the cwd"; fi

# --- Anchoring: the Git toplevel wins over a .claude/ in a parent directory ------
nest="$tmp/parent/project"
mkdir -p "$tmp/parent/.claude" "$nest/.claude" "$nest/src"
git -C "$nest" init -q
printf 'special_*\n' > "$nest/.claude/test-paths.txt"
check deny "coder writes a custom test path under a parent .claude/" \
  "$(event coder Write "$nest/src/special_case.rs" "$nest/src")"
check allow "coder writes production code under a parent .claude/" \
  "$(event coder Write "$nest/src/lib.rs" "$nest/src")"

# --- frozen-check.sh: a contract is everything from fn to impl ------------------
frozen="$tmp/frozen"
mkdir -p "$frozen/.claude" "$frozen/examples"
cp "$root/.claude/test-paths.txt" "$frozen/.claude/"
git -C "$frozen" init -q
cat > "$frozen/examples/get.lm" <<'LM'
// Element k of a.
export fn get(n: u64,
              a: &f32[n] @a4 ro,
              k: u64 < n) -> f32
  eff pure
  req n > 0
    && n < 1000
  spec a[k]
  equiv allow()
  impl {
    return a[k]
  }
LM
git -C "$frozen" add -A
git -C "$frozen" -c user.name=t -c user.email=t@t commit -qm base
frozen_ref=$(git -C "$frozen" rev-parse HEAD)
original=$(cat "$frozen/examples/get.lm")

frozen_case() { # $1 = expected exit status, $2 = description, $3 = sed expression, [$4 = extra file]
  local got
  printf '%s\n' "$original" | sed -E "$3" > "$frozen/examples/get.lm"
  [ -z "${4:-}" ] || printf 'fn extra(n: u64) -> u64\n  eff pure\n  spec n\n  equiv allow()\n  impl {\n    return n\n  }\n' > "$frozen/examples/$4"
  (cd "$frozen" && "$root/.claude/tools/frozen-check.sh" "$frozen_ref" >/dev/null 2>&1)
  got=$?
  rm -f "$frozen/examples/extra.lm"
  if [ "$got" = "$1" ]; then pass=$((pass + 1)); else failed=$((failed + 1)); echo "FAIL  exit $1 got $got  frozen-check: $2"; fi
}
frozen_case 0 "unchanged" 's/^//'
frozen_case 0 "impl changed only" 's/return a\[k\]/return a[k] * 1.0/'
frozen_case 0 "comment and layout of the contract changed" 's|^// Element k of a.|// Read one element.|; s/^  spec a\[k\]$/  spec   a[k]/'
frozen_case 1 "bound dropped on a continuation line of the signature" 's/k: u64 < n\)/k: u64)/'
frozen_case 1 "second line of a req changed" 's/&& n < 1000/\&\& n < 2000/'
frozen_case 1 "equiv permissions changed" 's/allow\(\)/allow(fp_reassoc)/'
frozen_case 1 "new function in an untracked file" 's/^//' extra.lm
frozen_case 0 "comment added inside the contract" 's|^  spec a\[k\]$|  spec a[k] // the element|'
frozen_case 0 "export moved to its own line" 's/^export fn get/export\
fn get/'
frozen_case 1 "export dropped" 's/^export fn get/fn get/'
frozen_case 1 "clause written after the impl" '$a\
  req n > 1'
frozen_case 1 "clause written on the impl line" 's/^  impl \{$/  impl { } ens result == a[k] {/'

frozen_move() { # $1 = description, $2 = new path ("" deletes the file)
  local got
  printf '%s\n' "$original" > "$frozen/examples/get.lm"
  if [ -n "$2" ]; then mv "$frozen/examples/get.lm" "$frozen/examples/$2"; else rm "$frozen/examples/get.lm"; fi
  (cd "$frozen" && "$root/.claude/tools/frozen-check.sh" "$frozen_ref" >/dev/null 2>&1)
  got=$?
  [ -z "$2" ] || rm -f "$frozen/examples/$2"
  printf '%s\n' "$original" > "$frozen/examples/get.lm"
  if [ "$got" = 1 ]; then pass=$((pass + 1)); else failed=$((failed + 1)); echo "FAIL  exit 1 got $got  frozen-check: $1"; fi
}
frozen_move "file deleted" ""
frozen_move "file renamed to a name with a space" "get copy.lm"

# --- check.sh: a changed file that is not valid UTF-8 is still read whole ------
bytes="$tmp/bytes"
mkdir -p "$bytes/.claude"
cp "$root/.claude/project.conf" "$bytes/.claude/"
git -C "$bytes" init -q
git -C "$bytes" -c user.name=t -c user.email=t@t commit -qm base --allow-empty
key=$(sed -n 's/^TICKET_KEY=[[:space:]]*\([A-Za-z][A-Za-z0-9]*\).*/\1/p' "$root/.claude/project.conf" | head -1)
printf 'a\377b\n// %s-1\n' "$key" > "$bytes/fixture.lm"
utf8=$(locale -a 2>/dev/null | grep -ix -m1 'c\.utf-\{0,1\}8\|en_US\.utf-\{0,1\}8')
bytes_out=$(cd "$bytes" && LC_ALL=$utf8 "$root/.claude/rules/check.sh" HEAD 2>"$tmp/bytes.err")
bytes_case() { # $1 = description, $2 = condition already evaluated (0 = holds)
  if [ "$2" = 0 ]; then pass=$((pass + 1)); else failed=$((failed + 1)); echo "FAIL  check.sh on a file that is not UTF-8: $1"; fi
}
[ -n "$utf8" ]; bytes_case "a UTF-8 locale to run it under" $?
[ ! -s "$tmp/bytes.err" ]; bytes_case "nothing on stderr" $?
printf '%s\n' "$bytes_out" | grep -qF -- '- .lm (1 file(s))'; bytes_case "its type listed under Not checked" $?
printf '%s\n' "$bytes_out" | grep -q 'fixture.lm:2.*ticket-key'; bytes_case "a finding after the invalid byte" $?

printf '%s passed, %s failed\n' "$pass" "$failed"
[ "$failed" -eq 0 ]
