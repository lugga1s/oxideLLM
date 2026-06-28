from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import time


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        if self.path in ["/healthz", "/readyz"]:
            body = json.dumps({"status": "ok"}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return

        self.send_error(404)

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        if length:
            self.rfile.read(length)

        if self.path != "/v1/chat/completions":
            self.send_error(404)
            return

        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-cache")
        self.send_header("Connection", "close")
        self.end_headers()

        chunks = [
            'data: {"choices":[{"delta":{"content":"hello"},"index":0}]}\n\n',
            'data: {"choices":[{"delta":{"content":" world"},"index":0,"finish_reason":"stop"}]}\n\n',
            "data: [DONE]\n\n",
        ]

        for chunk in chunks:
            self.wfile.write(chunk.encode())
            self.wfile.flush()
            time.sleep(0.001)

        self.close_connection = True

    def log_message(self, fmt, *args):
        return


class HighCapacityHTTPServer(ThreadingHTTPServer):
    request_queue_size = 1024
    disable_nagle_algorithm = True

if __name__ == "__main__":
    server = HighCapacityHTTPServer(("0.0.0.0", 9000), Handler)
    print("mock SSE server listening on :9000", flush=True)
    server.serve_forever()

