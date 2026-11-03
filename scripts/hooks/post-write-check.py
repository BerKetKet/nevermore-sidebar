#!/usr/bin/env python3
import sys
import json
import subprocess

def main():
    try:
        data = json.load(sys.stdin)
        # Check workspace formatting
        subprocess.run(["cargo", "fmt"], cwd="/home/bgg/projects/nevermore-sidebar", capture_output=True)
    except Exception:
        pass
    print(json.dumps({}))

if __name__ == "__main__":
    main()
