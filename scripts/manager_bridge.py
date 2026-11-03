#!/usr/bin/env python3
"""
Nevermore Manager Bridge
Maintains a persistent Chrome DevTools Protocol connection to the Manager AI
conversation at https://gemini.google.com/app/71ae2007ae72440f.
Uses native CDP Input events to bypass TrustedHTML restrictions.
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
        self.lock = threading.Lock()
        
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

    def send_gemini_message(self, text):
        # 1. Check initial model response count
        js_count = "document.querySelectorAll('message-content, .model-response-text').length"
        initial_count = self.evaluate(js_count) or 0
        
        # 2. Focus the editor element
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

        # 3. Insert text via CDP Input.insertText (bypasses TrustedHTML)
        self.call('Input.insertText', {'text': text})
        time.sleep(0.5)

        # 4. Click Send button or send Enter key
        js_click_send = """
        (() => {
            const sendBtn = document.querySelector('button[aria-label*="Send"], button.send-button, .send-button-container button');
            if (sendBtn && !sendBtn.disabled && sendBtn.offsetParent !== null) {
                sendBtn.click();
                return { clicked: true, method: 'button' };
            }
            return { clicked: false };
        })()
        """
        click_res = self.evaluate(js_click_send)
        if not click_res.get('clicked'):
            # Fallback: send Enter key via CDP Input.dispatchKeyEvent
            self.call('Input.dispatchKeyEvent', {
                'type': 'rawKeyDown',
                'windowsVirtualKeyCode': 13,
                'unmodifiedText': '\r',
                'text': '\r',
                'code': 'Enter',
                'key': 'Enter'
            })
            self.call('Input.dispatchKeyEvent', {
                'type': 'keyUp',
                'windowsVirtualKeyCode': 13,
                'code': 'Enter',
                'key': 'Enter'
            })

        # 5. Wait for new response turn to appear
        start_time = time.time()
        timeout = 180  # 3 minutes max
        new_turn_detected = False
        
        while time.time() - start_time < timeout:
            time.sleep(2)
            # Check for quota limit banner
            js_quota = """
            (() => {
                const text = document.body.innerText;
                const hit = text.includes("reached your usage limit") || text.includes("limit for today") || text.includes("try again later");
                return hit;
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

        # 6. Wait for streaming to complete (stable length for 6 seconds)
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
                if stable_count >= 3:  # 6 seconds unchanged
                    latest_text = curr_text
                    break
            else:
                stable_count = 0
                last_len = len(curr_text)

        # Write to latest reply file
        with open(REPLY_FILE, "w", encoding="utf-8") as f:
            f.write(latest_text)

        return {"status": "OK", "text": latest_text}

def run_daemon():
    if os.path.exists(SOCKET_PATH):
        try:
            os.unlink(SOCKET_PATH)
        except OSError:
            pass

    print("[Bridge Daemon] Connecting to Chrome DevTools...")
    cdp = CDPConnection()
    sid = cdp.attach_to_gemini()
    print(f"[Bridge Daemon] Attached to Gemini conversation, sessionId={sid}")

    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(SOCKET_PATH)
    os.chmod(SOCKET_PATH, 0o700)
    server.listen(5)
    print(f"[Bridge Daemon] Listening on {SOCKET_PATH}")

    while True:
        conn, _ = server.accept()
        try:
            raw_req = conn.recv(1024 * 64)
            if not raw_req:
                conn.close()
                continue
            req = json.loads(raw_req.decode('utf-8'))
            action = req.get('action')
            if action == 'send':
                msg = req.get('message', '')
                res = cdp.send_gemini_message(msg)
                conn.sendall(json.dumps(res).encode('utf-8'))
            elif action == 'eval':
                js = req.get('js', '')
                res = cdp.evaluate(js)
                conn.sendall(json.dumps({'status': 'OK', 'result': res}).encode('utf-8'))
            elif action == 'ping':
                conn.sendall(json.dumps({'status': 'OK'}).encode('utf-8'))
            else:
                conn.sendall(json.dumps({'status': 'ERROR', 'error': 'Unknown action'}).encode('utf-8'))
        except Exception as e:
            conn.sendall(json.dumps({'status': 'ERROR', 'error': str(e)}).encode('utf-8'))
        finally:
            conn.close()

def send_client(message):
    if not os.path.exists(SOCKET_PATH):
        print("Bridge socket not found. Start daemon with: scripts/manager_bridge.py --daemon")
        sys.exit(1)

    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(SOCKET_PATH)
    req = {'action': 'send', 'message': message}
    s.sendall(json.dumps(req).encode('utf-8'))
    
    data = b""
    while True:
        chunk = s.recv(4096)
        if not chunk: break
        data += chunk
    s.close()
    
    res = json.loads(data.decode('utf-8'))
    return res

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Nevermore Manager Bridge")
    parser.add_argument("--daemon", action="store_true", help="Run as background daemon")
    parser.add_argument("--send", type=str, help="Send a message to Manager AI")
    args = parser.parse_args()

    if args.daemon:
        run_daemon()
    elif args.send:
        res = send_client(args.send)
        print(res.get('text', ''))
    else:
        parser.print_help()
