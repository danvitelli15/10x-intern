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
| **Behavior** | A named grouping of actions that accomplishes a complete user-facing goal (e.g. `complete_ticket`, `complete_feature`, `init`) | Flow, process, pipeline |
| **Workflow** | A composition of behaviors to accomplish a higher-order goal — not yet instantiated in the codebase | — |
| **Context** | The struct that carries all dependencies (issue tracker, source control, runner, config) for a single execution run | Container, service locator |
| **Budget** | The maximum number of agent invocations allowed in a run, enforced by `Context` | Limit, cap |

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

## Configuration

| Term | Definition | Aliases to avoid |
|---|---|---|
| **Work directory** | The root directory used to resolve `.intern/` paths — defaults to CWD, overridable in config | Base dir, project root |
| **Context file** | A path in config pointing to a repo description file (e.g. `CLAUDE.md`) whose contents become `repo_context` | Context, docs file |

## Relationships

- A **Feature** has one or more **child issues** (which are **Tickets** or nested **Features**)
- A **Behavior** calls one or more **Actions**; actions do not call behaviors
- A **Prompt** is produced by rendering a **Prompt template** with issue data and **repo context**
- An **Override** shadows the **Scaffold** for a specific prompt name; if no override exists, the behavior errors and instructs the user to run `intern init`
- A **Ticket** produces a **FINDINGS** or **CLEAN** signal from the review **Action**
- A **Feature** produces an **IN_SCOPE_FINDINGS** or **CLEAN** signal from the feature review **Action**

## Example dialogue

> **Dev:** "So when we say `complete_ticket`, is that a **behavior** or an **action**?"
> **Domain expert:** "A **behavior** — it groups the `implement`, `review`, and `generate_test_instructions` **actions** together into a complete user-facing goal."
> **Dev:** "And if the review comes back with **FINDINGS**, what happens?"
> **Domain expert:** "The **behavior** loops — it creates a **child issue** with the findings and runs `implement` again on the same **ticket**."
> **Dev:** "What stops it from looping forever?"
> **Domain expert:** "The **budget**. Once `Context` hits `max_iterations`, any further `run_agent` call returns `BudgetExhausted`, the **ticket** is **skipped**, and execution continues with the next item."
> **Dev:** "What's the difference between a **ticket** being **skipped** vs a **HITL** issue?"
> **Domain expert:** "A **HITL** issue is one a human intentionally flagged for manual handling — the agent never touches it. A **skipped** ticket is one the agent tried and couldn't finish within **budget**."

## Flagged ambiguities

- **"issue" vs "ticket"**: Both are valid. **Ticket** is the broader colloquial term for any documented work item (in this project or a target repo). **Issue** is the GitHub-specific representation. In the codebase, `Ticket` as an `IssueType` variant means a leaf node — distinct from `Feature`. Context resolves the ambiguity: "ticket" in conversation rarely needs to be more precise than "a documented item of work."
- **"behavior"**: not yet formalized in the codebase — lives conceptually as the pattern of functions in `orchestrator.rs`, but there is no `Behavior` trait or module yet. The user has flagged this as a forthcoming design discussion.
- **"workflow"**: mentioned as a layer above behaviors but has no instantiation yet. Do not use this term for anything that already has a name.
