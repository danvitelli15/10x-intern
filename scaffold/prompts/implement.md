<strip-before-prompting>
# Available variables
# {{issue_id}}     — the GitHub issue number
# {{issue_title}}  — the issue title
# {{issue_body}}   — the full issue body / acceptance criteria
# {{repo_context}} — contents of your context_file (e.g. CLAUDE.md)
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

When complete:
- Stage and commit your changes with a message referencing issue #{{issue_id}}
