#!/usr/bin/env python3
"""Client MCP minimale per Concat (stdio), usato per montare il tutorial.

Uso:
  python3 concat_mcp.py tools
  python3 concat_mcp.py call clip_add '{"path":"...","mediaId":"...","trackId":"...","start":0}'
"""
import json
import subprocess
import sys

CONCAT = "/Applications/Concat.app/Contents/MacOS/concat"


class Concat:
    def __init__(self):
        self.proc = subprocess.Popen(
            [CONCAT, "--mcp", "headless"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, bufsize=1,
        )
        self.next_id = 1
        self.request("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "caa-tutorial", "version": "1.0"},
        })
        self.notify("notifications/initialized", {})

    def _send(self, payload):
        self.proc.stdin.write(json.dumps(payload) + "\n")
        self.proc.stdin.flush()

    def _read(self):
        while True:
            line = self.proc.stdout.readline()
            if not line:
                raise RuntimeError("Concat MCP ha chiuso lo stream")
            line = line.strip()
            if not line:
                continue
            return json.loads(line)

    def notify(self, method, params):
        self._send({"jsonrpc": "2.0", "method": method, "params": params})

    def request(self, method, params):
        request_id = self.next_id
        self.next_id += 1
        self._send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        while True:
            message = self._read()
            if message.get("id") == request_id:
                if "error" in message:
                    raise RuntimeError(json.dumps(message["error"], ensure_ascii=False))
                return message.get("result", {})

    def tools(self):
        return self.request("tools/list", {})

    def call(self, name, arguments):
        result = self.request("tools/call", {"name": name, "arguments": arguments})
        content = result.get("content") or []
        raw = "".join(part.get("text", "") for part in content if part.get("type") == "text")
        if result.get("isError"):
            raise RuntimeError(raw or "errore Concat")
        try:
            return json.loads(raw)
        except json.JSONDecodeError:
            return raw

    def close(self):
        self.proc.stdin.close()
        self.proc.terminate()
        self.proc.wait(timeout=10)


if __name__ == "__main__":
    client = Concat()
    try:
        if len(sys.argv) >= 2 and sys.argv[1] == "tools":
            for tool in client.tools()["tools"]:
                print(tool["name"])
        elif len(sys.argv) >= 3 and sys.argv[1] == "call":
            args = json.loads(sys.argv[3]) if len(sys.argv) > 3 else {}
            print(json.dumps(client.call(sys.argv[2], args), ensure_ascii=False, indent=2))
        else:
            print(__doc__)
    finally:
        client.close()
