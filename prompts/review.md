# Available variables
# {{issue_id}}    — the GitHub issue number
# {{issue_title}} — the issue title
# {{issue_body}}  — the full issue body / acceptance criteria
# {{diff}}        — the git diff since the base branch

Please review the implementation of the following GitHub issue:

## Issue #{{issue_id}}: {{issue_title}}

### Acceptance criteria
{{issue_body}}

### Changes made
```diff
{{diff}}
```

Evaluate whether the implementation satisfies the acceptance criteria. Also check for:
- Style guide violations
- Obvious testing gaps
- Obvious refactor candidates

If there are issues to address, create a single GitHub issue consolidating all findings as a child of issue #{{issue_id}}, then output the following on the last line of your response:
<reviewResult>FINDINGS</reviewResult>

If the implementation is satisfactory, output the following on the last line of your response:
<reviewResult>CLEAN</reviewResult>
