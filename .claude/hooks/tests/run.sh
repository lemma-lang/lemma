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
case_ allow coder  Bash "grep -n 'fn parse() -> Ast' crates/lemma-syntax/src/tests.rs 2>/dev/null"
case_ allow coder  Bash "cargo fmt --all"
case_ deny  coder  Bash "echo x > crates/lemma-codegen/src/tests.rs"
case_ deny  coder  Bash "sed -i '' 's/a/b/' crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "cp /tmp/x.rs crates/lemma-syntax/tests/parse.rs"
case_ deny  coder  Bash "echo x >> .claude/plans/LEM-1.md"
case_ deny  coder  Bash "echo '[build]' >> .cargo/config.toml"
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
case_ allow pr-reviewer Bash "just test 2>&1 | tail -30"
case_ deny  pr-reviewer Bash "just fmt"
case_ deny  pr-reviewer Bash "just setup"
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

printf '%s passed, %s failed\n' "$pass" "$failed"
[ "$failed" -eq 0 ]
