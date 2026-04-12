{{repo_context}}

## Feature Review: Issue #{{issue_id}}

**{{issue_title}}**

### Acceptance criteria
{{issue_body}}

### Aggregate changes
```diff
{{diff}}
```

Evaluate the implementation holistically. Check for:
- Coherence: no duplicate code paths, shared patterns are consistent across all tickets
- Completeness: the sum of all changes delivers what the feature described — flag any gaps
- Cross-cutting concerns: integration tests that span multiple tickets, refactor opportunities that only become visible at this scale

**For findings directly caused by this feature** (bugs, gaps, missing tests, refactor candidates within this scope):
Create a single child issue on #{{issue_id}} consolidating all in-scope findings.

**For observations not caused by this feature** (broader architectural concerns, unrelated tech debt surfaced during review):
Create a top-level issue labeled `hitl` with a reference to feature #{{issue_id}}.

After creating any issues, output your result on the last line:
- If there are in-scope findings: <featureReviewResult>IN_SCOPE_FINDINGS</featureReviewResult>
- If the feature is complete and coherent: <featureReviewResult>CLEAN</featureReviewResult>
