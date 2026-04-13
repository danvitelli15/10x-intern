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

## Source control

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Merge strategy** *(new)* | Config value that controls branch topology and PR lifecycle for a run — one of `Direct`, `PerTicket`, or `FeatureBranch` | Commit strategy (old name — avoid), integration strategy |
| **Direct** *(new)* | A **merge strategy** where the agent works on the current branch with no new branch created and no PR opened | — |
| **PerTicket** *(new)* | A **merge strategy** where each ticket gets its own branch and its own PR | — |
| **FeatureBranch** *(new)* | A **merge strategy** where all tickets in an `execute_ordered` run share one branch; a single PR is opened after all tickets complete | — |
| **Base branch** *(new)* | The branch a working branch is forked from, used as the base for `diff_from_base` in review — configured in `[source_control]` | Main, master, trunk |
| **Setup workspace** *(new)* | The precursor **behavior** step that prepares the working environment before implementation begins — a no-op today, will provision **worktrees** in future | Workspace init, branch setup |
| **Worktree** *(new)* | A `git worktree`-isolated working directory for a single ticket run — planned but not yet implemented | Workspace, working copy |
| **PR step** *(new)* | The terminal phase of `complete_ticket` that opens a pull request — may be suppressed when called from `execute_ordered` under `FeatureBranch` | Create PR |

## Result signals — source control *(new)*

| Term | Definition | Aliases to avoid |
|---|---|---|
| **branchCreated** *(new)* | An `<branchCreated>…</branchCreated>` tag emitted by the agent in its output, naming the branch it created — used as a fallback when `current_branch()` returns something unexpected | Branch signal, branch tag |

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
- **Setup workspace** precedes `implement` in every **behavior** that calls it
- Under `FeatureBranch`, `execute_ordered` suppresses the **PR step** per ticket and opens one PR after all tickets complete
- The agent emits a **branchCreated** tag; the orchestrator validates it against `current_branch()` post-implement
- **Merge strategy** and **base branch** are configured together in `[source_control]`

## Example dialogue

> **Dev:** "So the `clear` subcommand calls `complete_ticket` directly?"
> **Domain expert:** "No — the **subcommand** triggers the `clear` **workflow**, which calls `execute_ordered`, which calls `complete_ticket`. The **subcommand** doesn't know about `complete_ticket` at all."
> **Dev:** "And `complete_ticket` — is that a **behavior** or a **workflow**?"
> **Domain expert:** "A **behavior**. It groups the `implement`, `review`, and `generate_test_instructions` **actions** together. It's composable — `execute_ordered` calls it, and so does `complete_feature`. No **subcommand** calls it directly."
> **Dev:** "What about `init`? That's both a **subcommand** and a function name."
> **Domain expert:** "The **subcommand** triggers `init_workflow`. That workflow calls `scaffold_intern_directory`, which is the **behavior**. Today there's one behavior in that workflow. When we add the interactive config wizard, that becomes a second **behavior** in the same `init` **workflow** — the **subcommand** doesn't change."
> **Dev:** "So the rule is: **subcommands** map to **workflows**, **workflows** compose **behaviors**, **behaviors** compose **actions**?"
> **Domain expert:** "Exactly. And **behaviors** can also compose other **behaviors** — `complete_feature` calls `execute_ordered`, which calls `complete_ticket`. All of that is the **behavior** layer."

## Flagged ambiguities

- **"issue" vs "ticket"**: Both are valid. **Ticket** is the broader colloquial term for any documented work item (in this project or a target repo). **Issue** is the GitHub-specific representation. In the codebase, `Ticket` as an `IssueType` variant means a leaf node — distinct from `Feature`. Context resolves the ambiguity.
- **"command" vs "subcommand"**: Use **subcommand** — `intern` is the binary, `implement`/`clear`/`init` are subcommands. "Command" is too broad.
- **"commit_strategy" vs "merge_strategy"**: The config field is being renamed from `commit_strategy` to `merge_strategy`. Avoid `commit_strategy` going forward — it conflates commit frequency (owned by the agent) with branch topology (owned by the orchestrator). Use **merge strategy**.
- **"integration_strategy"**: Was considered as a rename candidate for `commit_strategy`. Rejected in favour of **merge strategy**. Do not use.
