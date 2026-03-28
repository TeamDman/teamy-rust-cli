param(
	[Parameter(ValueFromRemainingArguments = $true)]
	[string[]]$QueryArgs
)

function Format-Elapsed {
	param(
		[Parameter(Mandatory = $true)]
		[TimeSpan]$Elapsed
	)

	if ($Elapsed.TotalHours -ge 1) {
		return $Elapsed.ToString("hh\:mm\:ss\.fff")
	}

	return $Elapsed.ToString("mm\:ss\.fff")
}

function Get-TracyCaptureProcesses {
	param(
		[Parameter(Mandatory = $true)]
		[string]$CapturePath
	)

	$slugPattern = [Regex]::Escape($CapturePath)
	Get-CimInstance Win32_Process -Filter "Name = 'tracy-capture.exe'" -ErrorAction SilentlyContinue |
		Where-Object { $_.CommandLine -and $_.CommandLine -match $slugPattern } |
		ForEach-Object {
			try {
				Get-Process -Id $_.ProcessId -ErrorAction Stop
			} catch {
				$null
			}
		} |
		Where-Object { $_ -ne $null }
}

function Wait-ForTracyCaptureReady {
	param(
		[Parameter(Mandatory = $true)]
		[string]$CapturePath,
		[Parameter(Mandatory = $true)]
		[TimeSpan]$Timeout
	)

	$deadline = (Get-Date).Add($Timeout)
	do {
		$processes = @(Get-TracyCaptureProcesses -CapturePath $CapturePath)
		if ($processes.Count -gt 0) {
			return $processes
		}

		Start-Sleep -Milliseconds 250
	} while ((Get-Date) -lt $deadline)

	throw "Timed out waiting $(Format-Elapsed $Timeout) for tracy-capture to start for $CapturePath"
}

function Stop-TracyCaptureGracefully {
	param(
		[Parameter(Mandatory = $true)]
		[string]$CapturePath
	)

	$processes = @(Get-TracyCaptureProcesses -CapturePath $CapturePath)
	if ($processes.Count -eq 0) {
		return [TimeSpan]::Zero
	}

	$shutdownStopwatch = [System.Diagnostics.Stopwatch]::StartNew()
	foreach ($process in $processes) {
		if ($process.HasExited) {
			continue
		}

		$requestedClose = $false
		try {
			$requestedClose = $process.CloseMainWindow()
		} catch {
			$requestedClose = $false
		}

		if (-not $requestedClose) {
			try {
				Stop-Process -Id $process.Id -ErrorAction SilentlyContinue
			} catch {
				# Ignore shutdown failures and let the wait/kill fallback below handle them.
			}
		}
	}

	$waitDeadline = (Get-Date).AddSeconds(30)
	do {
		Start-Sleep -Milliseconds 250
		$processes = @($processes | Where-Object {
			try {
				$_.Refresh()
				-not $_.HasExited
			} catch {
				$false
			}
		})
	} while ($processes.Count -gt 0 -and (Get-Date) -lt $waitDeadline)

	foreach ($process in $processes) {
		try {
			if (-not $process.HasExited) {
				$process.Kill()
			}
		} catch {
			# Ignore final cleanup failures.
		}
	}

	$shutdownStopwatch.Stop()
	return $shutdownStopwatch.Elapsed
}

$overallStopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$captureLaunchElapsed = $null
$commandElapsed = $null
$cleanupElapsed = $null
$profilerElapsed = $null
$captureShutdownElapsed = [TimeSpan]::Zero
$captureFlushDelay = [TimeSpan]::FromSeconds(1)
$captureStartupTimeout = [TimeSpan]::FromSeconds(10)

$captureDir = Join-Path $PSScriptRoot "tracy"
if (-not (Test-Path $captureDir)) {
	$null = New-Item -ItemType Directory -Path $captureDir
}

$slug = "$((Get-Date).ToString("yyyy-MM-dd_HH-mm-ss")).tracy"
$capturePath = Join-Path $captureDir $slug

if (-not (Get-Command tracy-capture.exe -ErrorAction SilentlyContinue)) {
	throw "tracy-capture.exe not found in PATH"
}

