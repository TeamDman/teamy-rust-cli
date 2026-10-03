# Compile both rejected and accepted examples using the project's actual lint policy.
[CmdletBinding()]
param(
    [string]$ManifestPath = (Join-Path $PSScriptRoot 'Cargo.toml')
)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$taskManifest = (Resolve-Path -LiteralPath $ManifestPath).Path
$taskProject = Split-Path -Parent $taskManifest
$taskManifestText = [IO.File]::ReadAllText($taskManifest)
$taskLintTables = [regex]::Matches($taskManifestText, '(?ms)^\[lints\.(?:rust|clippy)\]\r?\n.*?(?=^\[|\z)')
if ($taskLintTables.Count -ne 2) { throw 'The policy fixtures require explicit package Rust and Clippy lint tables.' }
$taskLintText = ($taskLintTables | ForEach-Object { $_.Value.Trim() }) -join "`n`n"
$taskClippyConfig = Join-Path $taskProject 'clippy.toml'
if (-not (Test-Path -LiteralPath $taskClippyConfig -PathType Leaf)) { throw 'No package-local clippy.toml was found.' }
$taskMetadataText = & cargo metadata --offline --locked --no-deps --format-version 1 --manifest-path $taskManifest
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed before lint policy checks.' }
$taskMetadata = $taskMetadataText | ConvertFrom-Json
$taskRunRoot = Join-Path $taskMetadata.target_directory ('lint-policy/' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $taskRunRoot -Force | Out-Null
$taskTarget = Join-Path $taskRunRoot 'compiled'
$taskCases = @(
    @{ Name = 'println'; Source = 'fn main() { println!("debugging remnant"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'print'; Source = 'fn main() { print!("debugging remnant"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'eprintln'; Source = 'fn main() { eprintln!("debugging remnant"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'eprint'; Source = 'fn main() { eprint!("debugging remnant"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'qualified'; Source = 'fn main() { std::println!("debugging remnant"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'renamed'; Source = 'use std::println as renamed; fn main() { renamed!("debugging remnant"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'library'; Source = 'fn main() {}'; Library = 'pub fn helper() { println!("debugging remnant"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'test-target'; Source = 'fn main() {}'; Test = '#[test] fn remnant() { eprintln!("debugging remnant"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'example-target'; Source = 'fn main() {}'; Example = 'fn main() { println!("debugging remnant"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'build-without-exception'; Source = 'fn main() {}'; Build = 'fn main() { println!("cargo:rerun-if-changed=build.rs"); }'; Expected = 'disallowed_macros' },
    @{ Name = 'writer'; Source = 'use std::io::Write; fn render(writer: &mut impl Write) -> std::io::Result<()> { writeln!(writer, "command result")?; writer.flush() } fn main() -> std::io::Result<()> { render(&mut Vec::new()) }' },
    @{ Name = 'bootstrap-exception'; Source = 'fn main() { bootstrap_failure(); } #[expect(clippy::disallowed_macros, reason = "Requested logging subscriber could not initialize")] fn bootstrap_failure() { eprintln!("logging initialization failed"); }' },
    @{ Name = 'cargo-exception'; Source = 'fn main() {}'; Build = 'fn main() { cargo_instruction(format_args!("rerun-if-changed=build.rs")); } #[expect(clippy::disallowed_macros, reason = "Cargo instruction protocol")] fn cargo_instruction(instruction: std::fmt::Arguments<''_>) { println!("cargo:{instruction}"); }' },
    @{ Name = 'stale-exception'; Source = 'fn main() { bootstrap_failure(); } #[expect(clippy::disallowed_macros, reason = "Previously required bootstrap print")] fn bootstrap_failure() {}'; Expected = 'unfulfilled_lint_expectations' }
)
$taskResults = @()
foreach ($taskCase in $taskCases) {
    $taskDirectory = Join-Path $taskRunRoot $taskCase.Name
    New-Item -ItemType Directory -Path (Join-Path $taskDirectory 'src') -Force | Out-Null
    $taskFixtureManifest = @"
[package]
name = "stdio-policy-fixture"
version = "0.0.0"
edition = "2024"
authors = ["TeamDman"]
description = "Compile-time checks for explicit command writers and tracing diagnostics"
license = "MPL-2.0"
repository = "https://github.com/TeamDman/teamy-rust-cli"
readme = "README.md"
keywords = ["cli"]
categories = ["command-line-utilities"]

[workspace]

$taskLintText
"@
    [IO.File]::WriteAllText((Join-Path $taskDirectory 'Cargo.toml'), $taskFixtureManifest)
    [IO.File]::WriteAllText((Join-Path $taskDirectory 'README.md'), 'Compile-time lint policy fixture.')
    Copy-Item -LiteralPath $taskClippyConfig -Destination (Join-Path $taskDirectory 'clippy.toml')
    [IO.File]::WriteAllText((Join-Path $taskDirectory 'src/main.rs'), $taskCase.Source)
    foreach ($taskExtra in @(@{ Key = 'Library'; Path = 'src/lib.rs' }, @{ Key = 'Test'; Path = 'tests/remnant.rs' }, @{ Key = 'Example'; Path = 'examples/remnant.rs' }, @{ Key = 'Build'; Path = 'build.rs' })) {
        if ($taskCase.ContainsKey($taskExtra.Key)) {
            $taskExtraPath = Join-Path $taskDirectory $taskExtra.Path
            New-Item -ItemType Directory -Path (Split-Path -Parent $taskExtraPath) -Force | Out-Null
            [IO.File]::WriteAllText($taskExtraPath, $taskCase[$taskExtra.Key])
        }
    }
    $taskOutput = @(& cargo clippy --offline --manifest-path (Join-Path $taskDirectory 'Cargo.toml') --target-dir $taskTarget --all-targets --no-deps --message-format json 2>&1)
    $taskExit = $LASTEXITCODE
    $taskOutputText = ($taskOutput | ForEach-Object { $_.ToString() }) -join "`n"
    [IO.File]::WriteAllText((Join-Path $taskDirectory 'diagnostics.ndjson'), $taskOutputText)
    # A negative fixture must fail for its expected deny-level lint, not an unrelated error.
    $taskCodes = @()
    foreach ($taskLine in $taskOutput) {
        if ($taskLine.ToString().StartsWith('{')) {
            $taskDiagnostic = $taskLine.ToString() | ConvertFrom-Json
            if ($taskDiagnostic.reason -eq 'compiler-message' -and $taskDiagnostic.message.level -eq 'error') {
                $taskCodes += $taskDiagnostic.message.code.code
            }
        }
    }
    $taskPassed = if ($taskCase.ContainsKey('Expected')) {
        $taskExpectedCodes = @($taskCase.Expected, "clippy::$($taskCase.Expected)")
        $taskExit -ne 0 -and @($taskCodes | Where-Object { $_ -in $taskExpectedCodes }).Count -gt 0 -and @($taskCodes | Where-Object { $_ -notin $taskExpectedCodes }).Count -eq 0
    } else { $taskExit -eq 0 }
    $taskResults += [ordered]@{ name = $taskCase.Name; passed = $taskPassed; exit_code = $taskExit; error_codes = $taskCodes }
    if (-not $taskPassed) { throw "Lint policy fixture '$($taskCase.Name)' failed; inspect $taskDirectory/diagnostics.ndjson" }
    Write-Host "Lint policy: $($taskCase.Name) passed."
}
[ordered]@{ version = 1; passed = $true; manifest = $taskManifest; cases = $taskResults } | ConvertTo-Json -Depth 7 | Set-Content -LiteralPath (Join-Path $taskRunRoot 'acceptance.json') -Encoding utf8
Write-Host "All $($taskResults.Count) lint policy cases passed. Receipt: $taskRunRoot/acceptance.json"
# Propagate success after expected negative Cargo invocations.
exit 0
