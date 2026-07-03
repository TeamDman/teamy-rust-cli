param(
	[Parameter(ValueFromRemainingArguments = $true)]
	[string[]]$QueryArgs
)

if (-not $QueryArgs -or $QueryArgs.Count -eq 0) {
	# TODO(template): replace the default profiled command below to match the generated project's command surface.
	$QueryArgs = @("home", "show")
}

$profiler = Get-Command teamy-profiler -ErrorAction SilentlyContinue
if (-not $profiler) {
	throw "teamy-profiler not found in PATH"
}

& $profiler.Source run cargo `
	--project $PSScriptRoot `
	--bin teamy-rust-cli `
	--profile release `
	--feature tracy `
	-- @QueryArgs
exit $LASTEXITCODE
