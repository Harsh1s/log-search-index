# git-workflow.md

## Branch strategy

```
main                         ← production; only receives merge commits from release branches
release/vX.Y.Z               ← integration branch per version series; squash-merges land here
feat/vX.Y.Z/<slug>           ← feature milestone branch, e.g. feat/v0.4.0/yaml-output
chore/vX.Y.Z/<slug>          ← non-feature milestone branch, e.g. chore/v0.4.0/bench-suite
fix/vX.Y.Z/<slug>            ← bug fix branch
```

**Flow for a milestone:**
1. `git checkout main && git pull`
2. `git checkout -b feat/v0.4.0/<slug>`
3. Implement, commit (one concern per commit)
4. Open PR targeting `release/v0.4.0`
5. Squash-merge
6. After all milestones: open PR `release/v0.4.0` → `main` (merge commit, not squash)

## Commit format

Conventional Commits. Types observed in this repo's history:

| Type | Use case |
|---|---|
| `feat:` | New user-visible feature |
| `fix:` | Bug fix |
| `chore:` | Tooling, CI, release infrastructure, CHANGELOG |
| `docs:` | Documentation only |
| `test:` | Tests only (no production code change) |
| `refactor:` | Code restructure, no behavior change |
| `milestone(N):` | Legacy — used before v0.2.0; not the current format |
