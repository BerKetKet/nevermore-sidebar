#!/usr/bin/env python3
import sys
import json

def main():
    try:
        data = json.load(sys.stdin)
        tool_call = data.get("toolCall", {})
        args = tool_call.get("args", {})
        path = args.get("Path", "") or args.get("path", "")
        
        # Block edits to AGENTS.md
        if path.endswith("AGENTS.md") or "/AGENTS.md" in path:
            print(json.dumps({
                "decision": "deny",
                "reason": "AGENTS.md is locked against automated agent modification by Supreme Rules."
            }))
            return
            
        print(json.dumps({"decision": "allow"}))
    except Exception as e:
        print(json.dumps({"decision": "allow", "reason": f"Hook bypass on error: {str(e)}"}))

if __name__ == "__main__":
    main()
