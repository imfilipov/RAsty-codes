param(
    [string]$Octave = 'octave-cli',
    [string]$RustExe = (Join-Path $PSScriptRoot '..\target\release\ra-reconcile.exe'),
    [int]$Runs = 5
)
$ErrorActionPreference = 'Continue'
if ($Octave -eq 'octave-cli') {
    $found = Get-Command octave-cli -ErrorAction SilentlyContinue
    if ($found) { $Octave = $found.Source }
    else {
        $installed = 'C:\Program Files\GNU Octave\Octave-11.3.0\mingw64\bin\octave-cli.exe'
        if (Test-Path -LiteralPath $installed) { $Octave = $installed }
        else { throw 'GNU Octave was not found. Install Octave or pass -Octave <path>.' }
    }
}
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$octaveDir = Join-Path $root 'tests\octave_reference'
$octave = (& $Octave --quiet --no-gui --eval "run('$(Join-Path $octaveDir 'benchmark_ra6k.m')')" 2>$null | Out-String)
if ($LASTEXITCODE -ne 0) { throw "Octave benchmark failed:`n$octave" }
Write-Output $octave.Trim()
$matrix = Join-Path $root 'data\RA_n6000_R150.edges'
$alice = Join-Path $root 'data\alice_ra6k.txt'
$message = Join-Path $root 'data\message_ra6k.txt'
$output = Join-Path $env:TEMP 'ra-rust-benchmark-estimate.txt'
$threadCounts = @(1, 2, 4, [Environment]::ProcessorCount) | Select-Object -Unique
foreach ($threads in $threadCounts) {
    $times = @()
    for ($run = 0; $run -lt $Runs; $run++) {
        $summary = (& $RustExe decode --matrix $matrix --alice-file $alice --message-file $message --p 0.01 --max-iter 100 --threads $threads --output $output 2>&1 | Out-String)
        if ($LASTEXITCODE -ne 0) { throw "Rust decode failed:`n$summary" }
        if ($summary -notmatch 'elapsed_ms=([0-9.]+)') { throw "Rust timing missing:`n$summary" }
        $times += [double]::Parse($Matches[1], [Globalization.CultureInfo]::InvariantCulture)
    }
    $ordered = @($times | Sort-Object)
    $median = if ($ordered.Count % 2) { $ordered[[int]($ordered.Count / 2)] } else { ($ordered[$ordered.Count / 2 - 1] + $ordered[$ordered.Count / 2]) / 2 }
    Write-Output ("RUST_RA6K threads={0} median_ms={1:N3} runs={2}" -f $threads, $median, $Runs)
}
$batchA = Join-Path $root 'data\alice_ra6k_batch.txt'
$batchM = Join-Path $root 'data\message_ra6k_batch.txt'
foreach ($threads in @(1, 2, 4)) {
    $times = @()
    for ($run = 0; $run -lt $Runs; $run++) {
        $summary = (& $RustExe decode-batch --matrix $matrix --alice-file $batchA --message-file $batchM --p 0.01 --max-iter 100 --threads $threads --output $output 2>&1 | Out-String)
        if ($LASTEXITCODE -ne 0) { throw "Rust batch decode failed:`n$summary" }
        if ($summary -notmatch 'elapsed_ms=([0-9.]+)') { throw "Batch timing missing:`n$summary" }
        $times += [double]::Parse($Matches[1], [Globalization.CultureInfo]::InvariantCulture)
    }
    $ordered = @($times | Sort-Object)
    $median = if ($ordered.Count % 2) { $ordered[[int]($ordered.Count / 2)] } else { ($ordered[$ordered.Count / 2 - 1] + $ordered[$ordered.Count / 2]) / 2 }
    Write-Output ("RUST_BATCH frames=4 threads={0} median_ms={1:N3} frames_per_sec={2:N2}" -f $threads, $median, (4000 / $median))
}
Remove-Item -LiteralPath $output -ErrorAction SilentlyContinue
