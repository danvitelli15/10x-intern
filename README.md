# 10x-intern

Autonomous ticket execution for repos with well-documented, architecturally sound issues.

## How it works

`intern` reads a GitHub issue, breaks it into sub-issues, plans an implementation order,
dispatches an AI agent to implement each one, and runs a review pass when done.

Each step maps to a configurable adapter:

- **Issue tracker** — fetches issues and posts results (currently: GitHub)
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
# context_file = "CLAUDE.md"      # optional: repo context injected into every agent run
# work_directory = "."             # optional: override working directory

[issue_tracker]
kind = "github"
repo = "owner/repo"                # your repo in owner/repo format

[agent]
kind = "local"
# settings_file = ".claude/settings.json"

[run]
max_iterations = 100
commit_strategy = "feature-branch"
```

### Config reference

| Field | Required | Default | Description |
|---|---|---|---|
| `context_file` | no | — | Path to a repo context file (e.g. `CLAUDE.md`) injected into every agent run |
| `work_directory` | no | CWD | Working directory for agent runs |
| `issue_tracker.kind` | yes | — | Issue tracker adapter. Currently: `github` |
| `issue_tracker.repo` | yes | — | Repository in `owner/repo` format |
| `agent.kind` | yes | — | Agent runner. Currently: `local` |
| `agent.settings_file` | no | — | Path to a Claude Code settings file passed via `--settings` |
| `run.max_iterations` | no | `100` | Max agent invocations per run |
| `run.commit_strategy` | no | `feature-branch` | `direct` · `feature-branch` · `per-ticket` |

## Commands

### `intern implement <issue-id>`

Implements a GitHub issue and its sub-issues.

```bash
intern implement 42
intern implement 42 --dry-run
intern implement 42 --max-iterations 50
intern implement 42 --commit-strategy direct
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
