#!/usr/bin/env python3
"""Generate actual, unabridged Rust crate license/NOTICE texts for review.

Uses the local, resolved Cargo registry crates, not guessed SPDX permissions.
This emits distribution-*candidate* notices, and fails if a dependency has
no readable notice/license text. Maintainers still choose the applicable
license for dual-licensed packages and approve redistribution packaging.
"""
import hashlib
import json
import pathlib
import subprocess
import sys


def sha(data):
    return hashlib.sha256(data).hexdigest()


def wanted(name):
    lowered = name.lower()
    return (
        lowered.startswith(("license", "licence", "copying", "notice", "unlicense"))
        or lowered in ("copyright", "authors")
    )


def main():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1"],
        stderr=sys.stderr
    ))
    packages = sorted(
        (p for p in metadata["packages"] if p.get("source")),
        key=lambda p: (p["name"], p["version"], p["id"]),
    )
    documents = []
    missing = []
    manifest = []
    for p in packages:
        root = pathlib.Path(p["manifest_path"]).resolve().parent
        candidates = {f for f in root.iterdir() if f.is_file() and wanted(f.name)}
        if p.get("license_file"):
            license_path = root / p["license_file"]
            if not license_path.resolve().is_relative_to(root):
                raise ValueError(f"License file outside crate: {p['name']}")
            candidates.add(license_path)
        package_files = []
        for path in sorted(candidates, key=lambda x: str(x.relative_to(root))):
            if not path.is_file():
                continue
            payload = path.read_bytes()
            if not payload or len(payload) > 256 * 1024 or b"\x00" in payload:
                raise ValueError(f"Invalid license/NOTICE source {p['name']}/{path.name}")
            # License sources are text, not binary attachments. Preserve
            # original wording and exact hashes for distributor auditing.
            text = payload.decode("utf-8-sig")
            package_files.append({
                "relativePath": str(path.relative_to(root)).replace("\\", "/"),
                "sha256": sha(payload),
                "sizeBytes": len(payload),
            })
            documents.append((p["name"], p["version"], p["license"], path.name, text))
        if not package_files:
            missing.append(f"{p['name']}@{p['version']}")
        manifest.append({
            "name": p["name"], "version": p["version"],
            "licenseExpression": p.get("license"),
            "source": p["source"],
            "files": package_files,
        })
    lock = pathlib.Path("Cargo.lock")
    report = {
        "formatVersion": "M2M-THIRD-PARTY-NOTICES-1",
        "status": "automatically-collected-candidate; human-distribution-review-required",
        "externalRustPackages": len(packages),
        "missingNoticeTexts": missing,
        "cargoLockSha256": sha(lock.read_bytes()) if lock.exists() else None,
        "packages": manifest,
    }
    pathlib.Path("rust-license-texts-manifest.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    sections = [
        "# Rust third-party dependency license/NOTICE texts",
        "",
        "**Generated from the exact locally resolved crates.** All license texts are "
        "copied verbatim from the downloaded source packages. A product distributor "
        "must review package selections, license conditions, required attributions "
        "and transitive notices before shipping a Windows binary.",
        "",
        f"Cargo.lock SHA-256: `{report['cargoLockSha256']}`",
        f"External Rust packages: {len(packages)}",
        f"Missing readable license files: {len(missing)}",
        "",
    ]
    for name, version, expression, filename, notice in documents:
        sections.extend([
            "---", "",
            f"## {name} {version} — {filename}",
            "",
            f"SPDX declaration: `{expression or 'UNDECLARED'}`",
            "",
            "```text",
            notice.rstrip("\r\n"),
            "```",
            "",
        ])
    pathlib.Path("THIRD_PARTY_RUST_NOTICES.md").write_text(
        "\n".join(sections), encoding="utf-8"
    )
    print(json.dumps({
        "externalRustPackages": len(packages),
        "noticeFiles": len(documents),
        "missingNoticeTexts": missing,
        "cargoLockSha256": report["cargoLockSha256"],
        "reviewStatus": report["status"],
        "manifestSha256": sha(pathlib.Path("rust-license-texts-manifest.json").read_bytes()),
        "noticesSha256": sha(pathlib.Path("THIRD_PARTY_RUST_NOTICES.md").read_bytes()),
    }, sort_keys=True))
    if missing:
        raise SystemExit(f"ERROR: {len(missing)} dependency crates lack source LICENSE/NOTICE text")


if __name__ == "__main__":
    main()
