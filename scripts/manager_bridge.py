#!/usr/bin/env python3
"""
Nevermore Manager Bridge Daemon & Client
Maintains a single, persistent Chrome DevTools Protocol connection to the Manager AI
conversation at https://gemini.google.com/app/71ae2007ae72440f.

Prevents repeated Chrome DevTools permission prompts by holding ONE continuous WebSocket
connection open in the daemon, exposing a local Unix domain socket (/tmp/nevermore_bridge.sock)
for all scripts and subagents.
"""

import sys
import os
import json
import time
import socket
import struct
import argparse
import threading

SOCKET_PATH = "/tmp/nevermore_bridge.sock"
PORT_FILE = "/home/bgg/.config/google-chrome/DevToolsActivePort"
REPLY_FILE = "/tmp/manager_latest_reply.md"
TARGET_URL_SUBSTRING = "gemini.google.com/app/71ae2007ae72440f"


class CDPConnection:
    def __init__(self):
        self.sock = None
        self.buf = bytearray()
        self.msg_id = 0
        self.session_id = None
        self.lock = threading.Lock()
        self.connect()

    def connect(self):
        if not os.path.exists(PORT_FILE):
            raise RuntimeError(f"Chrome DevToolsActivePort file not found at {PORT_FILE}")
        with open(PORT_FILE) as f:
            lines = f.read().splitlines()
        self.port = int(lines[0])
        self.path = lines[1]
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.connect(("127.0.0.1", self.port))

        # HTTP Upgrade to WebSocket
        req = (
            f"GET {self.path} HTTP/1.1\r\n"
            f"Host: 127.0.0.1:{self.port}\r\n"
            f"Upgrade: websocket\r\n"
            f"Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n"
            f"Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(req.encode())
        resp = b""
        while b"\r\n\r\n" not in resp:
            chunk = self.sock.recv(1024)
            if not chunk:
                raise RuntimeError("Connection closed during WebSocket handshake")
            resp += chunk
        if b"101 WebSocket" not in resp:
            raise RuntimeError(f"WebSocket handshake failed: {resp.decode('utf-8', errors='replace')}")

        self.buf = bytearray()
        self.msg_id = 0
        self.session_id = None

    def close(self):
        try:
            if self.sock:
                self.sock.close()
        except Exception:
            pass
        self.sock = None

    def reconnect(self):
        self.close()
        time.sleep(1)
        self.connect()
        self.attach_to_gemini()

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

    def call(self, method, params=None, use_session=True):
        with self.lock:
            self.msg_id += 1
            mid = self.msg_id
            payload = {'id': mid, 'method': method}
            if params is not None:
                payload['params'] = params
            if use_session and self.session_id:
                payload['sessionId'] = self.session_id
            self._send_frame(json.dumps(payload))

            while True:
                msg = self._recv_frame()
                if not msg:
                    raise RuntimeError("Socket disconnected while waiting for response")
                if msg.get('id') == mid:
                    return msg

    def attach_to_gemini(self):
        targets_resp = self.call('Target.getTargets', use_session=False)
        targets = targets_resp.get('result', {}).get('targetInfos', [])
        gemini_target = None
        for t in targets:
            if TARGET_URL_SUBSTRING in t.get('url', ''):
                gemini_target = t
                break
        if not gemini_target:
            raise RuntimeError(f"No Chrome tab found with URL matching '{TARGET_URL_SUBSTRING}'")

        target_id = gemini_target['targetId']
        attach_resp = self.call('Target.attachToTarget', {'targetId': target_id, 'flatten': True}, use_session=False)
        self.session_id = attach_resp.get('result', {}).get('sessionId')
        return self.session_id

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
        js = """
        (() => {
            const ql = document.querySelector('rich-textarea div.ql-editor') || document.querySelector('rich-textarea [contenteditable="true"]');
            const sendBtn = document.querySelector('button[aria-label*="Send message"], button[aria-label*="Send"], button.send-button');
            const body = document.body.innerText;
            const quota = body.includes("reached your usage limit") || body.includes("resets at") || body.includes("limit for today");
            const responses = document.querySelectorAll('message-content, .model-response-text');
            return {
                attached: true,
                has_editor: !!ql,
                has_send_btn: !!sendBtn,
                quota_limited: quota,
                response_count: responses.length
            };
        })()
        """
        return self.evaluate(js)

    def send_gemini_message(self, text):
        js_count = "document.querySelectorAll('message-content, .model-response-text').length"
        initial_count = self.evaluate(js_count) or 0

        # Focus editor
        js_focus = """
        (() => {
            const ql = document.querySelector('rich-textarea div.ql-editor') || document.querySelector('rich-textarea [contenteditable="true"]');
            if (!ql) return false;
            ql.focus();
            return true;
        })()
        """
        if not self.evaluate(js_focus):
            raise RuntimeError("Could not find or focus chat editor element")

        time.sleep(0.3)
        self.call('Input.insertText', {'text': text})
        time.sleep(0.5)

        # Click send
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

        # Wait for new turn
        start_time = time.time()
        timeout = 180
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

        # Wait for streaming completion
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


def run_daemon():
    if os.path.exists(SOCKET_PATH):
        try:
            os.unlink(SOCKET_PATH)
        except OSError:
            pass

    print("[Bridge Daemon] Connecting to Chrome DevTools (one-time persistent connection)...", flush=True)
    cdp = CDPConnection()
    sid = cdp.attach_to_gemini()
    print(f"[Bridge Daemon] Attached to Gemini conversation, sessionId={sid}", flush=True)

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
                    res = cdp.send_gemini_message(msg)
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
                else:
                    conn.sendall(json.dumps({'status': 'ERROR', 'error': 'Unknown action'}).encode('utf-8'))
            except (socket.error, RuntimeError) as e:
                # Attempt one reconnect on socket drop
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
        raise RuntimeError(f"Bridge socket {SOCKET_PATH} not found. Is nevermore-manager-bridge service running?")

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
    parser.add_argument("--eval", type=str, help="Evaluate JS expression in Gemini tab")
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
