param(
    [string]$Output = "recordings/sample.rrd",
    [ValidateSet("default", "rsproxy")]
    [string]$Registry = "default"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $ProjectRoot

$cargoArgs = @("run", "--features", "rerun-recording", "--bin", "rerun_sample")
if ($Registry -eq "rsproxy") {
    $cargoArgs += @(
        "--config", "source.crates-io.replace-with='rsproxy'",
        "--config", "source.rsproxy.registry='sparse+https://rsproxy.cn/index/'"
    )
}
$cargoArgs += @("--", $Output)
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) {
    throw "Rerun 样例构建或运行失败，退出码: $LASTEXITCODE"
}

if (-not (Test-Path -LiteralPath $Output)) {
    throw "Rerun 记录未生成: $Output"
}

$recording = Get-Item -LiteralPath $Output
Write-Host ("recording=ready path={0} bytes={1}" -f $recording.FullName, $recording.Length)
