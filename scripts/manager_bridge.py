#!/usr/bin/env python3
"""
Nevermore Manager Bridge Daemon & Client
Maintains a single, persistent Chrome DevTools Protocol connection to the Manager AI
conversation at https://aistudio.google.com/prompts/13YvHFFS5vO79vKlLp_BV7TRIryaj_r2O
(or https://gemini.google.com/).

Prevents repeated Chrome DevTools permission prompts by holding ONE continuous WebSocket
connection open in the daemon, exposing a local Unix domain socket (/tmp/nevermore_bridge.sock)
and watching the file spool (.bridge/outbox.md -> .bridge/inbox.md) for all scripts and subagents.
"""

import sys
import os
import json
import time
import socket
import struct
import argparse
import threading
import subprocess

SOCKET_PATH = "/tmp/nevermore_bridge.sock"
PORT_FILE = "/home/bgg/.config/google-chrome/DevToolsActivePort"
REPLY_FILE = "/tmp/manager_latest_reply.md"
TARGET_URL_SUBSTRING = "aistudio.google.com/prompts"
SPOOL_DIR = "/home/bgg/projects/nevermore-sidebar/.bridge"
OUTBOX = os.path.join(SPOOL_DIR, "outbox.md")
INBOX = os.path.join(SPOOL_DIR, "inbox.md")
STATUS_FILE = os.path.join(SPOOL_DIR, "status.json")


