---
description: Initialize Claude-Factory in the current project directory. Creates .claude-factory/ state directory and registers the six phases.
allowed-tools: Bash, mcp__claude-factory__cf_init, mcp__claude-factory__cf_status
argument-hint: "[--product-name <name>]"
---

Initialize Claude-Factory in the current project ($ARGUMENTS).

1. Check that `cf_status` does not already show an initialized project (avoid re-initializing).
2. Call `cf_init` with the current project directory and any provided product name.
3. Display the resulting status — the factory is ready to begin with `/claude-factory:discover`.

If the project already has a `.claude-factory/` directory, display the current status instead.
