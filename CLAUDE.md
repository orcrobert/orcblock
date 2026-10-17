@AGENTS.md

## Claude Code
- Slash commands: /feature, /build, /bug, /ship, /status. The verify skill runs before every commit on its own.
- Delegate reviews to the code-reviewer, security-reviewer and ui-reviewer subagents; never approve your own diff in-context.
- Run parallel tasks in separate worktrees: start each session with claude --worktree.
- Put anything a future session needs into AGENTS.md, a skill or a spec, not only into auto memory.
