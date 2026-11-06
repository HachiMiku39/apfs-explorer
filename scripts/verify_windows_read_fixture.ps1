<#
Read and copy a manifest created by make_macos_read_fixture.py.
MountedRoot is the fixture directory on the mounted APFS volume. CopyRoot and
ReportPath must be on a different local drive. No writes are made to MountedRoot.
Requires Windows PowerShell 5.1 or PowerShell 7 on Windows.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$MountedRoot,
    [Parameter(Mandatory=$true)][string]$ManifestPath,
    [Parameter(Mandatory=$true)][string]$CopyRoot,
    [Parameter(Mandatory=$true)][string]$ReportPath
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Get-LocalPath([string]$Path) {
    if ($Path -notmatch '^[A-Za-z]:[\\/]') { throw 'Use an absolute local drive path' }
    return [IO.Path]::GetFullPath($Path).TrimEnd('\')
}
function Join-FixturePath([string]$Root, [string]$Relative) {
    if ([string]::IsNullOrEmpty($Relative) -or $Relative -match '[:\\]' -or
        $Relative.StartsWith('/') -or @($Relative.Split('/') | Where-Object { $_ -in @('', '.', '..') }).Count) {
        throw "Unsafe manifest path: $Relative"
    }
    return $Root + '\' + $Relative.Replace('/', '\')
}
$MountedRoot = Get-LocalPath $MountedRoot
$CopyRoot = Get-LocalPath $CopyRoot
$ReportPath = Get-LocalPath $ReportPath
$sourceDrive = [IO.Path]::GetPathRoot($MountedRoot)
foreach ($path in @($CopyRoot, $ReportPath)) {
    if ([IO.Path]::GetPathRoot($path) -ieq $sourceDrive) { throw 'Output must be on a different drive from APFS' }
}
if (Test-Path -LiteralPath $CopyRoot) { throw 'CopyRoot must be a new directory; existing files are never overwritten' }
$manifest = Get-Content -LiteralPath $ManifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($manifest.schema -ne 1) { throw 'Unsupported manifest schema' }

# CreateFileW handles extended paths independently of PowerShell's legacy path
# provider. All source handles use GENERIC_READ and OPEN_EXISTING exclusively.
if (-not ('OpenAPFS.ReadFixtureIO' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using Microsoft.Win32.SafeHandles;
namespace OpenAPFS {
    public static class ReadFixtureIO {
        [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
        static extern SafeFileHandle CreateFileW(string name, uint access, uint share,
            IntPtr security, uint creation, uint flags, IntPtr template);
        [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
        static extern bool CreateDirectoryW(string name, IntPtr security);
        public static void MakeDirectories(string path) {
            string full = path;
            string current = full.Substring(0, 2);
            foreach (string part in full.Substring(3).Split('\\')) {
                if (part.Length == 0) continue;
                current += "\\" + part;
                if (!CreateDirectoryW(@"\\?\" + current, IntPtr.Zero)) {
                    int error = Marshal.GetLastWin32Error();
                    if (error != 183) throw new Win32Exception(error, current);
                }
            }
        }
        static FileStream Open(string path, bool write) {
            SafeFileHandle handle = CreateFileW(@"\\?\" + path,
                write ? 0x40000000u : 0x80000000u, 1, IntPtr.Zero,
                write ? 1u : 3u, 0x08000000u, IntPtr.Zero);
            if (handle.IsInvalid) {
                int error = Marshal.GetLastWin32Error();
                handle.Dispose();
                throw new Win32Exception(error, path);
            }
            try { return new FileStream(handle, write ? FileAccess.Write : FileAccess.Read, 1048576, false); }
            catch { handle.Dispose(); throw; }
        }
        public static string HashAndCopy(string source, string destination, long expectedSize) {
            if (String.IsNullOrEmpty(destination)) destination = null;
            if (destination != null) MakeDirectories(destination.Substring(0, destination.LastIndexOf('\\')));
            using (FileStream input = Open(source, false))
            using (FileStream output = destination == null ? null : Open(destination, true))
            using (SHA256 hash = SHA256.Create()) {
                if (input.Length != expectedSize) throw new IOException("Unexpected source size: " + input.Length);
                byte[] buffer = new byte[1048576];
                long total = 0;
                int count;
                while ((count = input.Read(buffer, 0, buffer.Length)) != 0) {
                    hash.TransformBlock(buffer, 0, count, buffer, 0);
                    if (output != null) output.Write(buffer, 0, count);
                    total = checked(total + count);
                }
                hash.TransformFinalBlock(new byte[0], 0, 0);
                if (total != expectedSize) throw new IOException("Unexpected number of bytes read: " + total);
                if (output != null) output.Flush(true);
                return BitConverter.ToString(hash.Hash).Replace("-", "").ToLowerInvariant();
            }
        }
    }
}
'@
}

$results = [Collections.Generic.List[object]]::new()
$started = [DateTime]::UtcNow.ToString('o')
$failed = 0
foreach ($file in $manifest.files) {
    $entry = [ordered]@{path=$file.path; size=[long]$file.size; passed=$false; error=$null}
    try {
        $source = Join-FixturePath $MountedRoot $file.path
        $destination = Join-FixturePath $CopyRoot $file.path
        $hash = [OpenAPFS.ReadFixtureIO]::HashAndCopy($source, $destination, [long]$file.size)
        if ($hash -cne $file.sha256) { throw 'Source SHA256 mismatch' }
        $copiedHash = [OpenAPFS.ReadFixtureIO]::HashAndCopy($destination, $null, [long]$file.size)
        if ($copiedHash -cne $file.sha256) { throw 'Copied SHA256 mismatch' }
        $entry.passed = $true
    } catch {
        $entry.error = $_.Exception.Message
        $failed++
    }
    $results.Add([pscustomobject]$entry)
}
$casePassed = $false
try {
    $caseFile = @($manifest.files | Where-Object path -CEQ $manifest.case_probe.stored)
    if ($caseFile.Count -ne 1) { throw 'Invalid case probe in manifest' }
    $hash = [OpenAPFS.ReadFixtureIO]::HashAndCopy(
        (Join-FixturePath $MountedRoot $manifest.case_probe.lookup), $null, [long]$caseFile[0].size)
    if ($hash -cne $caseFile[0].sha256) { throw 'Case lookup hash mismatch' }
    $casePassed = $true
} catch { $failed++; $results.Add([pscustomobject]@{path='case_probe';passed=$false;error=$_.Exception.Message}) }

$report = [ordered]@{
    schema=1; fixture=$manifest.fixture; started_utc=$started; finished_utc=[DateTime]::UtcNow.ToString('o')
    regular_files=@($manifest.files).Count; case_lookup_passed=$casePassed; failures=$failed
    # Reading hard-link contents does not prove link identity. Symlink and xattr
    # semantics require separate tests and are deliberately not reported as passed.
    symlink_metadata='not-tested'; hardlink_identity='not-tested'; xattrs='not-tested'
    files=$results.ToArray()
}
[OpenAPFS.ReadFixtureIO]::MakeDirectories([IO.Path]::GetDirectoryName($ReportPath))
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ReportPath -Encoding UTF8
Write-Output "Verified $(@($manifest.files).Count) files; failures: $failed; report: $ReportPath"
if ($failed) { exit 1 }
