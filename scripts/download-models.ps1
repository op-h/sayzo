<#
.SYNOPSIS
    Downloads the offline voice models Sayzo needs into src-tauri/models.

.DESCRIPTION
    The models (~490 MB) are too large for the git repository, so they are fetched
    from the sherpa-onnx model releases and arranged the way the app expects:

      src-tauri/models/
        espeak-ng-data/          phoneme data shared by all voices
        kokoro/                  English (Kokoro-82M v1.0, fp32)
        ar-miro/  ar-dii/        Arabic (Piper)

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts/download-models.ps1
#>
[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$base = 'https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models'
$root = Split-Path -Parent $PSScriptRoot
$models = Join-Path $root 'src-tauri/models'
$work = Join-Path ([IO.Path]::GetTempPath()) 'sayzo-models'

if ((Test-Path (Join-Path $models 'kokoro/model.onnx')) -and -not $Force) {
    Write-Host "Models already present in $models (use -Force to re-download)."
    exit 0
}

New-Item -ItemType Directory -Force -Path $models, $work | Out-Null

function Get-Archive([string]$name) {
    $archive = Join-Path $work "$name.tar.bz2"
    if (-not (Test-Path $archive)) {
        Write-Host "Downloading $name..."
        Invoke-WebRequest -Uri "$base/$name.tar.bz2" -OutFile $archive
    }
    Write-Host "Extracting $name..."
    tar -xjf $archive -C $work
    if ($LASTEXITCODE -ne 0) { throw "Failed to extract $archive" }
    Join-Path $work $name
}

# English: Kokoro v1.0 (also provides espeak-ng-data and the lexicons)
$kokoro = Get-Archive 'kokoro-multi-lang-v1_0'
$dest = Join-Path $models 'kokoro'
New-Item -ItemType Directory -Force -Path $dest | Out-Null
foreach ($f in 'model.onnx', 'voices.bin', 'tokens.txt', 'lexicon-us-en.txt', 'lexicon-gb-en.txt', 'LICENSE') {
    Copy-Item (Join-Path $kokoro $f) $dest -Force
}
$espeak = Join-Path $models 'espeak-ng-data'
if (Test-Path $espeak) { Remove-Item $espeak -Recurse -Force }
Copy-Item (Join-Path $kokoro 'espeak-ng-data') $espeak -Recurse

# Arabic: Piper voices
$arabic = @{
    'ar-miro' = 'vits-piper-ar_JO-SA_miro_V2-high'
    'ar-dii'  = 'vits-piper-ar_JO-SA_dii-high'
}
foreach ($key in $arabic.Keys) {
    $dir = Get-Archive $arabic[$key]
    $dest = Join-Path $models $key
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    $onnx = Get-ChildItem $dir -Filter '*.onnx' | Select-Object -First 1
    Copy-Item $onnx.FullName (Join-Path $dest 'model.onnx') -Force
    Copy-Item (Join-Path $dir 'tokens.txt') $dest -Force
}

Remove-Item $work -Recurse -Force
$size = (Get-ChildItem $models -Recurse -File | Measure-Object Length -Sum).Sum / 1MB
Write-Host ("Done. {0:N0} MB of models in {1}" -f $size, $models)
