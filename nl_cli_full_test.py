#!/usr/bin/env python3
import json
import subprocess
import time
import urllib.request
import urllib.error

SYSTEM_PROMPT = r'''You are OpenClaw NL Agent, an AI that converts natural language instructions into structured action plans.

Given a user instruction, respond ONLY with a valid JSON array of action steps. Each step has:
- "type": one of "shell", "fetch", "analyze", "report"
- "description": brief human-readable description of this step
- "command": (for shell) the shell command to run
- "url": (for fetch) the URL to fetch
- "input": (for analyze) what to analyze (reference previous step output as "$prev")
- "message": (for report) the final summary message

Rules:
1. Keep steps minimal and focused
2. For web research: use fetch + analyze + report
3. For system tasks: use shell steps
4. For data collection: combine fetch/shell + analyze + report
5. Maximum 6 steps per plan
6. Shell commands must be safe (no rm -rf, no destructive ops without explicit user request)

Example for "查看系统信息":
[{"type":"shell","description":"Get system info","command":"uname -a && sw_vers 2>/dev/null || lsb_release -a 2>/dev/null"},{"type":"report","description":"Summary","message":"System information collected above."}]

Example for "搜索 Rust 最新版本":
[{"type":"fetch","description":"Fetch Rust releases page","url":"https://api.github.com/repos/rust-lang/rust/releases/latest"},{"type":"analyze","description":"Extract version","input":"$prev"},{"type":"report","description":"Summary","message":"Latest Rust version extracted above."}]

Respond ONLY with the JSON array, no markdown, no explanation.'''

TEST_CASES = [
    ("系统信息", "请查看当前系统信息，包括操作系统版本和机器架构。"),
    ("当前目录", "请告诉我当前项目所在目录，并列出前 10 个文件。"),
    ("Git 状态", "请检查当前仓库的 git 状态，并总结是否有未提交修改。"),
    ("Rust 环境", "请检查当前 Rust 和 Cargo 版本，并告诉我是否可用。"),
    ("代码统计", "请统计当前项目里 Rust 源文件数量，并统计 crates 目录总代码行数。"),
    ("WasmEdge 版本", "请检查 WasmEdge 是否安装，并显示版本信息。"),
    ("WasmEdge QuickJS 资源", "请检查项目里是否存在 wasmedge_quickjs.wasm，并告诉我路径。"),
    ("WasmEdge 简化测试", "请运行 tests/test_wasmedge_simple.sh，并总结结果。"),
    ("WasmEdge 沙箱测试", "请运行 tests/test_wasmedge_sandbox.sh，并总结失败原因。"),
    ("Gateway 健康", "请检查 http://localhost:8787 的健康状态，如果不可达也请明确说明。"),
    ("最新 Rust 版本", "请帮我搜索 Rust 最新发布版本，并提取版本信息。"),
    ("OpenClaw 技能状态", "请检查 OpenClaw Gateway 的技能状态接口 http://localhost:8787/skills/status ，如果失败就报告原因。"),
    ("天气查询", "请查询北京当前天气，并给出简短总结。"),
    ("新闻查询", "请搜索最新的 AI 相关新闻，并提炼 3 条重点。"),
]


def clean_json_text(text: str) -> str:
    text = text.strip()
    if text.startswith("```json"):
        text = text[len("```json"):]
    elif text.startswith("```"):
        text = text[len("```"):]
    if text.endswith("```"):
        text = text[:-3]
    return text.strip()


def ollama_chat(model: str, system: str, user: str) -> str:
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "stream": False,
    }
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        "http://localhost:11434/api/chat",
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=360) as resp:
        body = json.loads(resp.read().decode("utf-8"))
        return body["message"]["content"]


def warmup_model(model: str):
    try:
        _ = ollama_chat(model, "You are a helpful assistant.", "Reply with exactly: warm")
    except Exception as e:
        print(f"模型预热失败: {e}", flush=True)


