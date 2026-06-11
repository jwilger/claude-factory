---
description: Show the current Claude-Factory status — phase WIP, ready work items, blocked items, and open escalations.
allowed-tools: mcp__claude-factory__cf_status, mcp__claude-factory__cf_backlog
---

Display the Claude-Factory status for the current project.

1. Call `cf_status` and present the dashboard in a readable format.
2. If there are open escalations requiring human decisions, highlight them prominently.
3. If there is ready work, note that `/claude-factory:work` will pick it up.
4. If idle with no ready work, suggest what phase might need attention next.
