#!/usr/bin/env python3
"""Verify the MuJoCo JSON gateway without PowerShell or ROS tooling."""
import argparse, json, socket, time

def read_frame(sock):
    buf = getattr(read_frame, "buf", b"")
    while b"\n" not in buf:
        chunk = sock.recv(65536)
        if not chunk: raise RuntimeError("gateway closed connection")
        buf += chunk
    line, buf = buf.split(b"\n", 1)
    read_frame.buf = buf
    return json.loads(line)

def wait_for(sock, predicate, timeout=5):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        item = read_frame(sock)
        if predicate(item): return item
    raise TimeoutError("timed out waiting for gateway response")

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=50051)
    args = ap.parse_args()
    with socket.create_connection((args.host, args.port), timeout=5) as sock:
        sock.settimeout(5)
        first = wait_for(sock, lambda x: "sequence" in x)
        assert first["source"] == "mujoco", first
        assert len(first["tf"]) == 10, len(first["tf"])
        before = first["joint_position_rad"][0]
        sock.sendall(b'{"type":"enable","enabled":true}\n')
        ack = wait_for(sock, lambda x: x.get("type") == "ack")
        assert ack["status"] == "accepted", ack
        sock.sendall(b'{"type":"jog","joint_index":0,"step_rad":0.05}\n')
        ack = wait_for(sock, lambda x: x.get("type") == "ack")
        assert ack["status"] == "accepted", ack
        after = wait_for(sock, lambda x: "sequence" in x and abs(x["joint_position_rad"][0] - before) > 1e-7)
        print(f"telemetry=OK source={after['source']} tf={len(after['tf'])} sequence={after['sequence']}")
        print("control=OK enable accepted, jog accepted")

if __name__ == "__main__": main()
