<# Run only on an explicitly authorized test mount created from our fixture. #>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$MountedRoot,
    [Parameter(Mandatory=$true)][string]$ManifestPath,
    [Parameter(Mandatory=$true)][string]$ReportPath
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$source = Join-Path $MountedRoot 'plain.txt'
if ($MountedRoot -notmatch '^[A-Za-z]:[\\/]' -or $ReportPath -notmatch '^[A-Za-z]:[\\/]') {
    throw 'Use absolute local drive paths'
}
if ([IO.Path]::GetPathRoot($ReportPath) -ieq [IO.Path]::GetPathRoot($MountedRoot)) {
    throw 'Report must be on another drive'
}
$manifest = Get-Content -LiteralPath $ManifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$plain = @($manifest.files | Where-Object path -CEQ 'plain.txt')
if ($manifest.schema -ne 1 -or $plain.Count -ne 1) { throw 'Invalid fixture manifest' }
$expected = $plain[0].sha256
$before = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLower()
if ($before -cne $expected) { throw 'Source does not match the known test fixture' }
$results = [Collections.Generic.List[object]]::new()
$probe = Join-Path $MountedRoot ('OpenAPFS-denial-' + [Guid]::NewGuid().ToString('N') + '.tmp')
foreach ($operation in @('write-open', 'create')) {
    $denied = $false
    $errorCode = $null
    $errorMessage = $null
    try {
        $path = if ($operation -eq 'write-open') { $source } else { $probe }
        $mode = if ($operation -eq 'write-open') { [IO.FileMode]::Open } else { [IO.FileMode]::CreateNew }
        $handle = [IO.File]::Open($path, $mode, [IO.FileAccess]::Write, [IO.FileShare]::ReadWrite)
        # Never issue a content write, even if the filesystem grants the handle.
        $handle.Dispose()
    } catch {
        $exception = $_.Exception
        while ($exception.InnerException) { $exception = $exception.InnerException }
        $errorCode = $exception.HResult -band 0xFFFF
        $errorMessage = $exception.Message
        # ERROR_ACCESS_DENIED or ERROR_WRITE_PROTECT; unrelated I/O failures
        # must not masquerade as successful read-only enforcement.
        $denied = $errorCode -in @(5, 19)
    }
    $results.Add([pscustomobject]@{operation=$operation;denied=$denied;win32_error=$errorCode;message=$errorMessage})
}
$after = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLower()
$unchanged = $before -ceq $expected -and $after -ceq $expected -and -not (Test-Path -LiteralPath $probe)
$passed = $unchanged -and @($results | Where-Object {-not $_.denied}).Count -eq 0
$report = [ordered]@{schema=1;utc=[DateTime]::UtcNow.ToString('o');passed=$passed;source_hash_unchanged=$unchanged;checks=$results.ToArray()}
$report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $ReportPath -Encoding UTF8
$report | ConvertTo-Json -Depth 5
if (-not $passed) { exit 1 }
