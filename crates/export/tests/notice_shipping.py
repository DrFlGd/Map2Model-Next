#!/usr/bin/env python3
"""Stage and verify exact Cargo license texts for the Lead's native Electron packaging.

This is a *distribution candidate*, not human approval. No external archives,
native Windows EXE or Electron/Node licenses are silently deemed covered.
Commands:
  python notice_shipping.py stage DIRECTORY
  python notice_shipping.py verify DIRECTORY
The Lead must include resources/legal/* inside the actual packaged app and
verify the exact bytes after unbundling. No changes to Lead-owned desktop code.
"""
import hashlib
import json
import pathlib
import sys

# Intentional choices among already-declared SPDX alternatives. Future Cargo
# crate metadata with an unfamiliar license expression FAILS CLOSED; never
# quietly reinterpret LGPL or a required conjunction as MIT-only.
CHOICES = {
    "MIT": "MIT",
    "MIT OR Apache-2.0": "MIT",
    # adler2: dual/triple option; full bundled MIT grant verified by text check below.
    "0BSD OR MIT OR Apache-2.0": "MIT",
    "Apache-2.0 OR MIT": "MIT",
    "Unlicense OR MIT": "MIT",
    "MIT/Apache-2.0": "MIT",
    "Apache-2.0 OR BSL-1.0": "Apache-2.0",
    "MIT OR Apache-2.0 OR LGPL-2.1-or-later": "MIT",
    "(MIT OR Apache-2.0) AND Unicode-3.0": "MIT AND Unicode-3.0",
    "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT": "MIT",
}


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def load_source():
    manifest = pathlib.Path("rust-license-texts-manifest.json")
    notices = pathlib.Path("THIRD_PARTY_RUST_NOTICES.md")
    data = json.loads(manifest.read_text(encoding="utf-8"))
    if data.get("formatVersion") != "M2M-THIRD-PARTY-NOTICES-1":
        raise ValueError("unrecognized Cargo NOTICE manifest version")
    packages = data["packages"]
    if data.get("missingNoticeTexts") or not packages:
        raise ValueError("missing source license NOTICE texts")
    actual_choices = {}
    for p in packages:
        expression = p["licenseExpression"]
        if expression not in CHOICES or not p["files"]:
            raise ValueError(f"human license decision required for {p['name']}@{p['version']}: {expression}")
        selection = CHOICES[expression]
        actual_choices[f"{p['name']}@{p['version']}"] = {
            "declaredSpdx": expression,
            "chosenLicense": selection,
            "noticeFiles": [f["relativePath"] for f in p["files"]],
        }
    body = notices.read_bytes()
    if b"Rust third-party dependency license/NOTICE texts" not in body:
        raise ValueError("unrecognized source NOTICE bundle")
    for p in packages:
        texts = {}
        for f in p["files"]:
            heading = f"## {p['name']} {p['version']} — {pathlib.Path(f['relativePath']).name}"
            needle = heading.encode("utf-8")
            start = body.find(needle)
            if start < 0:
                raise ValueError(f"missing materialized license text section {heading}")
            end = body.find(b"\n---\n", start)
            texts[f["relativePath"]] = body[start:end if end >= 0 else None].decode("utf-8")
        chosen = actual_choices[f"{p['name']}@{p['version']}"]["chosenLicense"]
        # Catch the subtle case of a crate distributing only AUTHORS/NOTICE
        # without a separate LICENSE-MIT file. r-efi's AUTHORS contains the
        # full MIT grant and disclaimer, which is valid source-text evidence.
        if "MIT" in chosen and not any(
            "Permission is hereby granted" in text and
            "THE SOFTWARE IS PROVIDED" in text for text in texts.values()
        ):
            raise ValueError(f"selected MIT license text not found for {p['name']}")
        if chosen == "Apache-2.0" and not any(
            "Apache License" in text and "Version 2.0" in text
            for text in texts.values()
        ):
            raise ValueError(f"selected Apache license text not found for {p['name']}")
        if "Unicode-3.0" in chosen and not any(
            "UNICODE" in path.upper() and "Unicode" in text
            for path, text in texts.items()
        ):
            raise ValueError(f"mandatory Unicode-3.0 license notice missing for {p['name']}")
    return data, body, actual_choices


def stage(dest):
    data, body, choices = load_source()
    folder = dest / "resources" / "legal"
    folder.mkdir(parents=True, exist_ok=True)
    shipping = folder / "THIRD_PARTY_NOTICES.md"
    shipping.write_bytes(body)
    report = {
        "formatVersion": "M2M-DISTRIBUTOR-NOTICES-1",
        "reviewStatus": "PENDING-LEAD-AND-LEGAL-APPROVAL",
        "scope": "Rust Cargo dependency sources only; Electron/Node notices supplied by Lead",
        "sourceCargoLockSha256": data["cargoLockSha256"],
        "sourceNoticeSha256": digest(body),
        "externalRustPackages": len(data["packages"]),
        "candidateChoices": choices,
    }
    (folder / "THIRD_PARTY_NOTICES.manifest.json").write_bytes(
        (json.dumps(report, sort_keys=True, indent=2) + "\n").encode("utf-8")
    )
    verify(dest)
    print(json.dumps({"status": "staged-exact-bytes-not-release-approved",
        "packageCount": len(data["packages"]), "noticesSha256": digest(body),
        "path": str(shipping)}, sort_keys=True))


def verify(dest):
    data, body, choices = load_source()
    folder = dest / "resources" / "legal"
    delivered = (folder / "THIRD_PARTY_NOTICES.md").read_bytes()
    policy = json.loads((folder / "THIRD_PARTY_NOTICES.manifest.json").read_text())
    if delivered != body or policy.get("sourceNoticeSha256") != digest(delivered):
        raise ValueError("packaged NOTICE file differs from exact resolved Cargo source texts")
    if policy.get("sourceCargoLockSha256") != data["cargoLockSha256"]:
        raise ValueError("packaged NOTICE uses a different Cargo.lock")
    if policy.get("candidateChoices") != choices:
        raise ValueError("packaged resolved crate SPDX choices do not match recorded review")
    if policy.get("reviewStatus") != "PENDING-LEAD-AND-LEGAL-APPROVAL":
        raise ValueError("unexpected redistribution signoff state")
    print(json.dumps({"status": "packaged-Rust-notices-present-unmodified",
        "scope": "staging-dir-only; does not inspect Electron EXE",
        "fileSha256": digest(delivered),
        "verifiedCrates": len(data["packages"])}, sort_keys=True))


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in ("stage", "verify"):
        raise SystemExit("Usage: notice_shipping.py stage|verify DIRECTORY")
    destination = pathlib.Path(sys.argv[2])
    if sys.argv[1] == "stage":
        stage(destination)
    else:
        verify(destination)


if __name__ == "__main__":
    main()
