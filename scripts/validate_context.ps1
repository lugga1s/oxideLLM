$ErrorActionPreference = "Stop"

$required = @(
    "README.md"
)

$missing = @()
foreach ($path in $required) {
    if (-not (Test-Path $path)) {
        $missing += $path
    }
}

if ($missing.Count -gt 0) {
    Write-Host "Missing required files:" -ForegroundColor Red
    $missing | ForEach-Object { Write-Host " - $_" -ForegroundColor Red }
    exit 1
}

if (Test-Path ".context/agent-db") {
    $jsonFiles = Get-ChildItem ".context/agent-db/*.json"
    foreach ($file in $jsonFiles) {
        try {
            Get-Content -Raw $file.FullName | ConvertFrom-Json | Out-Null
        } catch {
            Write-Host "Invalid JSON: $($file.FullName)" -ForegroundColor Red
            Write-Host $_.Exception.Message -ForegroundColor Red
            exit 1
        }
    }
}

$scanPaths = @(
    "README.md",
    "AGENTS.md",
    "GEMINI.md",
    "DEEPSEEK.md",
    "CLAUDE.md",
    "docs/*.md",
    ".context/*.md",
    ".context/decisions/*.md",
    ".context/agent-db/*.json",
    "src/*.rs",
    "Cargo.toml"
) | Where-Object { Test-Path $_ }

$nonAscii = Select-String -Path $scanPaths -Pattern '[^\x00-\x7F]' -AllMatches -ErrorAction SilentlyContinue
if ($nonAscii) {
    Write-Host "Non-ASCII characters found:" -ForegroundColor Yellow
    $nonAscii | Select-Object Path, LineNumber, Line | Format-Table -AutoSize
    exit 1
}

Write-Host "Context validation passed." -ForegroundColor Green
