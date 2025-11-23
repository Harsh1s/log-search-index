# operating-rules.md

Complete rules for working on this project. Rules 1–12 are from CLAUDE.md;
rules 13+ are derived from discovered constraints and past incidents.

## Rules

1. **Plan before any non-trivial code.** Write a plan, get explicit approval,
   then execute milestone-by-milestone. Code written before a plan is approved
   will be discarded.

2. **Full files only.** Never deliver diffs or partial snippets with
   "...rest unchanged...". Every changed file ships as its complete new content.
   Use the Write tool for full rewrites; Edit tool for targeted, bounded changes.

3. **Zero placeholders, zero TODOs, zero "// for now" stubs.** If you cannot
   finish something, surface the blocker. Do not paper over it.

4. **One patch per error report.** Fix exactly what was reported. Surface other
   findings as separate items rather than silently fixing them.

5. **Don't invent scope.** If the user didn't ask for it, don't add it.
   A bug fix doesn't need a refactor. An endpoint doesn't need its own module
   if it fits in the existing one.

6. **Flag design decisions upfront.** List options, recommend one with
   justification, get approval, then write code.

7. **Compile checkpoint after every meaningful change.** All four must pass
   before moving to the next file:
   ```
   cargo build --workspace
   cargo test --workspace --no-fail-fast
   cargo clippy --workspace --all-targets -- -D warnings
