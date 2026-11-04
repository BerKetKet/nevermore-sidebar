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
SCOPE = "https://www.googleapis.com/auth/gmail.modify"
CONTEXT_ID = "B4B094846940FDDEACF31B13AA1053DC"

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
    print(f"[HTTP] Request: {first_line}")
    if "GET /?code=" in first_line:
        raw_code = first_line.split("GET /?code=")[1].split(" ")[0].split("&")[0]
        auth_code = urllib.parse.unquote(raw_code)
        resp = (
            "HTTP/1.1 200 OK\r\n"
            "Content-Type: text/html\r\n\r\n"
            "<html><body><h1>Authorization Successful!</h1><p>You can close this tab and return to Nevermore.</p></body></html>"
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

def cdp_call(req):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect("/tmp/nevermore_bridge.sock")
    s.sendall(json.dumps(req).encode("utf-8"))
    res = b""
    while True:
        c = s.recv(4096)
        if not c: break
        res += c
    return json.loads(res.decode("utf-8"))

auth_url = (
    f"https://accounts.google.com/o/oauth2/v2/auth?"
    f"client_id={CLIENT_ID}&redirect_uri={urllib.parse.quote(REDIRECT_URI, safe='')}&"
    f"response_type=code&scope={urllib.parse.quote(SCOPE, safe='')}&"
    f"access_type=offline&prompt=consent"
)

print("[OAuth] Creating browser target...")
create_res = cdp_call({
    "action": "call_cdp",
    "method": "Target.createTarget",
    "params": {"url": auth_url, "browserContextId": CONTEXT_ID},
    "use_session": False
})
target_id = create_res["result"]["result"]["targetId"]
print(f"[OAuth] Target created: {target_id}")

attach_res = cdp_call({
    "action": "call_cdp",
    "method": "Target.attachToTarget",
    "params": {"targetId": target_id, "flatten": True},
    "use_session": False
})
session_id = attach_res["result"]["result"]["sessionId"]
print(f"[OAuth] Attached sessionId: {session_id}")

# Automation loop to handle consent clicks
start_t = time.time()
while time.time() - start_t < 60 and not code_event.is_set():
    time.sleep(2)
    # Check page state
    eval_res = cdp_call({
        "action": "call_cdp",
        "method": "Runtime.evaluate",
        "params": {
            "expression": """(() => {
                const url = window.location.href;
                const body = document.body ? document.body.innerText : '';
                
                // 1. Account chooser
                const acct = Array.from(document.querySelectorAll("[data-identifier], div, li")).find(e => e.innerText && e.innerText.includes("berketket@gmail.com"));
                if (acct && url.includes("accountchooser")) {
                    acct.click();
                    return "clicked account";
                }

                // 2. Unverified app warning -> Advanced -> Go to Nevermore (unsafe)
                const advBtn = Array.from(document.querySelectorAll("button, a")).find(b => b.innerText && b.innerText.trim().toLowerCase().includes("advanced"));
                if (advBtn) {
                    advBtn.click();
                    return "clicked Advanced";
                }
                const unsafeLink = Array.from(document.querySelectorAll("a, button")).find(b => b.innerText && b.innerText.includes("Go to Nevermore"));
                if (unsafeLink) {
                    unsafeLink.click();
                    return "clicked unsafe link";
                }

                // 3. Permission checkboxes
                const chks = Array.from(document.querySelectorAll("input[type=checkbox]")).filter(c => !c.checked);
                if (chks.length > 0) {
                    chks.forEach(c => { c.click(); c.dispatchEvent(new Event('change', {bubbles: true})); });
                    return "checked permissions";
                }

                // 4. Continue / Allow buttons
                const continueBtn = Array.from(document.querySelectorAll("button")).find(b => {
                    const t = b.innerText.trim().toLowerCase();
                    return (t === "continue" || t === "allow" || t === "select all") && !b.disabled;
                });
                if (continueBtn) {
                    continueBtn.click();
                    return "clicked " + continueBtn.innerText.trim();
                }

                return "waiting: url=" + url.slice(0, 80) + " title=" + document.title;
            })()""",
            "returnByValue": True
        },
        "use_session": True,
        "session_id": session_id
    })
    val = eval_res.get("result", {}).get("result", {}).get("value", "")
    print(f"[OAuth Step] {val}")

if code_event.wait(timeout=10) and auth_code:
    print(f"[OAuth] Successfully acquired auth code: {auth_code[:15]}...")
    
    # Close tab
    cdp_call({
        "action": "call_cdp",
        "method": "Target.closeTarget",
        "params": {"targetId": target_id},
        "use_session": False
    })
    
    # Token exchange
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
    print(f"[OAuth] Refresh token acquired: {bool(refresh_token)}, Access token: {bool(access_token)}")
    
    if refresh_token:
        p1 = subprocess.run(["secret-tool", "store", "--label=Gmail Refresh Token", "service", "nevermore", "plugin", "email", "key", "gmail_refresh_token"], input=refresh_token, text=True)
    if access_token:
        p2 = subprocess.run(["secret-tool", "store", "--label=Gmail Access Token", "service", "nevermore", "plugin", "email", "key", "gmail_access_token"], input=access_token, text=True)
    
    print("[OAuth] COMPLETE! Stored in Secret Service.")
else:
    print("[OAuth] FAILED: Code event timed out.")
    sys.exit(1)
