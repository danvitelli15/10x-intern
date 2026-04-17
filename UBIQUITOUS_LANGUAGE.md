# Ubiquitous Language

## Work items

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Issue** | A unit of work tracked in GitHub — the universal term for any item the tool operates on | Task, item, card |
| **Ticket** | A documented item of work — used colloquially for any issue regardless of project or `IssueType`. In the codebase, `Ticket` specifically means `IssueType::Ticket` (a leaf node directly implementable by the agent) | Story, task |
| **Feature** | An issue of type `Feature` — a parent issue whose children must be completed to satisfy it | Epic (unless the issue tracker uses that term) |
| **Child issue** | An issue created under a parent Feature, either initially or as a result of review | Sub-task, sub-issue |
| **Label** | A GitHub label used to select a set of issues for execution (e.g. `hitl`, `sprint-1`) | Tag |

## Execution model

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Action** | A discrete, reusable operation that may be called from multiple places — always a pure function taking a `Context` or explicit parameters | Step, task, operation |
| **Behavior** | A named composition of actions (and other behaviors) that accomplishes a meaningful intermediate goal — not necessarily exposed at the CLI level | Flow, process, pipeline |
| **Workflow** | The composition of behaviors triggered by a single CLI subcommand — the complete user-facing operation a subcommand performs | — |
| **Context** | The struct that carries all dependencies (issue tracker, source control, runner, config) for a single execution run | Container, service locator |
| **Budget** | The maximum number of agent invocations allowed in a run, enforced by `Context` | Limit, cap |

## CLI surface

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Subcommand** | A named entry point in the CLI (e.g. `implement`, `clear`, `init`) — each subcommand triggers exactly one Workflow | Command, flag |

## Agent interaction

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Prompt** | A rendered string sent to the agent, produced by substituting variables into a prompt template | Message, input |
| **Prompt template** | A `.md` file containing `{{variable}}` placeholders — either from the scaffold or a repo override | Prompt file (acceptable informally) |
| **Scaffold** | The default prompt templates and config written by `intern init` | Boilerplate, starter |
| **Override** | A repo-specific prompt template in `.intern/prompts/` that replaces the scaffold default for that prompt | Custom prompt |
| **Repo context** | The contents of the configured `context_file` (e.g. `CLAUDE.md`), injected as `{{repo_context}}` into every prompt | Context, system prompt |
| **Agent output** | The stdout produced by a single agent invocation, including any embedded result signals | Response, result |

## Result signals

| Term | Definition | Aliases to avoid |
|---|---|---|
| **FINDINGS** | Review signal indicating the implementation has issues requiring a follow-up child issue | Fail, error |
| **CLEAN** | Review signal indicating the implementation satisfies acceptance criteria | Pass, ok, success |
| **IN_SCOPE_FINDINGS** | Feature review signal indicating gaps or bugs directly caused by the feature's implementation | — |
| **HITL** | A label meaning "human in the loop" — issues labeled `hitl` are skipped by the agent and left for a human | Manual, skip |

## Issue lifecycle

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Claimed** | An issue the agent has taken ownership of for the current run | Assigned, locked |
| **Complete** | An issue whose implementation was accepted by the review action | Done, resolved, closed |
| **Skipped** | An issue the agent abandoned, typically due to budget exhaustion or a second failed feature review | Abandoned, failed |

## Source control — branching

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Merge strategy** | Config value that controls branch topology and PR lifecycle for a run — one of `Direct`, `PerTicket`, or `FeatureBranch` | Commit strategy (old name — avoid), integration strategy |
| **Direct** | A **merge strategy** where the agent works on the current branch with no new branch created and no PR opened | — |
| **PerTicket** | A **merge strategy** where each ticket gets its own branch (forked from **base branch**) and its own PR back into the base | — |
| **FeatureBranch** | A **merge strategy** where a feature issue gets a branch, each child ticket gets its own branch forked from the feature branch, and every branch gets a PR into its parent — forming a recursive branch hierarchy | — |
| **Recursive branching** | The topology produced by `FeatureBranch`: child branches target the feature branch, which targets the base branch, allowing arbitrary depth without special-casing | — |
| **Base branch** | The branch a working branch is forked from, used as the merge target for PRs and the base for `diff_from_base` in review — configured in `[source_control]` | Main, master, trunk |
| **Setup workspace** | The precursor behavior step that prepares the working environment before implementation begins — creates a branch in `PerTicket`/`FeatureBranch`, no-op in `Direct`, will provision **worktrees** in future | Workspace init, branch setup |
| **Worktree** | A `git worktree`-isolated working directory for a single ticket run — planned but not yet implemented | Workspace, working copy |
| **PR step** | The terminal phase of `complete_ticket` (and `complete_feature` under `FeatureBranch`) that opens a pull request for the current branch — fires automatically whenever a branch was created; never fires under `Direct` | Create PR |

