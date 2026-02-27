#!/usr/bin/env bash
#
# Health check for AI inference services
# Checks all configured backends and reports status
#
set -euo pipefail

echo "=== AI Inference Services Health Check ==="
echo ""
date
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check Ollama (Primary)
echo -n "Ollama (primary, :11434):    "
if curl -s --max-time 2 http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Running${NC}"
    OLLAMA_STATUS=0
else
    echo -e "${RED}❌ Down${NC}"
    OLLAMA_STATUS=1
fi

# Check llama.cpp server (Backup)
echo -n "llama.cpp (backup, :8080):   "
if curl -s --max-time 2 http://localhost:8080/health > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Running${NC}"
    LLAMA_STATUS=0
else
    echo -e "${YELLOW}⚠️  Down (backup not critical)${NC}"
    LLAMA_STATUS=1
fi

# Check remote server (if configured)
REMOTE_ENDPOINT="${REMOTE_INFERENCE_ENDPOINT:-}"
if [ -n "$REMOTE_ENDPOINT" ]; then
    echo -n "Remote server:               "
    if curl -s --max-time 5 "$REMOTE_ENDPOINT/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Running${NC}"
        REMOTE_STATUS=0
    else
        echo -e "${YELLOW}⚠️  Down or not configured${NC}"
        REMOTE_STATUS=1
    fi
else
    echo -n "Remote server:               "
    echo -e "${YELLOW}⚠️  Not configured${NC}"
    REMOTE_STATUS=1
fi

echo ""

# Summary
if [ $OLLAMA_STATUS -eq 0 ]; then
    echo -e "${GREEN}✅ System healthy: Primary service running${NC}"
    exit 0
elif [ $LLAMA_STATUS -eq 0 ]; then
    echo -e "${YELLOW}⚠️  System degraded: Running on backup${NC}"
    exit 0
elif [ $REMOTE_STATUS -eq 0 ]; then
    echo -e "${YELLOW}⚠️  System degraded: Running on remote${NC}"
    exit 0
else
    echo -e "${RED}❌ System down: All services unavailable${NC}"
    exit 1
fi
