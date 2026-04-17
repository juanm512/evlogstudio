param(
    [string]$Endpoint = "http://localhost:8080/ingest",
    [string]$Token = "",
    [int]$IntervalMs = 1500
)

if (-not $Token) {
    Write-Host "Uso: .\test_ingest.ps1 -Token tok_xxx [-Endpoint http://...] [-IntervalMs 1500]"
    exit 1
}

$levels   = @("info","info","info","warn","error","debug","fatal")
$messages = @(
    "User logged in",
    "Payment processed",
    "Cache miss",
    "Slow query detected",
    "Connection timeout",
    "Request completed",
    "Token refreshed",
    "File uploaded",
    "Email sent",
    "Webhook received"
)
$plans    = @("free","pro","enterprise")
$methods  = @("GET","POST","PUT","DELETE")
$paths    = @("/api/users","/api/orders","/api/payments","/api/products","/api/auth")

Write-Host "Sending logs to $Endpoint - Ctrl+C to stop"

$i = 0
while ($true) {
    $i++
    $level   = $levels   | Get-Random
    $message = $messages | Get-Random
    $plan    = $plans    | Get-Random
    $method  = $methods  | Get-Random
    $path    = $paths    | Get-Random
    $userId  = "u" + (Get-Random -Minimum 1 -Maximum 100)
    $duration = Get-Random -Minimum 5 -Maximum 2000

    $body = @{
        message     = $message
        level       = $level
        duration_ms = $duration
        user        = @{ id = $userId; plan = $plan }
        http        = @{ method = $method; path = $path }
        request_id  = [guid]::NewGuid().ToString()
    } | ConvertTo-Json -Compress

    try {
        $headers = @{ 
            Authorization = "Bearer $Token"
            "Content-Type"  = "application/json" 
        }
        $res = Invoke-RestMethod -Uri $Endpoint -Method POST -Headers $headers -Body $body
        Write-Host "[$i] $level - $message (${duration}ms)"
    } catch {
        $errMsg = $_.Exception.Message
        Write-Host "[$i] ERROR: $errMsg" -ForegroundColor Red
    }

    Start-Sleep -Milliseconds $IntervalMs
}