## Source control — dirty state

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Dirty state** | The condition after implementation where the working tree has uncommitted changes, no commits, or both | Unclean, uncommitted |
| **DirtyBehavior** | A config-level enum (`Fail` / `Warn` / `Commit`) that governs the orchestrator's response to a specific dirty state axis | Dirty policy, dirty mode |
| **on_dirty_no_commits** | Config axis: the `DirtyBehavior` applied when the agent made zero commits after implementation — signals the agent did no useful work | — |
| **on_dirty_after_commit** | Config axis: the `DirtyBehavior` applied when the agent committed at least once but left uncommitted changes behind | — |

## Configuration

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Work directory** | The root directory used to resolve `.intern/` paths — defaults to CWD, overridable in config | Base dir, project root |
| **Context file** | A path in config pointing to a repo description file (e.g. `CLAUDE.md`) whose contents become `repo_context` | Context, docs file |

## Relationships

- A **Feature** has one or more **child issues** (which are **Tickets** or nested **Features**)
- A **Subcommand** triggers exactly one **Workflow**
- A **Workflow** composes one or more **Behaviors**; a **Behavior** may compose other **Behaviors**
- A **Behavior** calls one or more **Actions**; **Actions** do not call **Behaviors** or **Workflows**
- Some **Behaviors** are internal-only and never directly exposed as a **Subcommand**
- A **Prompt** is produced by rendering a **Prompt template** with issue data and **repo context**
- An **Override** shadows the **Scaffold** for a specific prompt name; if no override exists, the behavior errors and instructs the user to run `intern init`
- A **Ticket** produces a **FINDINGS** or **CLEAN** signal from the review **Action**
- A **Feature** produces an **IN_SCOPE_FINDINGS** or **CLEAN** signal from the feature review **Action**
- **Setup workspace** precedes `implement` in every behavior that calls it
- Under **FeatureBranch**, each child **Ticket** opens a PR into the feature branch; the feature itself opens a PR into the **base branch** — this is **recursive branching**
- Under **PerTicket**, each **Ticket** opens a PR into the **base branch** directly; no feature-level PR exists
- Under **Direct**, no branch is created and no PR is opened, regardless of any other config
- `on_dirty_no_commits` fires whenever the agent made zero commits, regardless of tree cleanliness; `on_dirty_after_commit` fires only when commits exist but the tree is also dirty
- **Merge strategy** and **base branch** are configured together in `[source_control]`

## Example dialogue

> **Dev:** "Under `FeatureBranch`, does `execute_ordered` suppress the **PR step** per ticket?"
> **Domain expert:** "No — each ticket still opens a PR. But the PR targets the *feature branch*, not main. The **recursive branching** model means every completed branch, at any level, gets a PR into its parent."
> **Dev:** "So if feature #99 has children #10 and #11, we get three PRs?"
> **Domain expert:** "Exactly. `feature/ticket-10` → `feature/ticket-99`, `feature/ticket-11` → `feature/ticket-99`, then `feature/ticket-99` → main. Three branches, three PRs."
> **Dev:** "What happens if the agent leaves uncommitted changes?"
> **Domain expert:** "Depends on which axis. If it made commits but left extra files behind, `on_dirty_after_commit` governs: `Fail` halts the run, `Warn` logs and continues, `Commit` stages everything and commits. If it made *zero* commits at all, that's a different signal — `on_dirty_no_commits` governs that one independently."
> **Dev:** "Why two axes instead of one?"
> **Domain expert:** "Zero commits is a stronger signal than leftover changes. You might tolerate a stray file but still want to fail-fast if the agent did nothing. The two-axis config lets you tune them independently."

## Flagged ambiguities

- **"issue" vs "ticket"**: Both are valid. **Ticket** is the broader colloquial term for any documented work item. **Issue** is the GitHub-specific representation. In the codebase, `Ticket` as an `IssueType` variant means a leaf node — distinct from `Feature`. Context resolves the ambiguity.
- **"command" vs "subcommand"**: Use **subcommand** — `intern` is the binary, `implement`/`clear`/`init` are subcommands. "Command" is too broad.
- **"commit_strategy"**: Renamed to **merge strategy** — the rename is complete. Do not use `commit_strategy`; it conflated commit frequency (owned by the agent) with branch topology (owned by the orchestrator).