class CDPConnection:
    def __init__(self):
        self.sock = None
        self.buf = bytearray()
        self.msg_id = 0
        self.session_id = None
        self.target_url = ""
        self.is_ai_studio = True
        self.lock = threading.Lock()
        self.send_lock = threading.Lock()
        self.connect()

    def auto_allow(self):
        """Automatically moves cursor to Allow button and clicks to approve handshake."""
        try:
            with open("/tmp/move_allow.js", "w") as f:
                f.write("workspace.cursorPos = Qt.point(1131, 310);\n")
            res = subprocess.run([
                "qdbus", "org.kde.KWin", "/Scripting",
                "org.kde.kwin.Scripting.loadScript", "/tmp/move_allow.js", "allowMover"
            ], capture_output=True, text=True)
            sid = res.stdout.strip()
            if sid:
                subprocess.run(["qdbus", "org.kde.KWin", f"/Scripting/Script{sid}", "run"], capture_output=True)
                subprocess.run(["qdbus", "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.unloadScript", "allowMover"], capture_output=True)
            env = dict(os.environ, YDOTOOL_SOCKET="/run/user/1000/.ydotool_socket")
            subprocess.run(["ydotool", "click", "0xC0"], env=env, capture_output=True)
            time.sleep(0.05)
            # Also send Right Arrow + Enter keyboard shortcut
            subprocess.run(["ydotool", "key", "106:1", "106:0", "28:1", "28:0"], env=env, capture_output=True)
        except Exception as e:
            print(f"[Bridge Daemon] auto_allow warning: {e}", flush=True)

    def connect(self):
        if not os.path.exists(PORT_FILE):
            raise RuntimeError(f"Chrome DevToolsActivePort file not found at {PORT_FILE}")
        with open(PORT_FILE) as f:
            lines = f.read().splitlines()
        self.port = int(lines[0])
        self.path = lines[1]
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect(("127.0.0.1", self.port))

        req = (
            f"GET {self.path} HTTP/1.1\r\n"
            f"Host: 127.0.0.1:{self.port}\r\n"
            f"Upgrade: websocket\r\n"
            f"Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n"
            f"Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(req.encode())
        self.sock.settimeout(1.0)
        resp = b""
        start_t = time.time()
        while b"\r\n\r\n" not in resp:
            try:
                chunk = self.sock.recv(1024)
                if not chunk:
                    break
                resp += chunk
            except socket.timeout:
                print("[Bridge Daemon] Handshake waiting, auto-clicking Allow prompt at (1131, 310)...", flush=True)
                self.auto_allow()
                if time.time() - start_t > 30.0:
                    raise RuntimeError("WebSocket handshake timed out even after auto-clicking Allow")

        self.sock.settimeout(None)
        if b"101 WebSocket" not in resp:
            raise RuntimeError(f"WebSocket handshake failed: {resp.decode('utf-8', errors='replace')}")

        self.buf = bytearray()
        self.msg_id = 0
        self.session_id = None
        print("[Bridge Daemon] WebSocket handshake verified and held permanently open.", flush=True)

    def close(self):
        try:
            if self.sock:
                self.sock.close()
        except Exception:
            pass
        self.sock = None

    def reconnect(self):
        self.close()
        time.sleep(2)
        self.connect()
        self.attach()

    def _send_frame(self, data_str):
        data = data_str.encode('utf-8')
        length = len(data)
        frame = bytearray([0x81])
        if length <= 125:
            frame.append(0x80 | length)
        elif length <= 65535:
            frame.append(0x80 | 126)
            frame.extend(struct.pack('>H', length))
        else:
            frame.append(0x80 | 127)
            frame.extend(struct.pack('>Q', length))
        mask = b'\x12\x34\x56\x78'
        frame.extend(mask)
        frame.extend(bytearray(b ^ mask[i % 4] for i, b in enumerate(data)))
        self.sock.sendall(frame)

    def _recv_frame(self):
        while True:
            while len(self.buf) < 2:
                chunk = self.sock.recv(4096)
                if not chunk:
                    return None
                self.buf.extend(chunk)
            b1, b2 = self.buf[0], self.buf[1]
            is_masked = bool(b2 & 0x80)
            length = b2 & 0x7f
            hdr_len = 2
            if length == 126:
                while len(self.buf) < 4:
                    chunk = self.sock.recv(4096)
                    self.buf.extend(chunk)
                length = struct.unpack('>H', self.buf[2:4])[0]
                hdr_len = 4
            elif length == 127:
                while len(self.buf) < 10:
                    chunk = self.sock.recv(4096)
                    self.buf.extend(chunk)
                length = struct.unpack('>Q', self.buf[2:10])[0]
                hdr_len = 10
            mask_len = 4 if is_masked else 0
            total_frame_len = hdr_len + mask_len + length
            while len(self.buf) < total_frame_len:
                chunk = self.sock.recv(4096)
                if not chunk:
                    return None
                self.buf.extend(chunk)
            payload_start = hdr_len + mask_len
            payload = self.buf[payload_start:total_frame_len]
            del self.buf[:total_frame_len]
            return json.loads(payload.decode('utf-8', errors='replace'))

    def call(self, method, params=None, use_session=True, session_id=None):
        with self.lock:
            self.msg_id += 1
            mid = self.msg_id
            payload = {'id': mid, 'method': method}
            if params is not None:
                payload['params'] = params
            sid = session_id if session_id is not None else (self.session_id if use_session else None)
            if sid:
                payload['sessionId'] = sid
            self._send_frame(json.dumps(payload))

            while True:
                msg = self._recv_frame()
                if not msg:
                    raise RuntimeError("Socket disconnected while waiting for response")
                if msg.get('id') == mid:
                    return msg

    def attach(self):
        """Attaches to AI Studio or Gemini tab without dropping WebSocket on temporary miss."""
        while True:
            try:
                targets_resp = self.call('Target.getTargets', use_session=False)
                targets = targets_resp.get('result', {}).get('targetInfos', [])
                target = None
                for t in targets:
                    url = t.get('url', '')
                    if "aistudio.google.com/prompts" in url or "gemini.google.com" in url:
                        target = t
                        break
                if target:
                    target_id = target['targetId']
                    attach_resp = self.call('Target.attachToTarget', {'targetId': target_id, 'flatten': True}, use_session=False)
                    self.session_id = attach_resp.get('result', {}).get('sessionId')
                    self.target_url = target.get('url', '')
                    self.is_ai_studio = "aistudio.google.com" in self.target_url
                    print(f"[Bridge Daemon] Attached to target '{target.get('title')}' ({self.target_url}), sessionId={self.session_id}", flush=True)
                    return self.session_id
                else:
                    print(f"[Bridge Daemon] No target matching '{TARGET_URL_SUBSTRING}' or Gemini yet; holding socket open and waiting...", flush=True)
                    time.sleep(2.0)
            except Exception as e:
                print(f"[Bridge Daemon] Warning in attach loop: {e}", flush=True)
                time.sleep(2.0)

    def evaluate(self, js_expr):
        resp = self.call('Runtime.evaluate', {
            'expression': js_expr,
            'returnByValue': True,
            'awaitPromise': True
        })
        res = resp.get('result', {})
        if 'exceptionDetails' in res:
            raise RuntimeError(f"JS Exception: {res['exceptionDetails']}")
        return res.get('result', {}).get('value')

    def get_latest_reply(self):
        if self.is_ai_studio:
            js = """
            (() => {
                const turns = Array.from(document.querySelectorAll('ms-chat-turn'));
                if (turns.length === 0) return document.body.innerText;
                const last = turns[turns.length - 1];
                return last.innerText;
            })()
            """
        else:
            js = """
            (() => {
                const responses = document.querySelectorAll('message-content, .model-response-text');
                if (responses.length === 0) return '';
                const last = responses[responses.length - 1];
                return last.innerText;
            })()
            """
        text = self.evaluate(js) or ""
        with open(REPLY_FILE, "w", encoding="utf-8") as f:
            f.write(text)
        return text

    def get_status(self):
        if self.is_ai_studio:
            js = """
            (() => {
                const ta = document.querySelector('textarea[aria-label="Enter a prompt"]') || document.querySelector('textarea.cdk-textarea-autosize') || document.querySelector('textarea');
                const runBtn = document.querySelector('button.ctrl-enter-submits') || document.querySelector('button.ms-button-primary');
                const turns = document.querySelectorAll('ms-chat-turn');
                return {
                    attached: true,
                    target_type: 'AI_STUDIO',
                    has_editor: !!ta,
                    has_send_btn: !!runBtn,
                    quota_limited: false,
                    response_count: turns.length
                };
            })()
            """
        else:
            js = """
            (() => {
                const ql = document.querySelector('rich-textarea div.ql-editor') || document.querySelector('rich-textarea [contenteditable="true"]');
                const sendBtn = document.querySelector('button[aria-label*="Send message"], button[aria-label*="Send"], button.send-button');
                const body = document.body.innerText;
                const quota = body.includes("reached your usage limit") || body.includes("resets at") || body.includes("limit for today");
                const responses = document.querySelectorAll('message-content, .model-response-text');
                return {
                    attached: true,
                    target_type: 'GEMINI',
                    has_editor: !!ql,
                    has_send_btn: !!sendBtn,
                    quota_limited: quota,
                    response_count: responses.length
                };
            })()
            """
        return self.evaluate(js)

    def send_message(self, text):
        with self.send_lock:
            if self.is_ai_studio:
                return self._send_ai_studio_message_impl(text)
            else:
                return self._send_gemini_message_impl(text)

    def _send_ai_studio_message_impl(self, text):
        # 1. Get initial turn count
        js_count = "document.querySelectorAll('ms-chat-turn').length"
        initial_count = self.evaluate(js_count) or 0

        # 2. Focus and set prompt text in AI Studio textarea
        js_set_prompt = f"""
        (() => {{
            const ta = document.querySelector('textarea[aria-label="Enter a prompt"]') ||
                       document.querySelector('textarea.cdk-textarea-autosize') ||
                       document.querySelector('textarea');
            if (!ta) return false;
            ta.focus();
            ta.value = {json.dumps(text)};
            ta.dispatchEvent(new Event('input', {{ bubbles: true }}));
            ta.dispatchEvent(new Event('change', {{ bubbles: true }}));
            return true;
        }})()
        """
        if not self.evaluate(js_set_prompt):
            raise RuntimeError("Could not find prompt textarea in Google AI Studio")

        time.sleep(0.3)

        # 3. Click Run button
        js_click_run = """
        (() => {
            const btn = document.querySelector('button.ctrl-enter-submits') ||
                        document.querySelector('button.ms-button-primary') ||
                        document.querySelector('button[aria-label*="Run" i]');
            if (btn && !btn.disabled) {
                btn.click();
                return true;
            }
            return false;
        })()
        """
        clicked = self.evaluate(js_click_run)
        if not clicked:
            # Fallback: Dispatch Ctrl+Enter via CDP
            self.call('Input.dispatchKeyEvent', {
                'type': 'rawKeyDown',
                'windowsVirtualKeyCode': 13,
                'unmodifiedText': '\r',
                'text': '\r',
                'code': 'Enter',
                'key': 'Enter',
                'modifiers': 2 # Ctrl
            })
            self.call('Input.dispatchKeyEvent', {
                'type': 'keyUp',
                'windowsVirtualKeyCode': 13,
                'code': 'Enter',
                'key': 'Enter',
                'modifiers': 2
            })

        print("[Bridge Daemon] Run triggered in AI Studio. Waiting for new response turn...", flush=True)

        # 4. Wait for new turn
        start_time = time.time()
        timeout = 240
        new_turn_detected = False

        while time.time() - start_time < timeout:
            time.sleep(2)
            curr_count = self.evaluate(js_count) or 0
            if curr_count > initial_count:
                new_turn_detected = True
                break

        if not new_turn_detected:
            raise RuntimeError("Timed out waiting for AI Studio response turn to appear")

        print("[Bridge Daemon] New turn detected. Waiting for streaming completion...", flush=True)

        # 5. Wait for streaming completion
        print("[Bridge Daemon] Model is thinking/generating. Waiting for execution to complete...", flush=True)
        js_is_running = """
        (() => {
            const btn = document.querySelector('button.ctrl-enter-submits') ||
                        document.querySelector('button.ms-button-primary');
            return btn ? btn.innerText.includes('Stop') : false;
        })()
        """
        js_get_latest = """
        (() => {
            const turns = Array.from(document.querySelectorAll('ms-chat-turn'));
            if (turns.length === 0) return '';
            const last = turns[turns.length - 1];
            return last.innerText;
        })()
        """

        # First wait while button says Stop
        while time.time() - start_time < timeout:
            time.sleep(2)
            running = self.evaluate(js_is_running)
            if not running:
                print("[Bridge Daemon] Stop button reverted to Run. Settling...", flush=True)
                time.sleep(2)
                break

        # Then wait for text stabilization
        last_len = -1
        stable_count = 0
        latest_text = ""

        while time.time() - start_time < timeout:
            curr_text = self.evaluate(js_get_latest) or ""
            if len(curr_text) == last_len and len(curr_text) > 0:
                stable_count += 1
                if stable_count >= 2:
                    latest_text = curr_text
                    break
            else:
                stable_count = 0
                last_len = len(curr_text)
            time.sleep(2)

        if not latest_text:
            latest_text = self.evaluate(js_get_latest) or ""

        with open(REPLY_FILE, "w", encoding="utf-8") as f:
            f.write(latest_text)

        return {"status": "OK", "text": latest_text}

    def _send_gemini_message_impl(self, text):
        js_count = "document.querySelectorAll('message-content, .model-response-text').length"
        initial_count = self.evaluate(js_count) or 0

        js_focus_and_clear = """
        (() => {
            const ql = document.querySelector('rich-textarea div.ql-editor') || document.querySelector('rich-textarea [contenteditable="true"]');
            if (!ql) return false;
            ql.focus();
            try {
                document.execCommand('selectAll', false, null);
                document.execCommand('delete', false, null);
            } catch(e) {}
            ql.innerHTML = '<p><br></p>';
            ql.dispatchEvent(new Event('input', { bubbles: true }));
            return true;
        })()
        """
        if not self.evaluate(js_focus_and_clear):
            raise RuntimeError("Could not find or focus chat editor element")

        time.sleep(0.3)
        self.call('Input.insertText', {'text': text})
        time.sleep(0.5)

        js_get_send_rect = """
        (() => {
            const sendBtn = document.querySelector('button[aria-label*="Send message"], button[aria-label*="Send"], button.send-button');
            if (sendBtn && !sendBtn.disabled && sendBtn.offsetParent !== null) {
                const r = sendBtn.getBoundingClientRect();
                return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
            }
            return null;
        })()
        """
        rect = self.evaluate(js_get_send_rect)
        if rect:
            self.call('Input.dispatchMouseEvent', {'type': 'mouseMoved', 'x': rect['x'], 'y': rect['y']})
            time.sleep(0.1)
            self.call('Input.dispatchMouseEvent', {'type': 'mousePressed', 'x': rect['x'], 'y': rect['y'], 'button': 'left', 'clickCount': 1})
            time.sleep(0.05)
            self.call('Input.dispatchMouseEvent', {'type': 'mouseReleased', 'x': rect['x'], 'y': rect['y'], 'button': 'left', 'clickCount': 1})
        else:
            self.call('Input.dispatchKeyEvent', {'type': 'rawKeyDown', 'windowsVirtualKeyCode': 13, 'unmodifiedText': '\r', 'text': '\r', 'code': 'Enter', 'key': 'Enter'})
            self.call('Input.dispatchKeyEvent', {'type': 'keyUp', 'windowsVirtualKeyCode': 13, 'code': 'Enter', 'key': 'Enter'})

        start_time = time.time()
        timeout = 240
        new_turn_detected = False

        while time.time() - start_time < timeout:
            time.sleep(2)
            js_quota = """
            (() => {
                const text = document.body.innerText;
                return text.includes("reached your usage limit") || text.includes("limit for today") || text.includes("try again later");
            })()
            """
            if self.evaluate(js_quota):
                return {"status": "QUOTA_LIMITED", "text": "Hit Gemini rate/quota limit"}

            curr_count = self.evaluate(js_count) or 0
            if curr_count > initial_count:
                new_turn_detected = True
                break

        if not new_turn_detected:
            raise RuntimeError("Timed out waiting for Manager AI response turn to appear")

        last_len = -1
        stable_count = 0
        latest_text = ""
        js_get_latest = """
        (() => {
            const responses = document.querySelectorAll('message-content, .model-response-text');
            if (responses.length === 0) return '';
            const last = responses[responses.length - 1];
            return last.innerText;
        })()
        """

        while time.time() - start_time < timeout:
            time.sleep(2)
            curr_text = self.evaluate(js_get_latest) or ""
            if len(curr_text) == last_len and len(curr_text) > 0:
                stable_count += 1
                if stable_count >= 3:
                    latest_text = curr_text
                    break
            else:
                stable_count = 0
                last_len = len(curr_text)

        with open(REPLY_FILE, "w", encoding="utf-8") as f:
            f.write(latest_text)

        return {"status": "OK", "text": latest_text}


def file_spool_loop(cdp):
    os.makedirs(SPOOL_DIR, exist_ok=True)
    print(f"[Bridge Spool] Watching spool directory: {SPOOL_DIR}", flush=True)
    while True:
        try:
            st = cdp.get_status()
            with open(STATUS_FILE + ".tmp", "w", encoding="utf-8") as f:
                json.dump(st, f, indent=2)
            os.replace(STATUS_FILE + ".tmp", STATUS_FILE)

            if os.path.exists(OUTBOX) and os.path.getsize(OUTBOX) > 0:
                processing_file = OUTBOX + ".processing"
                try:
                    os.replace(OUTBOX, processing_file)
                except OSError:
                    continue

                with open(processing_file, "r", encoding="utf-8") as f:
                    msg = f.read().strip()
                if msg:
                    print(f"[Bridge Spool] Found outbox.md ({len(msg)} bytes). Transmitting to Manager AI...", flush=True)
                    res = cdp.send_message(msg)
                    if res.get("status") == "OK":
                        reply_text = res.get("text", "")
                        with open(INBOX + ".tmp", "w", encoding="utf-8") as f:
                            f.write(reply_text)
                        os.replace(INBOX + ".tmp", INBOX)
                        try:
                            os.unlink(processing_file)
                        except OSError:
                            pass
                        print(f"[Bridge Spool] Delivered response to inbox.md ({len(reply_text)} chars). Processing file removed.", flush=True)
                    else:
                        err = res.get("error", "Unknown error")
                        print(f"[Bridge Spool] Error sending message: {err}", flush=True)
                        with open(INBOX + ".tmp", "w", encoding="utf-8") as f:
                            f.write(f"[BRIDGE ERROR] {err}\n")
                        os.replace(INBOX + ".tmp", INBOX)
                        try:
                            os.unlink(processing_file)
                        except OSError:
                            pass
        except Exception as e:
            print(f"[Bridge Spool] Exception in spool loop: {e}", flush=True)
        time.sleep(1.0)


def run_daemon():
    if os.path.exists(SOCKET_PATH):
        try:
            os.unlink(SOCKET_PATH)
        except OSError:
            pass

    print("[Bridge Daemon] Connecting to Chrome DevTools (one-time persistent connection)...", flush=True)
    cdp = CDPConnection()
    sid = cdp.attach()
    print(f"[Bridge Daemon] Attached to Manager conversation, sessionId={sid}", flush=True)

    # Start file spool watcher thread
    spool_thread = threading.Thread(target=file_spool_loop, args=(cdp,), daemon=True)
    spool_thread.start()

    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(SOCKET_PATH)
    os.chmod(SOCKET_PATH, 0o700)
    server.listen(10)
    print(f"[Bridge Daemon] Listening on {SOCKET_PATH}", flush=True)

    while True:
        conn, _ = server.accept()
        try:
            chunks = []
            while True:
                chunk = conn.recv(65536)
                if not chunk:
                    break
                chunks.append(chunk)
                try:
                    req = json.loads(b"".join(chunks).decode('utf-8'))
                    break
                except json.JSONDecodeError:
                    continue
            if not chunks:
                conn.close()
                continue

            action = req.get('action')
            try:
                if action == 'send':
                    msg = req.get('message', '')
                    res = cdp.send_message(msg)
                    conn.sendall(json.dumps(res).encode('utf-8'))
                elif action == 'get_latest':
                    text = cdp.get_latest_reply()
                    conn.sendall(json.dumps({'status': 'OK', 'text': text}).encode('utf-8'))
                elif action == 'status':
                    st = cdp.get_status()
                    conn.sendall(json.dumps({'status': 'OK', 'data': st}).encode('utf-8'))
                elif action == 'eval':
                    js = req.get('js', '')
                    res = cdp.evaluate(js)
                    conn.sendall(json.dumps({'status': 'OK', 'result': res}).encode('utf-8'))
                elif action == 'ping':
                    conn.sendall(json.dumps({'status': 'OK'}).encode('utf-8'))
                elif action == 'call_cdp':
                    method = req.get('method')
                    params = req.get('params')
                    use_session = req.get('use_session', True)
                    session_id = req.get('session_id')
                    res = cdp.call(method, params=params, use_session=use_session, session_id=session_id)
                    conn.sendall(json.dumps({'status': 'OK', 'result': res}).encode('utf-8'))
                else:
                    conn.sendall(json.dumps({'status': 'ERROR', 'error': 'Unknown action'}).encode('utf-8'))
            except (socket.error, RuntimeError) as e:
                print(f"[Bridge Daemon] Connection error: {e}. Attempting reconnect...", flush=True)
                try:
                    cdp.reconnect()
                    conn.sendall(json.dumps({'status': 'ERROR', 'error': f"Reconnected after error: {e}"}).encode('utf-8'))
                except Exception as re_err:
                    conn.sendall(json.dumps({'status': 'ERROR', 'error': f"Failed to reconnect: {re_err}"}).encode('utf-8'))

        except Exception as e:
            conn.sendall(json.dumps({'status': 'ERROR', 'error': str(e)}).encode('utf-8'))
        finally:
            conn.close()


def send_ipc(action, **kwargs):
    if not os.path.exists(SOCKET_PATH):
        raise RuntimeError(f"Bridge socket {SOCKET_PATH} not found. Is nevermore-bridge service running?")

    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(SOCKET_PATH)
    req = {'action': action, **kwargs}
    s.sendall(json.dumps(req).encode('utf-8'))
    s.shutdown(socket.SHUT_WR)

    data = b""
    while True:
        chunk = s.recv(4096)
        if not chunk:
            break
        data += chunk
    s.close()
    return json.loads(data.decode('utf-8'))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Nevermore Manager Bridge Daemon & Client")
    parser.add_argument("--daemon", action="store_true", help="Run as background daemon")
    parser.add_argument("--send", type=str, help="Send message to Manager AI")
    parser.add_argument("--send-file", type=str, help="Send file content to Manager AI")
    parser.add_argument("--get-latest", action="store_true", help="Get latest reply from Manager AI")
    parser.add_argument("--status", action="store_true", help="Check bridge and tab status")
    parser.add_argument("--eval", type=str, help="Evaluate JS expression in Gemini/AI Studio tab")
    parser.add_argument("--ping", action="store_true", help="Check daemon health")
    args = parser.parse_args()

    if args.daemon:
        run_daemon()
    elif args.send:
        res = send_ipc('send', message=args.send)
        if res.get('status') == 'OK':
            print(res.get('text', ''))
        else:
            print(f"[BRIDGE ERROR] {res.get('error', res)}", file=sys.stderr)
            sys.exit(1)
    elif args.send_file:
        with open(args.send_file, "r", encoding="utf-8") as f:
            content = f.read()
        res = send_ipc('send', message=content)
        if res.get('status') == 'OK':
            print(res.get('text', ''))
        else:
            print(f"[BRIDGE ERROR] {res.get('error', res)}", file=sys.stderr)
            sys.exit(1)
    elif args.get_latest:
        res = send_ipc('get_latest')
        print(res.get('text', ''))
    elif args.status:
        res = send_ipc('status')
        print(json.dumps(res, indent=2))
    elif args.eval:
        res = send_ipc('eval', js=args.eval)
        print(json.dumps(res, indent=2))
    elif args.ping:
        res = send_ipc('ping')
        print(json.dumps(res, indent=2))
    else:
        parser.print_help()
