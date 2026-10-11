#requires -version 5.1
<#
Purpose: Inspect the privately held, corrected Interlaken PMTiles in a portable
Windows build, with NO Rust/Cargo installation and NO network calls.
This is source-FORMAT evidence only; NOT a generation-ready declaration.
#>
param(
  [Parameter(Mandatory = $true)]
  [string] $ArchivePath
)
$ErrorActionPreference = "Stop"
$expected = "75e03d8e0ec36d347ee4ed412ed816d67af53309236a52ecb028930c99f06aa0"
$exe = Join-Path $PSScriptRoot "terrain_offline.exe"
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
  throw "INVALID_SOURCE: terrain_offline.exe must be beside this script."
}
$archive = (Resolve-Path -LiteralPath $ArchivePath -ErrorAction Stop).ProviderPath
$sha = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($sha -ne $expected) {
  throw "HASH_MISMATCH: This is not the approved format-test bytes (expected $expected; observed $sha). The old max-zoom-18 file is not accepted."
}
$length = (Get-Item -LiteralPath $archive).Length
if ($length -ne 3127295) {
  throw "INVALID_SOURCE: expected 3127295 bytes, observed $length."
}
function Invoke-Inspection([string[]] $tile) {
  $outputLines = & $exe inspect $archive @tile 2>&1
  if ($LASTEXITCODE -ne 0) {
    throw "INVALID_SOURCE: Native Rust inspection returned exit code $LASTEXITCODE : $($outputLines -join ' ')"
  }
  return (($outputLines -join [Environment]::NewLine) | ConvertFrom-Json)
}
function Require([bool] $condition, [string] $why) {
  if (-not $condition) { throw "FORMAT_CHECK_FAILED: $why" }
}
$summary = Invoke-Inspection @()
Require ($summary.archiveSha256 -ceq $expected) "native Rust byte digest"
Require ($summary.archiveBytes -eq 3127295) "native Rust input length"
Require ($summary.headerMinZoom -eq 13 -and $summary.headerMaxZoom -eq 17) "declared zoom range"
Require ($summary.actualIndexedMinZoom -eq 13 -and $summary.actualIndexedMaxZoom -eq 17) "actual index zoom range"
Require ($summary.headerZoomRangeConsistent -eq $true) "header/index consistency"
Require ($summary.indexedTileCount -eq 40) "expected indexed count"
foreach ($test in @(@(13,1),@(14,2),@(15,4),@(16,9),@(17,24))) {
  $z = "$($test[0])"
  $observed = $summary.indexedTilesByZoom.PSObject.Properties[$z].Value
  Require ($observed -eq $test[1]) "missing or excess tiles at z$z"
}
Require ($summary.metadataExplicitlyDeclaresTerrarium -eq $false) "attribution-only metadata"
Require ($null -eq $summary.encodingDeclaredInMetadata) "archive does not attest encoding"
Require ($summary.verticalDatumVerified -eq $false -and $null -eq $summary.verticalDatumId) "datum remains unknown"
Require ($summary.sourceLicenseVerified -eq $false) "license remains unverified"
Require ($summary.nativeResolutionVerified -eq $false) "native resolution remains unverified"
Require ($summary.productionGenerationReady -eq $false) "real source MUST NOT be generation ready"
# True source WebP/RGB independently decoded in the private test environment.
# Candidate Terrarium pixel heights, not verified values in any vertical datum.
$golden = @(
  @(13,4275,2891,1495.0,563.75,829.25),
  @(14,8551,5783,563.75,578.875,831.0),
  @(15,17102,11566,563.75,565.25,578.375),
  @(16,34204,23133,565.4375,568.03125,567.21875),
  @(17,68408,46266,565.453125,568.453125,568.0)
)
foreach ($g in $golden) {
  $s = Invoke-Inspection @("$($g[0])", "$($g[1])", "$($g[2])")
  $t = $s.tileSample
  Require ($t.verifiedDirectoryEntry -eq $true) "index presence z$($g[0])"
  Require ($t.webpWidth -eq 512 -and $t.webpHeight -eq 512) "512px WebP dimensions z$($g[0])"
  Require ($t.maskedPixelCount -eq 0) "missing/alpha source pixels z$($g[0])"
  Require ($t.elevationNorthWestMeters -eq $g[3]) "northwest decoded candidate height z$($g[0])"
  Require ($t.elevationCenterMeters -eq $g[4]) "center decoded candidate height z$($g[0])"
  Require ($t.elevationSouthEastMeters -eq $g[5]) "southeast decoded candidate height z$($g[0])"
  Require ($s.productionGenerationReady -eq $false) "readiness MUST remain false"
}
Write-Output "PASS - native Windows Rust PMTiles/512px RGB format inspection for SHA $expected"
Write-Output "PASS - actual 40 tile IDs and five independent WebP/Terrarium candidate numerical goldens"
Write-Output "NOT TESTED/APPROVED - source-specific legal rights, native resolution, vertical datum, signed encoding attestation, full UI/3MF"
Write-Output "productionGenerationReady=false - correct. No networking or PMTiles bytes uploaded."
