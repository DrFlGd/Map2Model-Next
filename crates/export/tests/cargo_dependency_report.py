#!/usr/bin/env python3
"""Produce exact resolved Cargo transitive SPDX-license inventory for lead review.

This is not an OS package distributor or license-text bundler. Do not infer
redistribution approval from successful metadata collection.
"""
import hashlib
import json
import pathlib
import subprocess
import sys


def main():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1"], stderr=sys.stderr
    ))
    external = sorted(
        (
            {
                "name": package["name"],
                "version": package["version"],
                "licenseExpression": package.get("license"),
                "licenseFile": package.get("license_file"),
                "repository": package.get("repository"),
                "source": package.get("source"),
            }
            for package in metadata["packages"] if package.get("source")
        ),
        key=lambda p: (p["name"], p["version"]),
    )
    missing = [f'{p["name"]}@{p["version"]}' for p in external
               if not p["licenseExpression"] and not p["licenseFile"]]
    lock = pathlib.Path("Cargo.lock")
    report = {
        "reportScope": "exact resolved Rust dependency license declarations, NOT license-text redistribution signoff",
        "reviewStatus": "manual-NOTICE-and-target-specific-license-review-required",
        "externalPackageCount": len(external),
        "missingLicenseMetadata": missing,
        "cargoLockSha256": hashlib.sha256(lock.read_bytes()).hexdigest() if lock.exists() else None,
        "externalPackages": external,
    }
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    if missing:
        raise SystemExit(f"{len(missing)} resolved packages lack licensing metadata")


if __name__ == "__main__":
    main()