if (-not (Get-Command tracy-profiler.exe -ErrorAction SilentlyContinue)) {
	Write-Warning "tracy-profiler.exe not found in PATH; capture will still be produced at $capturePath"
}

if (-not $QueryArgs -or $QueryArgs.Count -eq 0) {
	# TODO(template): replace the default traced command below to match the generated project's command surface.
	$QueryArgs = @("home", "show")
}

Write-Host "Capture: $capturePath"
Write-Host "Logging performance information to $capturePath"
$capture = $null
$wt = Get-Command wt.exe -ErrorAction SilentlyContinue
$captureLaunchStopwatch = [System.Diagnostics.Stopwatch]::StartNew()

if ($wt) {
	Start-Process -FilePath "wt.exe" -ArgumentList @("-w", "new", "tracy-capture.exe", "-o", $capturePath)
} else {
	Write-Warning "wt.exe not found in PATH; launching tracy-capture in the current session"
	$capture = Start-Process -FilePath "tracy-capture.exe" -ArgumentList @("-o", $capturePath) -PassThru
}
$captureLaunchStopwatch.Stop()
$captureLaunchElapsed = $captureLaunchStopwatch.Elapsed
Write-Host "Capture launch time: $(Format-Elapsed $captureLaunchElapsed)"
Write-Host "Waiting for tracy-capture process to appear (timeout $(Format-Elapsed $captureStartupTimeout))"
$captureProcesses = @(Wait-ForTracyCaptureReady -CapturePath $capturePath -Timeout $captureStartupTimeout)
Write-Host "tracy-capture ready (pid: $($captureProcesses.Id -join ', '))"
Write-Host "Waiting 00:01.000 for tracy-capture to get ready"
Start-Sleep -Seconds 1

try {
	Write-Host "Running: cargo run --release --features tracy -- $($QueryArgs -join ' ')"
	$commandStopwatch = [System.Diagnostics.Stopwatch]::StartNew()
	cargo run --release --features tracy -- @QueryArgs --log-filter debug
	$commandStopwatch.Stop()
	$commandElapsed = $commandStopwatch.Elapsed
	Write-Host "Traced command time: $(Format-Elapsed $commandElapsed)"
	if ($LASTEXITCODE -ne 0) {
		throw "cargo run failed with exit code $LASTEXITCODE"
	}
}
finally {
	$cleanupStopwatch = [System.Diagnostics.Stopwatch]::StartNew()
	Write-Host "Waiting $(Format-Elapsed $captureFlushDelay) before closing tracy-capture"
	Start-Sleep -Milliseconds ([int]$captureFlushDelay.TotalMilliseconds)
	$captureShutdownElapsed = Stop-TracyCaptureGracefully -CapturePath $capturePath
	$cleanupStopwatch.Stop()
	$cleanupElapsed = $cleanupStopwatch.Elapsed
	Write-Host "Capture cleanup time: $(Format-Elapsed $cleanupElapsed)"
	Write-Host "Capture shutdown wait: $(Format-Elapsed $captureShutdownElapsed)"
}

if (Get-Command tracy-profiler.exe -ErrorAction SilentlyContinue) {
	Write-Host "Displaying results from $capturePath"
	$profilerStopwatch = [System.Diagnostics.Stopwatch]::StartNew()
	tracy-profiler.exe "$capturePath"
	$profilerStopwatch.Stop()
	$profilerElapsed = $profilerStopwatch.Elapsed
	Write-Host "Profiler time: $(Format-Elapsed $profilerElapsed)"
} else {
	Write-Host "Capture saved to $capturePath"
}

$overallStopwatch.Stop()
Write-Host "Timing summary:"
Write-Host "  capture launch: $(Format-Elapsed $captureLaunchElapsed)"
if ($commandElapsed) {
	Write-Host "  traced command: $(Format-Elapsed $commandElapsed)"
}
Write-Host "  cleanup:        $(Format-Elapsed $cleanupElapsed)"
Write-Host "  capture stop:   $(Format-Elapsed $captureShutdownElapsed)"
if ($profilerElapsed) {
	Write-Host "  profiler:       $(Format-Elapsed $profilerElapsed)"
}
Write-Host "  total wrapper:  $(Format-Elapsed $overallStopwatch.Elapsed)"