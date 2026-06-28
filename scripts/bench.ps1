param(
    [string]$TargetUrl = "http://localhost:8080/v1/chat/completions",
    [int]$Vus = 1000,
    [string]$Duration = "30s"
)

$env:TARGET_URL = $TargetUrl
$env:VUS = "$Vus"
$env:DURATION = $Duration

k6 run k6/proxy-vs-direct.js

