[CmdletBinding()]
param(
	[Parameter(Mandatory = $true)]
	[string]$OtherRepoPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$sourceRepoPath = $PSScriptRoot
$destinationRepoPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OtherRepoPath)

if (-not (Test-Path -LiteralPath $sourceRepoPath -PathType Container)) {
	throw "Source repository path does not exist: $sourceRepoPath"
}

if (-not (Test-Path -LiteralPath $destinationRepoPath)) {
	New-Item -ItemType Directory -Path $destinationRepoPath -Force | Out-Null
}

if (-not (Test-Path -LiteralPath $destinationRepoPath -PathType Container)) {
	throw "Destination path is not a directory: $destinationRepoPath"
}

$excludedDirectories = @(
	(Join-Path -Path $sourceRepoPath -ChildPath '.git'),
	(Join-Path -Path $sourceRepoPath -ChildPath 'target')
)

Write-Verbose "Source: $sourceRepoPath"
Write-Verbose "Destination: $destinationRepoPath"
Write-Verbose "Excluded directories: $($excludedDirectories -join ', ')"
Write-Output "Copying template files from '$sourceRepoPath' to '$destinationRepoPath'..."

# /E  : copy subdirectories, including empty ones.
# /XD : exclude directories from the copy operation.
& robocopy $sourceRepoPath $destinationRepoPath /E /XD $excludedDirectories
$robocopyExitCode = $LASTEXITCODE

$robocopyExitCodeTable = [ordered]@{
	0  = 'No files were copied. No failures occurred. No files were mismatched.'
	1  = 'All files were copied successfully.'
	2  = 'Extra files or directories were detected. Nothing was copied.'
	3  = 'Some files were copied. Extra files were present.'
	4  = 'Some mismatched files or directories were detected.'
	5  = 'Some files were copied. Some files were mismatched.'
	6  = 'Extra files and mismatched files were present. Nothing was copied.'
	7  = 'Files were copied, and files were mismatched, and extra files were present.'
	8  = 'Several files or directories could not be copied (copy failures occurred).'
	9  = 'Files were copied, but some failures occurred.'
	10 = 'Extra files were present, and some failures occurred.'
	11 = 'Files were copied, extra files were present, and some failures occurred.'
	12 = 'Mismatched files were present, and some failures occurred.'
	13 = 'Files were copied, mismatched files were present, and some failures occurred.'
	14 = 'Extra files and mismatched files were present, and some failures occurred.'
	15 = 'Files were copied, extra files and mismatched files were present, and some failures occurred.'
	16 = 'Serious error. Robocopy did not copy any files.'
}

if ($PSBoundParameters.ContainsKey('Verbose')) {
	Write-Verbose 'Robocopy exit code meanings:'
	foreach ($entry in $robocopyExitCodeTable.GetEnumerator()) {
		Write-Verbose ("  {0,2}: {1}" -f $entry.Key, $entry.Value)
	}
}

$robocopyExitDescription = if ($robocopyExitCodeTable.Contains($robocopyExitCode)) {
	$robocopyExitCodeTable[$robocopyExitCode]
} else {
	'Unknown robocopy exit code.'
}

if ($robocopyExitCode -gt 7) {
	throw "robocopy failed with exit code ${robocopyExitCode}: $robocopyExitDescription"
}

Write-Output "Copy complete. robocopy exit code: $robocopyExitCode ($robocopyExitDescription)"
