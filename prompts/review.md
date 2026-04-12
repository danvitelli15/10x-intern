{{repo_context}}

## Review: Issue #{{issue_id}}

**{{issue_title}}**

### Acceptance criteria
{{issue_body}}

### Changes made
```diff
{{diff}}
```

Review the changes against the acceptance criteria. Check for:
- Gaps: does the implementation fully satisfy every acceptance criterion?
- Correctness: are there bugs, edge cases, or error paths not handled?
- Consistency: do the changes follow the patterns established in the rest of the codebase?

Do not flag pre-existing issues unrelated to this change.

If there are issues to address, create a single GitHub issue consolidating all findings as a child of issue #{{issue_id}}, then output on the last line:
<reviewResult>FINDINGS</reviewResult>

If the implementation is satisfactory, output on the last line:
<reviewResult>CLEAN</reviewResult>
