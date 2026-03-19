#!/bin/bash
# AgentKit: Validate commit messages match Angular convention
# Called by Claude Code PreToolUse hook

INPUT=$(cat -)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# Only check git commit commands
if echo "$COMMAND" | grep -q "git commit"; then
    MSG=$(echo "$COMMAND" | grep -oP '(?<=-m ")[^"]*')
    if [ -n "$MSG" ]; then
        PATTERN="^(feat|fix|refactor|test|docs|chore|build|ci)(\(.+\)): .+"
        if ! echo "$MSG" | grep -qE "$PATTERN"; then
            echo "Commit message does not match Angular convention." >&2
            echo "Expected: <type>(<scope>): <description>" >&2
            echo "Allowed types: feat|fix|refactor|test|docs|chore|build|ci" >&2
            exit 2
        fi
    fi
fi

exit 0
