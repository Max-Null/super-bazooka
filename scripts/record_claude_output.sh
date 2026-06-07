#!/usr/bin/env bash
# Record real claude CLI stream-json output to a fixture file.
# Usage: bash scripts/record_claude_output.sh "Your prompt here"
#
# This captures real, un-mocked stream-json events that can be replayed
# in Playwright E2E tests against the frontend.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PROMPT="${1:-Say hello in 3 words.}"
FIXTURE="${SCRIPT_DIR}/e2e/fixtures/real-stream-$(date +%s).jsonl"

echo "🔴 Recording claude output for: $PROMPT"
echo "📁 Fixture: $FIXTURE"

claude --print \
  --output-format stream-json \
  --verbose \
  --dangerously-skip-permissions \
  --max-turns 2 \
  -p "$PROMPT" > "$FIXTURE" 2>/dev/null

COUNT=$(wc -l < "$FIXTURE")
echo "✅ Recorded $COUNT stream-json events"
echo "📁 $FIXTURE"
