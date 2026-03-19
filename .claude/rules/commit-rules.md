---
description: Commit convention rules
---

# Commit Rules

## Convention: Angular

All commits must follow the Angular commit convention:
`<type>(<scope>): <description>`

### Allowed Types
feat, fix, refactor, test, docs, chore, build, ci

### Scope Policy
Scope is **required** on every commit.

### Branch Naming
Pattern: `{type}/{ticket}-{description}`
Example: `feature/PROJ-123-add-user-auth`

### Co-Authorship
**Never** add `Co-Authored-By` lines to commits. All commits are authored solely by the committer.

### Breaking Changes
Use `BREAKING CHANGE:` footer or `!` after type/scope for breaking changes.
