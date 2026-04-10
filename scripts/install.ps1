param(
    [string]$Destination = "$env:LOCALAPPDATA\Programs\leaf"
)

$ErrorActionPreference = "Stop"

$Repo = "RivoLink/leaf"
$AssetName = "leaf-windows-x86_64.exe"

function Write-Info {
    param([string]$Message)
    Write-Host $Message
}

function Ensure-InstallDir {
    param([string]$Dir)

    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
}

function Get-LatestTag {
    param([string]$Repo)

    $release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    if (-not $release.tag_name) {
        throw "Unable to resolve latest release tag for $Repo"
    }
    return $release.tag_name
}

function Get-DownloadUrl {
    param(
        [string]$Repo,
        [string]$Tag,
        [string]$Asset
    )

    return "https://github.com/$Repo/releases/download/$Tag/$Asset"
}

function Install-Binary {
    param(
        [string]$Url,
        [string]$DestinationBin
    )

    Invoke-WebRequest -Uri $Url -OutFile $DestinationBin
}

function Add-ToUserPath {
    param([string]$Dir)

    $currentPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $pathParts = @($currentPath -split ';' | Where-Object { $_ -ne '' })

    if ($Dir -notin $pathParts) {
        $pathParts += $Dir
        [Environment]::SetEnvironmentVariable('Path', ($pathParts -join ';'), 'User')
        if ($env:Path) {
            $env:Path = "$Dir;$env:Path"
        } else {
            $env:Path = $Dir
        }
        Write-Info "Added $Dir to your user PATH"
        Write-Info "PATH updated for current session"
    } else {
        Write-Info "$Dir is already in your user PATH"
    }
}

$destinationDir = $Destination
$destinationBin = Join-Path $destinationDir "leaf.exe"

Ensure-InstallDir -Dir $destinationDir

$tagName = Get-LatestTag -Repo $Repo
$downloadUrl = Get-DownloadUrl -Repo $Repo -Tag $tagName -Asset $AssetName

Install-Binary -Url $downloadUrl -DestinationBin $destinationBin
Add-ToUserPath -Dir $destinationDir

Write-Info "Installed leaf $tagName to $destinationBin"
