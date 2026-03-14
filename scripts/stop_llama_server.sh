#!/bin/bash
echo "🛑 停止 llama-server..."
pkill -f "llama-server.*8080"
echo "✅ 已停止"
