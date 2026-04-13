<strip-before-prompting>
# Available variables
# {{issue_id}}     — the GitHub issue number
# {{issue_title}}  — the issue title
# {{issue_body}}   — the full issue body / acceptance criteria
# {{diff}}         — the git diff since the base branch
# {{repo_context}} — contents of your context_file (e.g. CLAUDE.md)
</strip-before-prompting>

{{repo_context}}

## Test Instructions: Issue #{{issue_id}}

**{{issue_title}}**

### Acceptance criteria
{{issue_body}}

### Changes made
```diff
{{diff}}
```

Write clear, step-by-step instructions for a human to manually verify that the implementation is correct. Focus on what a reviewer would need to do to confirm each acceptance criterion is met. Note any side effects or indirect changes visible in the diff that should also be verified.

Post these instructions as a comment on issue #{{issue_id}}.
