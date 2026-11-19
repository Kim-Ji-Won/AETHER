"""Test-only TLS peer. Ephemeral private keys never leave its private temp directory."""
import json
import os
from pathlib import Path
import socket
import ssl
import subprocess
import sys
import tempfile

mode = sys.argv[1]
payload = bytes.fromhex(sys.stdin.readline().strip())
expected = json.loads(sys.stdin.readline())

with tempfile.TemporaryDirectory() as root:
    os.chdir(root)

    def cert(*args):
        subprocess.run(["/usr/bin/openssl", *args], check=True, timeout=10,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    Path("ca.conf").write_text("[req]\ndistinguished_name=dn\n[dn]\n"
                               "[ca]\nbasicConstraints=critical,CA:TRUE\nkeyUsage=critical,keyCertSign\n")
    cert("req", "-x509", "-newkey", "rsa:2048", "-nodes", "-sha256", "-days", "1",
         "-subj", "/CN=AETHER ephemeral test CA", "-keyout", "ca.key", "-out", "ca.pem",
         "-config", "ca.conf", "-extensions", "ca")
    cert("req", "-new", "-newkey", "rsa:2048", "-nodes", "-subj", "/CN=fixture",
         "-keyout", "peer.key", "-out", "peer.csr")
    san = "DNS:wrong.invalid" if mode == "hostname" else "IP:127.0.0.1"
    Path("peer.conf").write_text("basicConstraints=critical,CA:FALSE\nextendedKeyUsage=serverAuth\n"
                                 "keyUsage=digitalSignature,keyEncipherment\nsubjectAltName=" + san)
    cert("x509", "-req", "-in", "peer.csr", "-CA", "ca.pem", "-CAkey", "ca.key",
         "-set_serial", "2", "-days", "1", "-sha256", "-extfile", "peer.conf", "-out", "peer.pem")
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    ctx.load_cert_chain("peer.pem", "peer.key")
    with socket.socket(socket.AF_INET) as listener:
        listener.settimeout(8)
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        port = listener.getsockname()[1]
        print(port, Path("ca.pem").absolute(), flush=True)
        sock, _ = listener.accept()
        with sock:
            sock.settimeout(8)
            try:
                peer = ctx.wrap_socket(sock, server_side=True)
            except ssl.SSLError:
                assert mode in ("untrusted", "hostname")
                print("NO_HTTP", flush=True)
            else:
                with peer:
                    stream = peer.makefile("rb")
                    with stream:
                        first = stream.readline(256)
                        if mode in ("untrusted", "hostname"):
                            assert not first
                            print("NO_HTTP", flush=True)
                        else:
                            assert first == b"POST /v1/responses HTTP/1.1\r\n"
                            headers = {}
                            while True:
                                line = stream.readline(1024)
                                if line == b"\r\n":
                                    break
                                key, value = line.split(b":", 1)
                                headers[key.lower()] = value.strip()
                            assert b"authorization" not in headers and b"cookie" not in headers
                            assert json.loads(stream.read(int(headers[b"content-length"]))) == expected
                            if mode == "redirect":
                                peer.sendall(f"HTTP/1.1 307 Temporary Redirect\r\nLocation: https://127.0.0.1:{port}/forbidden\r\nContent-Length: 0\r\n\r\n".encode())
                            else:
                                # Keep the peer open even after the last SSE byte, until client detach.
                                peer.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n" + payload)
                            assert stream.read(1) == b""
                            print("ONE_REQUEST_PEER_CLOSED", flush=True)
        listener.setblocking(False)
        try:
            extra, _ = listener.accept()
        except BlockingIOError:
            pass
        else:
            extra.close()
            raise AssertionError("unexpected second connection")
