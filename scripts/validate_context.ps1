$ErrorActionPreference = "Stop"

$required = @(
    "README.md",
    "AGENTS.md",
    "GEMINI.md",
    "DEEPSEEK.md",
    "CLAUDE.md",
    ".github/copilot-instructions.md",
    "docs/implementation-playbook.md",
    "docs/agent-execution-system.md",
    "docs/agent-task-cards.md",
    "docs/multi-agent-handoff.md",
    "docs/agent-quality-scorecard.md",
    "docs/agent-readiness-matrix.md",
    "docs/review-gates.md",
    "docs/context-packets.md",
    "docs/verification-ledger.md",
    "docs/validation-gates.md",
    ".context/agent-db/project_facts.json",
    ".context/agent-db/session_plan.json",
    ".context/agent-db/task_cards.json"
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
)

$nonAscii = Select-String -Path $scanPaths -Pattern '[^\x00-\x7F]' -AllMatches -ErrorAction SilentlyContinue
if ($nonAscii) {
    Write-Host "Non-ASCII characters found:" -ForegroundColor Yellow
    $nonAscii | Select-Object Path, LineNumber, Line | Format-Table -AutoSize
    exit 1
}

Write-Host "Context validation passed." -ForegroundColor Green
