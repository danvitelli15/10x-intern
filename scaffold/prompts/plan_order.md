<strip-before-prompting>
# Available variables
# {{issues_list}} — formatted list of all issues to be ordered
</strip-before-prompting>

You are planning the execution order for a set of work items.

## Work items
{{issues_list}}

Analyze the work items and determine the optimal execution order. Consider:
- Explicit dependencies (one item references another as a predecessor)
- Implicit dependencies (one item must logically precede another to avoid rework)
- Risk: items that establish shared patterns should come before items that use them

Respond with ONLY a JSON array of objects in the order they should be executed. No other text.

Each object must have an `id` field containing the issue number. Additional fields may be added in the future.

Example: [{"id": 3}, {"id": 1}, {"id": 2}]
