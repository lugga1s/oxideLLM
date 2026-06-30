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

$pathspecs = @(
    "README.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "NOTICE.md",
    ".github/*.md",
    ".github/ISSUE_TEMPLATE/*.md",
    ".github/instructions/*.md",
    "docs/*.md",
    "examples/*.md",
    "src/*.rs",
    "Cargo.toml",
    "Cargo.lock"
)

$scanPaths = @(& git ls-files -- $pathspecs)
if ($LASTEXITCODE -ne 0) {
    Write-Host "Unable to list tracked files with git." -ForegroundColor Red
    exit 1
}

$nonAscii = Select-String -LiteralPath $scanPaths -Pattern '[^\x00-\x7F]' -AllMatches -ErrorAction SilentlyContinue
if ($nonAscii) {
    Write-Host "Non-ASCII characters found:" -ForegroundColor Yellow
    $nonAscii | Select-Object Path, LineNumber, Line | Format-Table -AutoSize
    exit 1
}

Write-Host "Context validation passed." -ForegroundColor Green
