#!/bin/bash
# AgentKit: Log tool usage for activity tracking
# Called by Claude Code PostToolUse hook

INPUT=$(cat -)
TOOL=$(echo "$INPUT" | jq -r '.tool_name // "unknown"')
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

echo "$TIMESTAMP | $TOOL" >> .agentkit/agent-activity.log

exit 0
