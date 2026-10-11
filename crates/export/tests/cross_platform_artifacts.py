#!/usr/bin/env python3
"""Require identical Rust 3MF and shipping NOTICE bytes across Windows and Ubuntu CI artifacts."""
import hashlib
import json
import pathlib
import sys

REQUIRED = (
    "synthetic-terrain.3mf",
    "Cargo.lock",
    "rust-license-texts-manifest.json",
    "THIRD_PARTY_RUST_NOTICES.md",
    ".m2m-notice-staging/resources/legal/THIRD_PARTY_NOTICES.md",
    ".m2m-notice-staging/resources/legal/THIRD_PARTY_NOTICES.manifest.json",
)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    if len(sys.argv) != 3:
        raise SystemExit("Usage: cross_platform_artifacts.py UBUNTU_ARTIFACT_DIR WINDOWS_ARTIFACT_DIR")
    root_linux, root_windows = map(pathlib.Path, sys.argv[1:])
    hashes = {}
    for name in REQUIRED:
        a = (root_linux / name).read_bytes()
        b = (root_windows / name).read_bytes()
        if not a or a != b:
            raise ValueError(f"Cross-OS artifact bytes differ: {name} "
                             f"linux={digest(a)} windows={digest(b)}")
        hashes[name] = digest(a)
    if hashes["THIRD_PARTY_RUST_NOTICES.md"] != hashes[
        ".m2m-notice-staging/resources/legal/THIRD_PARTY_NOTICES.md"
    ]:
        raise ValueError("shipped NOTICE differs from resolved crate license texts")
    if hashes["synthetic-terrain.3mf"] != (
        "8463789ff97816d3dccd8b8085444c623150fb68417f3da04eafe9364afce675"
    ):
        raise ValueError("synthetic Core golden 3MF changed without independent review")
    notices = json.loads((root_linux / "rust-license-texts-manifest.json").read_text("utf8"))
    if hashes["Cargo.lock"] != notices["cargoLockSha256"]:
        raise ValueError("Cargo.lock differs from packaged NOTICE source lock digest")
    manifest_path = root_linux / (
        ".m2m-notice-staging/resources/legal/THIRD_PARTY_NOTICES.manifest.json"
    )
    shipping = json.loads(manifest_path.read_text("utf8"))
    if shipping["sourceNoticeSha256"] != hashes["THIRD_PARTY_RUST_NOTICES.md"]:
        raise ValueError("shipping manifest refers to different source notice texts")
    if shipping["sourceCargoLockSha256"] != hashes["Cargo.lock"]:
        raise ValueError("shipping manifest refers to different Cargo.lock")
    print(json.dumps({
        "status": "windows-ubuntu-byte-identical-and-shipping-manifest-consistent",
        "verifiedFiles": hashes,
        "sourceCrates": len(notices["packages"]),
        "scope": "CI artifact comparison; not actual installed Windows Electron EXE"
    }, sort_keys=True))


if __name__ == "__main__":
    main()
