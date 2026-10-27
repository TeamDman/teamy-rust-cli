param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ManifestResource {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr LoadLibraryEx(string file, IntPtr reserved, uint flags);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern uint SizeofResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll")]
    public static extern IntPtr LockResource(IntPtr resource);
    [DllImport("kernel32.dll")]
    public static extern bool FreeLibrary(IntPtr module);
}
'@
$path = (Resolve-Path -LiteralPath $Executable).Path
# LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE: never execute the candidate.
$module = [ManifestResource]::LoadLibraryEx($path, [IntPtr]::Zero, 0x22)
if ($module -eq [IntPtr]::Zero) { throw 'Cannot load executable resources' }
try {
    $resource = [ManifestResource]::FindResource($module, [IntPtr]1, [IntPtr]24)
    if ($resource -eq [IntPtr]::Zero) { throw 'Executable lacks RT_MANIFEST #1' }
    $size = [ManifestResource]::SizeofResource($module, $resource)
    $data = [ManifestResource]::LockResource([ManifestResource]::LoadResource($module, $resource))
    if ($data -eq [IntPtr]::Zero -or $size -eq 0) { throw 'Empty manifest resource' }
    $bytes = New-Object byte[] $size
    [Runtime.InteropServices.Marshal]::Copy($data, $bytes, 0, $bytes.Length)
    $text = [Text.Encoding]::UTF8.GetString($bytes).Trim([char]0)
    [xml]$xml = $text
    $namespaces = New-Object Xml.XmlNamespaceManager($xml.NameTable)
    $namespaces.AddNamespace('asm', 'urn:schemas-microsoft-com:asm.v1')
    $namespaces.AddNamespace('app', 'urn:schemas-microsoft-com:asm.v3')
    $namespaces.AddNamespace('dpi', 'http://schemas.microsoft.com/SMI/2016/WindowsSettings')
    $namespaces.AddNamespace('legacy', 'http://schemas.microsoft.com/SMI/2005/WindowsSettings')
    $settings = '/asm:assembly/app:application/app:windowsSettings'
    $modern = $xml.SelectSingleNode("$settings/dpi:dpiAwareness", $namespaces)
    $legacy = $xml.SelectSingleNode("$settings/legacy:dpiAware", $namespaces)
    if ($null -eq $modern -or $modern.InnerText -ne 'PerMonitorV2,PerMonitor') { throw 'Missing native DPI awareness/fallback list' }
    if ($null -eq $legacy -or $legacy.InnerText -ne 'true/pm') { throw 'Missing legacy DPI fallback' }
    $controls = $xml.SelectSingleNode('/asm:assembly/asm:dependency/asm:dependentAssembly/asm:assemblyIdentity[@name="Microsoft.Windows.Common-Controls"]', $namespaces)
    if ($null -eq $controls -or $controls.version -ne '6.0.0.0') { throw 'Missing Common Controls v6' }
    if ($OutputPath) { [IO.File]::WriteAllText($OutputPath, $text) }
    Write-Output "Verified RT_MANIFEST #1: PerMonitorV2,PerMonitor; true/pm; Common Controls v6 ($path)"
} finally { [void][ManifestResource]::FreeLibrary($module) }
