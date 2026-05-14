param(
    [string]$ProjectRoot = "generate/project",
    [int]$TimeoutSeconds = 5,
    [string]$LogDir = "generate/run_logs",
    [string[]]$Cases = @(),
    [string]$Cargo = "cargo",
    [switch]$Release
)

$ErrorActionPreference = "Stop"

function Resolve-RepoPath {
    param([string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return (Resolve-Path -LiteralPath $Path).Path
    }

    return (Resolve-Path -LiteralPath (Join-Path (Get-Location) $Path)).Path
}

function Stop-ProcessTree {
    param([int]$ProcessId)

    if ($IsWindows -or $env:OS -eq "Windows_NT") {
        & taskkill.exe /PID $ProcessId /T /F *> $null
        return
    }

    try {
        Stop-Process -Id $ProcessId -Force -ErrorAction Stop
    } catch {
        # Process may have already exited.
    }
}

function Safe-FileName {
    param([string]$Name)
    return ($Name -replace '[\\/:*?"<>|]', '_')
}

$projectRootPath = Resolve-RepoPath $ProjectRoot

if (-not (Test-Path -LiteralPath $LogDir)) {
    New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
}
$logDirPath = Resolve-RepoPath $LogDir

$projects = Get-ChildItem -LiteralPath $projectRootPath -Directory |
    Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName "Cargo.toml") } |
    Sort-Object Name

if ($Cases.Count -gt 0) {
    $caseSet = @{}
    foreach ($case in $Cases) {
        $caseSet[$case] = $true
    }
    $projects = $projects | Where-Object { $caseSet.ContainsKey($_.Name) }
}

if (-not $projects -or $projects.Count -eq 0) {
    Write-Host "No generated Cargo projects found under: $projectRootPath"
    exit 1
}

$summary = @()
$runArgs = @("run", "--quiet")
if ($Release) {
    $runArgs = @("run", "--quiet", "--release")
}

Write-Host "Running generated projects under: $projectRootPath"
Write-Host "Timeout per case: $TimeoutSeconds seconds"
Write-Host "Logs: $logDirPath"
Write-Host ""

$index = 0
foreach ($project in $projects) {
    $index += 1
    $safeName = Safe-FileName $project.Name
    $stdoutPath = Join-Path $logDirPath "$safeName.stdout.log"
    $stderrPath = Join-Path $logDirPath "$safeName.stderr.log"

    if (Test-Path -LiteralPath $stdoutPath) {
        Remove-Item -LiteralPath $stdoutPath -Force
    }
    if (Test-Path -LiteralPath $stderrPath) {
        Remove-Item -LiteralPath $stderrPath -Force
    }

    Write-Host ("[{0}/{1}] {2} ... " -f $index, $projects.Count, $project.Name) -NoNewline

    $startedAt = Get-Date
    $process = Start-Process `
        -FilePath $Cargo `
        -ArgumentList $runArgs `
        -WorkingDirectory $project.FullName `
        -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath `
        -NoNewWindow `
        -PassThru

    $finished = $process.WaitForExit($TimeoutSeconds * 1000)
    $endedAt = Get-Date

    if ($finished) {
        $exitCode = $process.ExitCode
        if ($exitCode -eq 0) {
            $status = "exited"
            Write-Host "EXIT 0"
        } else {
            $status = "failed"
            Write-Host "FAILED $exitCode"
        }
    } else {
        Stop-ProcessTree -ProcessId $process.Id
        $exitCode = $null
        $status = "timeout"
        Write-Host "TIMEOUT, killed"
    }

    $summary += [PSCustomObject]@{
        Case = $project.Name
        Status = $status
        ExitCode = $exitCode
        StartedAt = $startedAt.ToString("yyyy-MM-dd HH:mm:ss")
        EndedAt = $endedAt.ToString("yyyy-MM-dd HH:mm:ss")
        Stdout = $stdoutPath
        Stderr = $stderrPath
    }
}

$summaryPath = Join-Path $logDirPath "summary.csv"
$summary | Export-Csv -LiteralPath $summaryPath -NoTypeInformation -Encoding UTF8

Write-Host ""
Write-Host "Summary:"
$summary | Group-Object Status | Sort-Object Name | ForEach-Object {
    Write-Host ("  {0}: {1}" -f $_.Name, $_.Count)
}
Write-Host "Summary CSV: $summaryPath"

if (($summary | Where-Object { $_.Status -eq "failed" }).Count -gt 0) {
    exit 2
}

exit 0
