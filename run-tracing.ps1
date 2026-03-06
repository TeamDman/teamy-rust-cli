param(
	[Parameter(ValueFromRemainingArguments = $true)]
	[string[]]$QueryArgs
)

$slug = "$((Get-Date).ToString("yyyy-MM-dd_HH-mm-ss")).tracy"

if (-not (Get-Command tracy-capture.exe -ErrorAction SilentlyContinue)) {
	throw "tracy-capture.exe not found in PATH"
}

if (-not (Get-Command tracy-profiler.exe -ErrorAction SilentlyContinue)) {
	Write-Warning "tracy-profiler.exe not found in PATH; capture will still be produced at $slug"
}

if (-not $QueryArgs -or $QueryArgs.Count -eq 0) {
	$QueryArgs = @("'flower .jar$")
}

Write-Host "Logging performance information to $slug"
$capture = $null
$wt = Get-Command wt.exe -ErrorAction SilentlyContinue

if ($wt) {
	Start-Process -FilePath "wt.exe" -ArgumentList @("-w", "new", "tracy-capture.exe", "-o", $slug)
} else {
	Write-Warning "wt.exe not found in PATH; launching tracy-capture in the current session"
	$capture = Start-Process -FilePath "tracy-capture.exe" -ArgumentList @("-o", $slug) -PassThru
}

try {
	Write-Host "Running: cargo run --release --features tracy -- $($QueryArgs -join ' ')"
	cargo run --release --features tracy -- @QueryArgs --debug
}
finally {
	$slugPattern = [Regex]::Escape($slug)
	Get-CimInstance Win32_Process -Filter "Name = 'tracy-capture.exe'" -ErrorAction SilentlyContinue |
		Where-Object { $_.CommandLine -and $_.CommandLine -match $slugPattern } |
		ForEach-Object { Stop-Process -Id $_.ProcessId -ErrorAction SilentlyContinue }

	if ($capture -and -not $capture.HasExited) {
		$null = $capture.CloseMainWindow()
		Start-Sleep -Milliseconds 500
		if (-not $capture.HasExited) {
			$capture.Kill()
		}
	}

}

if (Get-Command tracy-profiler.exe -ErrorAction SilentlyContinue) {
	Write-Host "Displaying results"
	tracy-profiler.exe "$slug"
} else {
	Write-Host "Capture saved to $slug"
}