def execute_shell(command: str) -> tuple[str, bool]:
    result = subprocess.run(["/bin/sh", "-c", command], capture_output=True, text=True)
    out = result.stdout.strip()
    err = result.stderr.strip()
    text = out
    if err:
        text = (text + "\n" + err).strip()
    if not text:
        text = "(no output)"
    if result.returncode != 0:
        text += f"\n[exit_code={result.returncode}]"
    return text, result.returncode != 0


def execute_fetch(url: str) -> tuple[str, bool]:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "OpenClaw-NL-Agent/0.1"})
        with urllib.request.urlopen(req, timeout=20) as resp:
            body = resp.read().decode("utf-8", errors="replace")
            text = f"HTTP {resp.status}\n{body}"
            return text[:4000], False
    except Exception as e:
        return str(e), True


def execute_analyze(input_text: str) -> tuple[str, bool]:
    lines = [line for line in input_text.splitlines() if line.strip()]
    summary = "\n".join(lines[:20]) if lines else "(no analyzable content)"
    return summary, False


def print_block(title: str):
    print("\n" + "=" * 79, flush=True)
    print(title, flush=True)
    print("=" * 79, flush=True)


def main():
    model = "qwen3.5:9b"
    total = len(TEST_CASES)
    passed = 0
    failed = 0

    print_block("OpenClaw CLI Natural Language Full Test")
    print(f"Model: {model}", flush=True)
    print(f"Cases: {total}", flush=True)
    print("预热模型中...", flush=True)
    warmup_model(model)
    print("模型预热完成", flush=True)

    for idx, (name, prompt) in enumerate(TEST_CASES, start=1):
        print_block(f"[{idx}/{total}] {name}")
        print("自然语言输入:", flush=True)
        print(prompt, flush=True)

        start = time.time()
        case_failed = False
        try:
            plan_raw = ollama_chat(model, SYSTEM_PROMPT, prompt)
            cleaned = clean_json_text(plan_raw)
            print("\n生成计划:", flush=True)
            print(cleaned, flush=True)
            steps = json.loads(cleaned)
            if not isinstance(steps, list):
                raise ValueError("plan is not a JSON array")
        except Exception as e:
            print("\n计划生成失败:", flush=True)
            print(str(e), flush=True)
            failed += 1
            continue

        prev_output = ""
        for step_index, step in enumerate(steps, start=1):
            kind = step.get("type", "")
            desc = step.get("description", "")
            print(f"\nStep {step_index}: {kind} - {desc}", flush=True)
            if kind == "shell":
                command = step.get("command", "")
                print(f"$ {command}", flush=True)
                output, is_err = execute_shell(command)
            elif kind == "fetch":
                url = step.get("url", "")
                print(f"GET {url}", flush=True)
                output, is_err = execute_fetch(url)
            elif kind == "analyze":
                input_text = step.get("input", "")
                if "$prev" in input_text:
                    input_text = input_text.replace("$prev", prev_output)
                elif not input_text:
                    input_text = prev_output
                output, is_err = execute_analyze(input_text)
            elif kind == "report":
                output = step.get("message", "Task completed.")
                is_err = False
            else:
                output = f"Unknown step type: {kind}"
                is_err = True

            prev_output = output
            print("输出:", flush=True)
            print(output[:3000], flush=True)
            if len(output) > 3000:
                print("...[truncated]", flush=True)
            if is_err:
                case_failed = True

        elapsed_ms = int((time.time() - start) * 1000)
        if case_failed:
            print(f"\n结果: FAIL  ({elapsed_ms}ms)", flush=True)
            failed += 1
        else:
            print(f"\n结果: PASS  ({elapsed_ms}ms)", flush=True)
            passed += 1

    print_block("Summary")
    print(f"Total:  {total}", flush=True)
    print(f"Passed: {passed}", flush=True)
    print(f"Failed: {failed}", flush=True)
    rate = 0 if total == 0 else passed * 100.0 / total
    print(f"Success Rate: {rate:.1f}%", flush=True)


if __name__ == "__main__":
    main()
