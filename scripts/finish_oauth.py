#!/usr/bin/env python3
import socket
import json
import urllib.parse
import urllib.request
import time
import subprocess
import threading
import sys

CLIENT_ID = "980304884-4tvur12b436av3h6mggami7rfk8l5bra.apps.googleusercontent.com"
CLIENT_SECRET = subprocess.run(
    ["secret-tool", "lookup", "service", "nevermore", "plugin", "email", "key", "client_secret"],
    capture_output=True, text=True, check=True
).stdout.strip()
REDIRECT_URI = "http://127.0.0.1:8999"
SESSION_ID = "19B2F626608D54BAD8427E81C5644144"
TARGET_ID = "58B4C5BFC8DEAC5F07E6CF69F17140EA"

auth_code = None
code_event = threading.Event()

def http_listener():
    global auth_code
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", 8999))
    server.listen(1)
    print("[HTTP] Listening on 127.0.0.1:8999...")
    conn, _ = server.accept()
    data = conn.recv(4096).decode("utf-8", errors="replace")
    first_line = data.split("\r\n")[0]
    print(f"[HTTP] Received request: {first_line}")
    if "GET /?code=" in first_line:
        raw_code = first_line.split("GET /?code=")[1].split(" ")[0].split("&")[0]
        auth_code = urllib.parse.unquote(raw_code)
        resp = (
            "HTTP/1.1 200 OK\r\n"
            "Content-Type: text/html\r\n\r\n"
            "<html><body><h1>Nevermore Authorization Successful!</h1><p>You can close this tab and return to Nevermore.</p></body></html>"
        )
        conn.sendall(resp.encode("utf-8"))
        conn.close()
        server.close()
        code_event.set()
    else:
        resp = "HTTP/1.1 400 Bad Request\r\n\r\nMissing code"
        conn.sendall(resp.encode("utf-8"))
        conn.close()
        server.close()

listener_thread = threading.Thread(target=http_listener, daemon=True)
listener_thread.start()

time.sleep(1)

# Dispatch mouse click at Continue button (x=1304, y=712)
for t in ["mousePressed", "mouseReleased"]:
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect("/tmp/nevermore_bridge.sock")
    req = {
        "action": "call_cdp",
        "method": "Input.dispatchMouseEvent",
        "params": {"type": t, "x": 1304, "y": 712, "button": "left", "clickCount": 1},
        "use_session": True,
        "session_id": SESSION_ID
    }
    s.sendall(json.dumps(req).encode("utf-8"))
    c = s.recv(4096)
    s.close()

print("[OAuth] Clicked Continue, waiting for redirect code...")

if code_event.wait(timeout=20) and auth_code:
    print(f"[OAuth] Successfully acquired auth code: {auth_code[:15]}...")
    
    # Close tab
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect("/tmp/nevermore_bridge.sock")
    req = {
        "action": "call_cdp",
        "method": "Target.closeTarget",
        "params": {"targetId": TARGET_ID},
        "use_session": False
    }
    s.sendall(json.dumps(req).encode("utf-8"))
    s.recv(4096)
    s.close()
    
    # Exchange code for tokens
    print("[OAuth] Exchanging code for tokens...")
    token_url = "https://oauth2.googleapis.com/token"
    token_data = urllib.parse.urlencode({
        "code": auth_code,
        "client_id": CLIENT_ID,
        "client_secret": CLIENT_SECRET,
        "redirect_uri": REDIRECT_URI,
        "grant_type": "authorization_code"
    }).encode("utf-8")
    
    req = urllib.request.Request(token_url, data=token_data, method="POST")
    with urllib.request.urlopen(req) as resp:
        tokens = json.loads(resp.read().decode("utf-8"))
    
    refresh_token = tokens.get("refresh_token")
    access_token = tokens.get("access_token")
    print(f"[OAuth] Refresh token: {bool(refresh_token)}, Access token: {bool(access_token)}")
    
    if refresh_token:
        subprocess.run(["secret-tool", "store", "--label=Gmail Refresh Token", "service", "nevermore", "plugin", "email", "key", "gmail_refresh_token"], input=refresh_token, text=True, check=True)
    if access_token:
        subprocess.run(["secret-tool", "store", "--label=Gmail Access Token", "service", "nevermore", "plugin", "email", "key", "gmail_access_token"], input=access_token, text=True, check=True)
    
    print("[OAuth] SUCCESS! All credentials stored into Secret Service.")
else:
    print("[OAuth] FAILED: Code event timed out.")
    sys.exit(1)
