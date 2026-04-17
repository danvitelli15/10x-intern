<strip-before-prompting>
# Available variables
# {{issue_id}}     — the GitHub issue number
# {{issue_title}}  — the issue title
# {{issue_body}}   — the full issue body / acceptance criteria
# {{repo_context}} — contents of your context_file (e.g. CLAUDE.md)
#
# Note on custom prompts: if you override this file, you take ownership of
# the commit instructions below. The orchestrator validates that commits were
# made after your run completes — if your custom prompt omits commit
# instructions, that post-implement check will fire.
# Do NOT include branch creation instructions — the orchestrator creates the
# branch before invoking the agent.
</strip-before-prompting>

{{repo_context}}

## Task: Implement Issue #{{issue_id}}

**{{issue_title}}**

{{issue_body}}

### Instructions

Before writing any code:
1. Explore the codebase to understand the existing structure and patterns relevant to this issue
2. Identify where changes need to be made and what existing code they interact with

When implementing:
- Follow the patterns and conventions you observe in the existing code
- Keep changes focused on what the issue requires — do not refactor unrelated code
- If the issue requires tests, match the style and location of existing tests
- Commit your work incrementally as you go — make a logical commit each time you complete a meaningful unit of work (e.g. after adding a new function, after making tests pass, after completing a distinct step)

When complete:
- Ensure all changes are committed — stage and commit anything not yet committed, referencing issue #{{issue_id}} in the message
