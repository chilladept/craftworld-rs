#!/usr/bin/env python3
"""Where did the context go? Summarise Claude Code transcripts for this project.

Usage: python -I scripts/ctx_audit.py [--last N] [--dir PROJECT_TRANSCRIPT_DIR] [--session ID_PREFIX]

Prints (capped, ~60 lines): per-session baseline/peak/turns, and the context growth
attributed to each tool-result class. Token numbers for results are estimates
(chars/3.5, images 1500); session totals come from the API usage fields.
"""
import argparse, collections, glob, json, os, re

CLASSES = [
    (r"pose_fit", "pose_fit"), (r"cargo (test|clippy|build|run|fmt|check)", "cargo"),
    (r"rpcs3_v|rpcs3", "rpcs3-tools"), (r"pad\.py|keep\.py", "pad/keep"),
    (r"screenshot|capture", "capture"), (r"git (status|log|worktree|fetch|pull|diff|show)", "git"),
    (r"gh (pr|run|api)", "gh"), (r"\b(cat|sed -n|head|tail|Get-Content|type)\b", "file-dump"),
    (r"\b(rg|grep|Select-String)\b", "grep"), (r"python", "python-script"),
]


def klass(name, cmd):
    if name in ("Bash", "PowerShell"):
        for pat, k in CLASSES:
            if re.search(pat, cmd):
                return f"{name}:{k}"
        return f"{name}:other"
    return name


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--last", type=int, default=20)
    ap.add_argument("--dir", default=None)
    ap.add_argument("--session", help="id prefix (scratchpad folder name): print its current context only")
    a = ap.parse_args()
    home = os.path.expanduser("~")
    folder = "".join(c if c.isalnum() else "-" for c in os.path.join(home, "Documents", "Projects", "LBP2 Rust Rewrite"))
    d = a.dir or os.path.join(home, ".claude", "projects", folder)
    if a.session:
        f = max(glob.glob(os.path.join(d, a.session + "*.jsonl")), key=os.path.getmtime)
        ids, cur = set(), 0
        for line in open(f, encoding="utf8"):
            if '"usage"' not in line:
                continue
            m = json.loads(line).get("message") or {}
            u = m.get("usage") or {}
            ids.add(m.get("id"))
            cur = sum(u.get(k) or 0 for k in ("cache_read_input_tokens", "input_tokens", "cache_creation_input_tokens")) or cur
        print(f"{a.session}: {len(ids)} calls, context {cur // 1000}k (plan handoff at 150k, stop near 170k)")
        return
    files = sorted(glob.glob(d + "/*.jsonl"), key=os.path.getmtime)[-a.last:]
    grow, cnt = collections.Counter(), collections.Counter()
    print("session  turns  first   peak   cache-read-sum(M)")
    tb = ts = 0
    for f in files:
        uses, last, pend, ctxs = {}, None, [], []
        for line in open(f, encoding="utf8"):
            try:
                e = json.loads(line)
            except ValueError:
                continue
            m = e.get("message") or {}
            if e.get("type") == "assistant":
                u = m.get("usage") or {}
                c = sum(u.get(k) or 0 for k in ("cache_read_input_tokens", "input_tokens", "cache_creation_input_tokens"))
                for b in m.get("content") or []:
                    if b.get("type") == "tool_use":
                        i = b.get("input", {})
                        uses[b["id"]] = (b["name"], str(i.get("command") or i.get("file_path") or i.get("prompt") or i)[:300])
                if c:
                    ctxs.append(c)
                    if last is not None and c > last and pend:
                        w = sum(x[1] for x in pend) or 1
                        for k, wt in pend:
                            grow[k] += (c - last) * wt / w
                            cnt[k] += 1
                    last, pend = c, []
            elif e.get("type") == "user" and isinstance(m.get("content"), list):
                for b in m["content"]:
                    if b.get("type") != "tool_result":
                        continue
                    n, s = uses.get(b.get("tool_use_id"), ("?", ""))
                    x, w = b.get("content"), 0
                    if isinstance(x, str):
                        w = len(x) / 3.5
                    else:
                        for y in x or []:
                            w += len(y.get("text", "")) / 3.5 if y.get("type") == "text" else 1500
                    pend.append((klass(n, s), max(w, 20)))
        if ctxs:
            tb += ctxs[0] * len(ctxs)
            ts += sum(ctxs)
            print(f"{os.path.basename(f)[:8]} {len(ctxs):5d} {ctxs[0]:7d} {max(ctxs):7d} {sum(ctxs)/1e6:8.1f}")
    if ts:
        print(f"baseline share of all cache-read tokens: {100*tb/ts:.0f}%")
    print("growth attributed to tool-result class (k tokens, calls, tokens/call)")
    for k, v in grow.most_common(18):
        print(f"{k:28s} {int(v)//1000:5d}k {cnt[k]:5d} {int(v)/max(cnt[k],1):6.0f}")


if __name__ == "__main__":
    main()
