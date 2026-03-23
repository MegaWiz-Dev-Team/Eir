#!/usr/bin/env python3
"""
Integration test: Bifrost MCPClient → Eir MCP Server

Simulates Bifrost's MCPClient calling Eir's /api/mcp endpoint.
Run this when Eir is serving to verify the full MCP flow.

Usage:
    python3 tests/test_mcp_integration.py --url http://localhost:9090/api/mcp

Or set environment variable:
    EIR_MCP_URL=http://localhost:9090/api/mcp python3 tests/test_mcp_integration.py
"""

import os
import sys
import json
import argparse
import urllib.request


def jsonrpc_request(url: str, method: str, params: dict = None) -> dict:
    """Send JSON-RPC request to MCP server."""
    body = json.dumps({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params or {},
    }).encode()

    req = urllib.request.Request(
        url,
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read().decode())
    except Exception as e:
        return {"error": str(e)}


def test_initialize(url: str) -> bool:
    """Test MCP initialize handshake."""
    resp = jsonrpc_request(url, "initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "test", "version": "1.0"},
    })
    ok = "result" in resp and resp["result"].get("serverInfo", {}).get("name") == "eir-openemr"
    print(f"  {'✅' if ok else '❌'} initialize → {resp.get('result', {}).get('serverInfo', resp.get('error', '?'))}")
    return ok


def test_tools_list(url: str) -> bool:
    """Test tools/list discovery."""
    resp = jsonrpc_request(url, "tools/list")
    tools = resp.get("result", {}).get("tools", [])
    expected = {"search_patients", "get_patient_summary", "create_encounter", "get_sleep_reports"}
    found = {t["name"] for t in tools}
    ok = expected == found
    print(f"  {'✅' if ok else '❌'} tools/list → {len(tools)} tools: {found}")
    return ok


def test_search_patients(url: str, query: str = "สมชาย") -> bool:
    """Test search_patients tool."""
    resp = jsonrpc_request(url, "tools/call", {
        "name": "search_patients",
        "arguments": {"query": query},
    })
    content = resp.get("result", {}).get("content", [{}])[0].get("text", "")
    ok = "PID" in content and "error" not in content.lower()
    print(f"  {'✅' if ok else '❌'} search_patients(\"{query}\") → {content[:80]}...")
    return ok


def test_get_patient_summary(url: str, pid: int = 1001) -> bool:
    """Test get_patient_summary tool."""
    resp = jsonrpc_request(url, "tools/call", {
        "name": "get_patient_summary",
        "arguments": {"patient_id": pid},
    })
    content = resp.get("result", {}).get("content", [{}])[0].get("text", "")
    ok = "CPAP" in content or "ข้อมูลผู้ป่วย" in content
    print(f"  {'✅' if ok else '❌'} get_patient_summary({pid}) → {content[:80]}...")
    return ok


def test_get_sleep_reports(url: str, pid: int = 1001) -> bool:
    """Test get_sleep_reports tool."""
    resp = jsonrpc_request(url, "tools/call", {
        "name": "get_sleep_reports",
        "arguments": {"patient_id": pid, "days": 30},
    })
    content = resp.get("result", {}).get("content", [{}])[0].get("text", "")
    ok = "Sleep Report" in content or "AHI" in content or "ไม่พบ" in content
    print(f"  {'✅' if ok else '❌'} get_sleep_reports({pid}, 30d) → {content[:80]}...")
    return ok


def test_create_encounter(url: str, pid: int = 1001) -> bool:
    """Test create_encounter tool."""
    resp = jsonrpc_request(url, "tools/call", {
        "name": "create_encounter",
        "arguments": {"patient_id": pid, "type": "data_review"},
    })
    content = resp.get("result", {}).get("content", [{}])[0].get("text", "")
    ok = "สร้าง Encounter สำเร็จ" in content or "Encounter" in content
    print(f"  {'✅' if ok else '❌'} create_encounter({pid}) → {content[:80]}...")
    return ok


def main():
    parser = argparse.ArgumentParser(description="Test Eir MCP Server")
    parser.add_argument("--url", default=os.getenv("EIR_MCP_URL", "http://localhost:9090/api/mcp"))
    args = parser.parse_args()

    print(f"\n🧪 Testing Eir MCP Server: {args.url}\n")

    tests = [
        ("Initialize", lambda: test_initialize(args.url)),
        ("Tools List", lambda: test_tools_list(args.url)),
        ("Search Patients", lambda: test_search_patients(args.url)),
        ("Patient Summary", lambda: test_get_patient_summary(args.url)),
        ("Sleep Reports", lambda: test_get_sleep_reports(args.url)),
        ("Create Encounter", lambda: test_create_encounter(args.url)),
    ]

    passed = 0
    failed = 0
    for name, test_fn in tests:
        try:
            if test_fn():
                passed += 1
            else:
                failed += 1
        except Exception as e:
            print(f"  ❌ {name} → Exception: {e}")
            failed += 1

    total = passed + failed
    print(f"\n{'✅' if failed == 0 else '⚠️'} Results: {passed}/{total} passed\n")
    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    main()
