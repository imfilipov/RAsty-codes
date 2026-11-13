param(
    [string]$Octave = 'octave-cli',
    [string]$RustExe = (Join-Path $PSScriptRoot '..\target\release\ra-reconcile.exe'),
    [switch]$IncludeDvb
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
$small = (& $Octave --quiet --no-gui --eval "run('$(Join-Path $octaveDir 'compare_small.m')')" 2>$null | Out-String)
if ($LASTEXITCODE -ne 0) { throw "Octave small reference failed:`n$small" }
$rustEncode = (& $RustExe encode --matrix (Join-Path $root 'data\H_small.edges') --message 10 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0) { throw "Rust encode failed:`n$rustEncode" }
$rustDv = (& $RustExe decode --matrix (Join-Path $root 'data\H_small.edges') --alice 0110 --message 1000 --p 0.1 --max-iter 10 --threads 1 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0) { throw "Rust DV decode failed:`n$rustDv" }
$rustCv = (& $RustExe decode-cv --matrix (Join-Path $root 'data\H_small.edges') --x (Join-Path $root 'data\x_small.txt') --soft (Join-Path $root 'data\soft_small.txt') --snr-db 0 --max-iter 10 --threads 1 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0) { throw "Rust CV decode failed:`n$rustCv" }
if ($small -notmatch 'SMALL_DV estimate=0110 success=1' -or $small -notmatch 'SMALL_CV estimate=0110' -or $small -notmatch 'SMALL_ENCODE codeword=1011') { throw "Unexpected Octave reference output:`n$small" }
if ($rustEncode -notmatch 'codeword=1011' -or $rustDv -notmatch 'estimate=0110' -or $rustCv -notmatch 'estimate=0110') { throw "Small Rust and Octave outputs differ.`nOctave:`n$small`nRust encode:`n$rustEncode`nRust DV:`n$rustDv`nRust CV:`n$rustCv" }
Write-Output 'PASS: small RA encoder, DV decoder, and CV decoder match Octave.'

$oldDvbFlag = $env:RA_TEST_DVB
if ($IncludeDvb) { $env:RA_TEST_DVB = '1' }
$octLarge = (& $Octave --quiet --no-gui --eval "run('$(Join-Path $octaveDir 'compare_large.m')')" 2>$null | Out-String)
$env:RA_TEST_DVB = $oldDvbFlag
if ($LASTEXITCODE -ne 0) { throw "Octave large reference failed:`n$octLarge" }
$cases = @(@{Name='RA_n6000_R150.edges'; Label='RA6K'; Alice='alice_ra6k.txt'; Message='message_ra6k.txt'; P='0.01'})
if ($IncludeDvb) { $cases += @{Name='DVB_S2_110.edges'; Label='DVB'; Alice='alice_dvb.txt'; Message='message_dvb.txt'; P='0.08'} }
foreach ($item in $cases) {
    $name = $item.Name
    $case = $item.Label
    $line = ($octLarge -split "`r?`n" | Where-Object { $_ -match "^$case " } | Select-Object -Last 1)
    if (-not $line) { throw "Missing $case summary in Octave output.`n$octLarge" }
    $matrix = Join-Path $root "data\$name"
    $alice = $item.Alice
    $message = $item.Message
    $p = $item.P
    $out = Join-Path $env:TEMP "ra-reconcile-$case-output.txt"
    $rust = (& $RustExe decode --matrix $matrix --alice-file (Join-Path $root "data\$alice") --message-file (Join-Path $root "data\$message") --p $p --max-iter 100 --threads 1 --output $out 2>&1 | Out-String)
    if ($LASTEXITCODE -ne 0) { throw "Rust $case decode failed:`n$rust" }
    $bits = [IO.File]::ReadAllText($out).Trim()
    Remove-Item -LiteralPath $out -ErrorAction SilentlyContinue
    $referenceFile = Join-Path $env:TEMP "ra-rust-$case-octave-estimate.txt"
    $referenceBits = [IO.File]::ReadAllText($referenceFile).Trim()
    Remove-Item -LiteralPath $referenceFile -ErrorAction SilentlyContinue
    if ($rust -notmatch 'success=true' -or $line -notmatch 'success=1' -or $bits -cne $referenceBits) { throw "Rust $case estimate differs from Octave.`nOctave:`n$line`nRust:`n$rust" }
    Write-Output "PASS: $case full estimated bit string and syndrome status match Octave."
}
