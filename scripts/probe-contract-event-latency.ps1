[CmdletBinding()]
param(
    [Uri]$BaseUrl = 'http://127.0.0.1:3000',
    [ValidatePattern('^[A-Za-z0-9_-]+$')]
    [string]$Symbol = 'ETH',
    [ValidateRange(1, 1000)]
    [int]$Samples = 30,
    [ValidateRange(1, 64)]
    [int]$Concurrency = 8,
    [switch]$AllowRemote
)

$ErrorActionPreference = 'Stop'

function Test-IsLoopbackHost {
    param([Parameter(Mandatory = $true)][string]$HostName)

    if ($HostName -eq 'localhost') {
        return $true
    }

    $address = $null
    if ([System.Net.IPAddress]::TryParse($HostName, [ref]$address)) {
        return [System.Net.IPAddress]::IsLoopback($address)
    }

    return $false
}

function Get-Percentile {
    param(
        [Parameter(Mandatory = $true)][double[]]$Values,
        [Parameter(Mandatory = $true)][ValidateRange(0.0, 1.0)][double]$Percentile
    )

    if ($Values.Count -eq 0) {
        return 0
    }

    $ordered = @($Values | Sort-Object)
    $index = [Math]::Max(0, [Math]::Ceiling($Percentile * $ordered.Count) - 1)
    return [Math]::Round($ordered[$index], 2)
}

if ($BaseUrl.Scheme -notin @('http', 'https')) {
    throw 'BaseUrl must use http or https.'
}
if (-not [string]::IsNullOrEmpty($BaseUrl.UserInfo)) {
    throw 'BaseUrl must not contain credentials.'
}
if (-not $AllowRemote -and -not (Test-IsLoopbackHost -HostName $BaseUrl.DnsSafeHost)) {
    throw 'Remote targets are blocked by default. Use -AllowRemote only with explicit authorization.'
}

$encodedSymbol = [Uri]::EscapeDataString($Symbol.ToUpperInvariant())
$endpoints = @(
    [pscustomobject]@{
        Name = 'summary'
        Path = "/api/contract-whale/summary?symbol=$encodedSymbol"
        P95LimitMs = 2000
    },
    [pscustomobject]@{
        Name = 'latest'
        Path = "/api/contract-whale/latest?symbol=$encodedSymbol&limit=50"
        P95LimitMs = 2000
    },
    [pscustomobject]@{
        Name = 'contract-events'
        Path = "/api/contract-events?symbol=$encodedSymbol&range=24h&limit=20&min_notional_usd=10000000"
        P95LimitMs = 3000
    }
)

$requests = for ($sample = 1; $sample -le $Samples; $sample += 1) {
    foreach ($endpoint in $endpoints) {
        [pscustomobject]@{
            Name = $endpoint.Name
            Path = $endpoint.Path
            Uri = [Uri]::new($BaseUrl, $endpoint.Path).AbsoluteUri
        }
    }
}

Add-Type -AssemblyName System.Net.Http

$worker = {
    param($Name, $Path, $RequestUri)

    $handler = [System.Net.Http.HttpClientHandler]::new()
    $handler.UseProxy = $false
    $client = [System.Net.Http.HttpClient]::new($handler)
    $client.Timeout = [TimeSpan]::FromMilliseconds(6000)
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    $status = 0
    try {
        $response = $client.GetAsync(
            $RequestUri,
            [System.Net.Http.HttpCompletionOption]::ResponseHeadersRead
        ).GetAwaiter().GetResult()
        try {
            $status = [int]$response.StatusCode
        }
        finally {
            $response.Dispose()
        }
    }
    catch {
        $status = 0
    }
    finally {
        $stopwatch.Stop()
        $client.Dispose()
        $handler.Dispose()
    }

    [pscustomobject]@{
        Name = $Name
        Path = $Path
        Status = $status
        ElapsedMs = [Math]::Round($stopwatch.Elapsed.TotalMilliseconds, 2)
    }
}

$pool = [RunspaceFactory]::CreateRunspacePool(1, $Concurrency)
$jobs = [System.Collections.Generic.List[object]]::new()
$results = [System.Collections.Generic.List[object]]::new()

try {
    $pool.Open()
    foreach ($request in $requests) {
        $powershell = [PowerShell]::Create()
        $powershell.RunspacePool = $pool
        [void]$powershell.AddScript($worker.ToString())
        [void]$powershell.AddArgument($request.Name)
        [void]$powershell.AddArgument($request.Path)
        [void]$powershell.AddArgument($request.Uri)
        $jobs.Add([pscustomobject]@{
            PowerShell = $powershell
            Handle = $powershell.BeginInvoke()
            Name = $request.Name
            Path = $request.Path
        })
    }

    foreach ($job in $jobs) {
        try {
            $output = @($job.PowerShell.EndInvoke($job.Handle))
            if ($output.Count -gt 0) {
                $results.Add($output[0])
            }
            else {
                $results.Add([pscustomobject]@{
                    Name = $job.Name
                    Path = $job.Path
                    Status = 0
                    ElapsedMs = 6000
                })
            }
        }
        catch {
            $results.Add([pscustomobject]@{
                Name = $job.Name
                Path = $job.Path
                Status = 0
                ElapsedMs = 6000
            })
        }
        finally {
            $job.PowerShell.Dispose()
        }
    }
}
finally {
    $pool.Close()
    $pool.Dispose()
}

$failed = $false
foreach ($result in $results) {
    Write-Output ("{0} status={1} elapsed_ms={2}" -f $result.Path, $result.Status, $result.ElapsedMs)
    if ($result.Status -notin @(200, 503) -or $result.ElapsedMs -gt 5000) {
        $failed = $true
    }
}

foreach ($endpoint in $endpoints) {
    $endpointResults = @($results | Where-Object Name -eq $endpoint.Name)
    $elapsed = [double[]]@($endpointResults | ForEach-Object { $_.ElapsedMs })
    $statuses = @($endpointResults | ForEach-Object { $_.Status } | Sort-Object -Unique) -join ','
    $p50 = Get-Percentile -Values $elapsed -Percentile 0.50
    $p95 = Get-Percentile -Values $elapsed -Percentile 0.95
    $p99 = Get-Percentile -Values $elapsed -Percentile 0.99
    $max = if ($elapsed.Count -gt 0) { [Math]::Round(($elapsed | Measure-Object -Maximum).Maximum, 2) } else { 0 }
    Write-Output ("{0} status={1} p50_ms={2} p95_ms={3} p99_ms={4} max_ms={5}" -f $endpoint.Path, $statuses, $p50, $p95, $p99, $max)

    if ($p95 -ge $endpoint.P95LimitMs -or $max -gt 5000) {
        $failed = $true
    }
}

if ($failed) {
    exit 1
}

exit 0
