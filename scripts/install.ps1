param(
    [string]$Destination = "$HOME\bin"
)

$ErrorActionPreference = "Stop"

$repo = "RivoLink/leaf"
$destinationDir = $Destination
$destinationBin = Join-Path $destinationDir "leaf.exe"
$assetName = "leaf-windows-x86_64.exe"

New-Item -ItemType Directory -Force -Path $destinationDir | Out-Null

$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
if (-not $release.tag_name) {
    throw "Unable to resolve latest release tag for $repo"
}

$tagName = $release.tag_name
$downloadUrl = "https://github.com/$repo/releases/download/$tagName/$assetName"

Invoke-WebRequest -Uri $downloadUrl -OutFile $destinationBin

Write-Host "Installed leaf $tagName to $destinationBin"
Write-Host "Add $destinationDir to PATH if needed."
