# 10x-intern

Autonomous ticket execution for repos with well-documented, architecturally sound issues.

## How it works

`intern` reads a GitHub issue, breaks it into sub-issues, plans an implementation order,
dispatches an AI agent to implement each one, runs a review pass when done, and opens a pull
request for the completed work.

Each step maps to a configurable adapter:

- **Issue tracker** — fetches issues and posts results (currently: GitHub)
- **Source control** — creates branches, checks dirty state, opens PRs (currently: Git + `gh`)
- **Agent runner** — executes implementation and review (currently: Claude Code, run locally)

The architecture is designed so these adapters can be swapped out. Future versions may support
different issue trackers, hosted agent runners, or alternative AI tools — without changing how
the core workflow operates.

## Prerequisites

| Dependency | Purpose | Adapter |
|---|---|---|
| Rust toolchain | Build the binary | — |
| GitHub CLI (`gh`) | Fetch issues, create PRs | `issue_tracker.kind = "github"` |
| Claude Code | Agent execution | `agent.kind = "local"` |

Dependencies marked with an adapter are only required when that adapter is configured.
Both `gh` and `claude` must be authenticated before use.

## Installation

```bash
git clone https://github.com/danvitelli15/10x-intern.git
cd 10x-intern
cargo build --release
cp target/release/intern /usr/local/bin/intern
```

## Setup

### 1. Initialize your repo

Run this inside the repo you want `intern` to work on:

```bash
intern init
```

This scaffolds `.intern/config.toml` and default prompt files under `.intern/prompts/`.

### 2. Configure

Edit `.intern/config.toml`:

```toml
# context_file = "CLAUDE.md"        # path to repo context file (e.g. CLAUDE.md, AGENTS.md)
# work_directory = "."               # override the working directory for .intern/ lookups

[issue_tracker]
kind = "github"
repo = "owner/repo"                  # replace with your repo in owner/repo format

[source_control]
kind = "git"
base_branch = "main"                 # branch new work is forked from
merge_strategy = "feature-branch"   # direct | per-ticket | feature-branch
use_worktree = false
on_dirty_after_commit = "warn"       # fail | warn | commit
on_dirty_no_commits = "fail"         # fail | warn | commit

[agent]
kind = "local"
# settings_file = ".claude/settings.json"  # optional: path to Claude Code settings file

[run]
max_iterations = 100
```

### Config reference

| Field | Required | Default | Description |
|---|---|---|---|
| `context_file` | no | — | Path to a repo context file (e.g. `CLAUDE.md`) injected into every agent run |
| `work_directory` | no | CWD | Working directory for agent runs |
| `issue_tracker.kind` | yes | — | Issue tracker adapter. Currently: `github` |
| `issue_tracker.repo` | yes | — | Repository in `owner/repo` format |
| `source_control.kind` | no | `git` | Source control adapter. Currently: `git` |
| `source_control.base_branch` | no | `main` | Branch new work is forked from; PR target for top-level branches |
| `source_control.merge_strategy` | no | `feature-branch` | `direct` · `per-ticket` · `feature-branch` — controls branch topology and PR creation |
| `source_control.use_worktree` | no | `false` | Provision a `git worktree` per ticket run (planned — not yet active) |
| `source_control.on_dirty_after_commit` | no | `warn` | `fail` · `warn` · `commit` — response when agent committed but left uncommitted changes |
| `source_control.on_dirty_no_commits` | no | `fail` | `fail` · `warn` · `commit` — response when agent made zero commits after implementation |
| `agent.kind` | yes | — | Agent runner. Currently: `local` |
| `agent.settings_file` | no | — | Path to a Claude Code settings file passed via `--settings` |
| `run.max_iterations` | no | `100` | Max agent invocations per run |

#### Merge strategies

| Strategy | Branches | PRs |
|---|---|---|
| `direct` | None — works on current branch | None |
| `per-ticket` | One branch per ticket, forked from `base_branch` | One PR per ticket into `base_branch` |
| `feature-branch` | Feature branch + one branch per child ticket | Each branch gets a PR into its parent branch (recursive) |

## Commands

### `intern implement <issue-id>`

Implements a GitHub issue and its sub-issues.

```bash
intern implement 42
intern implement 42 --dry-run
intern implement 42 --max-iterations 50
intern implement 42 --merge-strategy direct
```

### `intern clear <label>`

Works through all open issues matching a GitHub label.

```bash
intern clear "ready"
intern clear "ready" --dry-run
```

### `intern review <issue-id>`

Runs the review phase against a completed issue.

```bash
intern review 42
intern review 42 --dry-run
```

### `intern init`

Scaffolds config and prompt files in the current directory.

```bash
intern init
intern init --defaults   # skip prompts, use all defaults
```
