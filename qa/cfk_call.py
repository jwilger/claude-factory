#!/usr/bin/env python3
"""One-shot MCP client for driving a cfk kernel bound to an arbitrary project.

The in-session MCP server is bound to the claude-factory repo, so QA runs need a
separate cfk pointed at the test product. cfk persists all state to its on-disk
event store on every call, so a fresh process per tool call is correct: state
carries over between invocations via the event store.

Usage:
    cfk_call.py <project_dir> <tool_name> [json_args]

Examples:
    cfk_call.py /tmp/prod cf_status
    cfk_call.py /tmp/prod cf_backlog_add '{"phase":"architecture",
        "work_type":"AdrDrafting","description":"Pick a datastore"}'

Prints the tool's structured result (parsed from the MCP content) to stdout as
JSON, or an {"error": ...} object with a non-zero exit code on failure.

The cfk binary is resolved from $CFK_BIN, else the build-on-demand cache, else a
prebuilt fallback.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

PROTOCOL_VERSION = "2024-11-05"


def resolve_cfk() -> str:
    env = os.environ.get("CFK_BIN")
    if env and Path(env).is_file():
        return env
    here = Path(__file__).resolve().parent.parent
    for candidate in (
        here / "plugins/claude-factory/.bin/cfk",
        here / "plugins/claude-factory/bin/cfk",
    ):
        if candidate.is_file():
            return str(candidate)
    print(json.dumps({"error": "cfk binary not found; set $CFK_BIN"}), flush=True)
    sys.exit(2)


def main() -> int:
    if len(sys.argv) < 3:
        print(json.dumps({"error": "usage: cfk_call.py <project_dir> <tool> [json_args]"}))
        return 2

    project_dir = sys.argv[1]
    tool_name = sys.argv[2]
    tool_args = json.loads(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3].strip() else {}

    requests = [
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": "cfk-qa-harness", "version": "1.0"},
            },
        },
        {"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}},
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {"name": tool_name, "arguments": tool_args},
        },
    ]
    stdin = "".join(json.dumps(r) + "\n" for r in requests)

    proc = subprocess.run(
        [resolve_cfk(), project_dir],
        input=stdin,
        capture_output=True,
        text=True,
        timeout=120,
    )

    # Find the JSON-RPC response with id == 2 (the tool call).
    response = None
    for line in proc.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("id") == 2:
            response = msg
            break

    if response is None:
        print(json.dumps({"error": "no tool response", "stderr": proc.stderr[-2000:]}))
        return 1

    if "error" in response:
        print(json.dumps({"error": response["error"]}))
        return 1

    # Unwrap the MCP tool result: result.content[].text (JSON or plain text).
    result = response.get("result", {})
    texts = [c.get("text", "") for c in result.get("content", []) if c.get("type") == "text"]
    blob = "".join(texts)
    is_error = result.get("isError", False)
    try:
        parsed = json.loads(blob)
    except json.JSONDecodeError:
        parsed = blob
    print(json.dumps({"isError": is_error, "result": parsed}))
    return 1 if is_error else 0


if __name__ == "__main__":
    sys.exit(main())
