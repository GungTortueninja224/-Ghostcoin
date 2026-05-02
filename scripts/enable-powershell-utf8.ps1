param(
    [switch]$Persist
)

$utf8ProfileBlock = @'
# GhostCoin UTF-8 console settings
[Console]::InputEncoding = [System.Text.UTF8Encoding]::new()
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
chcp 65001 > $null
'@

function Enable-Utf8CurrentSession {
    [Console]::InputEncoding = [System.Text.UTF8Encoding]::new()
    [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
    chcp 65001 > $null
}

function Enable-Utf8PersistedProfile {
    try {
        $profileDir = Split-Path -Path $PROFILE -Parent

        if (-not (Test-Path -LiteralPath $profileDir)) {
            New-Item -ItemType Directory -Path $profileDir -Force -ErrorAction Stop | Out-Null
        }

        if (-not (Test-Path -LiteralPath $PROFILE)) {
            New-Item -ItemType File -Path $PROFILE -Force -ErrorAction Stop | Out-Null
        }

        $profileContent = Get-Content -LiteralPath $PROFILE -Raw -ErrorAction Stop
        if ($profileContent -notmatch [Regex]::Escape("GhostCoin UTF-8 console settings")) {
            Add-Content -LiteralPath $PROFILE -Value "`r`n$utf8ProfileBlock`r`n" -ErrorAction Stop
            Write-Host "UTF-8 block added to PowerShell profile: $PROFILE"
        }
        else {
            Write-Host "UTF-8 block already present in PowerShell profile."
        }
    }
    catch {
        Write-Warning "Could not update PowerShell profile automatically: $($_.Exception.Message)"
        Write-Host "Open PowerShell as your regular user and rerun this script with -Persist."
    }
}

Enable-Utf8CurrentSession
Write-Host "UTF-8 enabled for current PowerShell session."

if ($Persist) {
    Enable-Utf8PersistedProfile
}
else {
    Write-Host "Run with -Persist to make it permanent in your PowerShell profile."
}
