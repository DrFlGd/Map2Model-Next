#Requires -Version 5.1
<#
.SYNOPSIS
  Offline, format-only native Rust verification of the user's private
  Interlaken regional PMTiles extract. Do not commit/archive the source file.
.DESCRIPTION
  Operate from the repository root after Rust dependencies have been cached.
  E.g. .\crates\data\scripts\inspect-private-interlaken.ps1 -ArchivePath "C:\Downloads\interlaken-z17.pmtiles"
  Disconnect network before running for an independently observed offline run.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath
)

$ErrorActionPreference = "Stop"
$expectedSha = "75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0"
$inputFile = (Resolve-Path -LiteralPath $ArchivePath -ErrorAction Stop).ProviderPath
$actualHash = (Get-FileHash -LiteralPath $inputFile -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualHash -ne $expectedSha) {
    throw "HASH_MISMATCH: expected $expectedSha; got $actualHash. The earlier maxZoom=18 file is NOT this fixture."
}
$size = (Get-Item -LiteralPath $inputFile).Length
if ($size -ne 3127295) {
    throw "INVALID_SOURCE: unexpected private archive byte length $size"
}
$env:CARGO_NET_OFFLINE = "true"
$env:MAP2MODEL_PRIVATE_INTERLAKEN_PM_TILES = $inputFile
try {
    Write-Output "Private sample SHA256: $actualHash"
    Write-Output "Private sample bytes: $size"
    rustc --version
    if ($LASTEXITCODE -ne 0) { throw "Rust compiler is unavailable" }
    cargo --version
    if ($LASTEXITCODE -ne 0) { throw "Cargo is unavailable" }

    cargo test --offline -p map2model-data --test private_mapterhorn -- --nocapture
    if ($LASTEXITCODE -ne 0) { throw "PRIVATE_RUST_SAMPLE_TEST_FAILED" }

    cargo run --offline -p map2model-data --bin terrain_offline -- inspect $inputFile 13 4275 2891
    if ($LASTEXITCODE -ne 0) { throw "PRIVATE_RUST_INSPECTION_FAILED" }

    Write-Output "Format-only Rust smoke finished. Datum/license/native resolution are NOT verified."
    Write-Output "No terrain bytes should be attached to public GitHub actions, issues or PRs."
}
finally {
    Remove-Item Env:MAP2MODEL_PRIVATE_INTERLAKEN_PM_TILES -ErrorAction SilentlyContinue
    Remove-Item Env:CARGO_NET_OFFLINE -ErrorAction SilentlyContinue
}
