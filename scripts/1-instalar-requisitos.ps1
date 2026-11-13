#Requires -RunAsAdministrator
$ErrorActionPreference = "Stop"

function Install-Pkg($id, $extra = @()) {
    $installed = winget list --id $id -e --accept-source-agreements 2>$null | Select-String $id
    if ($installed) {
        Write-Host "[ok] $id ya instalado" -ForegroundColor Green
        return
    }
    Write-Host "[..] Instalando $id" -ForegroundColor Cyan
    winget install --id $id -e --accept-package-agreements --accept-source-agreements @extra
}

if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    throw "winget no está disponible. Instala 'App Installer' desde Microsoft Store."
}

Install-Pkg "Git.Git"
Install-Pkg "Microsoft.VisualStudio.2022.BuildTools" @(
    "--override", "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
)
Install-Pkg "Rustlang.Rustup"
Install-Pkg "OpenJS.NodeJS.LTS"
Install-Pkg "GitHub.cli"
Install-Pkg "Anthropic.ClaudeCode"

# Refrescar PATH en esta sesión
$env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
            [Environment]::GetEnvironmentVariable("Path", "User")

if (Get-Command rustup -ErrorAction SilentlyContinue) {
    rustup default stable-msvc
    rustup component add clippy rustfmt
}

# WebView2 (Tauri). Viene con Windows 11; en Windows 10 puede faltar.
$wv2 = Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
if (-not $wv2) { Install-Pkg "Microsoft.EdgeWebView2Runtime" }

Write-Host "`nListo. Cierra esta terminal y abre una nueva para verificar versiones." -ForegroundColor Green
