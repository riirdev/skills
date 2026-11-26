# Multi-Agent & Parallel Porting Orchestration

Large rewrites fail under parallel automation when agents collide on Git state, drift from shared conventions, or burn cycles on redundant reviews instead of automated verification.

## 1. Workspace Isolation & Git Discipline

- **One Worktree per Batch/Shard**: Give each parallel worker or batch its own `git worktree` (or isolated directory).
- **Never Share a File Across Concurrent Workers**: Assign exclusive file ownership per worker. Parallelize across independent files/modules, never inside the same file.
- **Single-Unit Explicit Commits**: Stage and commit explicit file paths (`git add <file> && git commit -S`).
- **Forbidden Git Commands in Parallel Loops**: Never run `git stash`, `git reset --hard`, `git checkout --`, `git clean -fdx`, or `git push --force` while workers are active — they destroy sibling workers' uncommitted progress.
- **Read-Only Reference Guarantee**: Assert before every write or test run that the reference directory has zero modifications.

## 2. Worker Topology

For complex or subtle porting units (unsafe memory, FFI boundaries, state machines):

```text
[Unit Spec + PORTING.md + LIFETIMES.tsv]
                 │
                 ▼
         (1) Implementer
     Produces minimal unit port
                 │
                 ▼  (Diff + source contract ONLY — omit implementer's reasoning)
    (2) Independent Reviewer(s)
   Hunts for semantic bugs, UB, missed edge cases
                 │
                 ▼
          (3) Fixer / Gate
   Applies valid fixes -> runs automated inventory + parity tests
```

### Avoiding Over-Review Churn
- **Do NOT run multi-agent review on every trivial edit.** Use automated compiler gates, `riir-skills unsafe-audit`, layout assertions, unit tests, and differential suites (`riir-skills diff-run`) as the primary fast inner loop.
- Reserve **diff-only adversarial review** (in a clean context without the implementer's self-justifications) for:
  - Phase 0 blueprints (`PORTING.md` and `LIFETIMES.tsv`)
  - Pilot units
  - High-risk `unsafe` / FFI / concurrency / security boundaries
  - Batch convergence gates

## 3. Bounded Escalation (Preventing Infinite Loops)

- **3-Strike Rule**: If a single module fails compilation, parity, or review 3 times on the same blocker, stop churning. Record the exact blocker, failing test/compiler output, and hypothesis in `BLOCKED.md`, and advance independent modules.
- **Compiler Errors as Queue**: When porting tightly coupled headers/types in bulk, finish the batch translation first, then resolve `cargo check` errors crate-by-crate and dependency-first instead of pausing mid-file to guess unported types.
