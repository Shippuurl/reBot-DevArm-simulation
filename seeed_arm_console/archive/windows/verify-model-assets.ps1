param(
    [string]$Root = "assets/robot"
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$assetRoot = (Resolve-Path (Join-Path $projectRoot $Root)).Path
$manifests = Get-ChildItem -LiteralPath $assetRoot -Recurse -Filter model*.json -File
if ($manifests.Count -eq 0) {
    throw "未找到 Rerun 模型清单: $assetRoot"
}

foreach ($manifestPath in $manifests) {
    $modelRoot = $manifestPath.Directory.Parent.FullName
    $manifest = Get-Content -LiteralPath $manifestPath.FullName -Raw | ConvertFrom-Json
    if ($manifest.schema_version -ne 1) {
        throw "不支持的清单版本 $($manifest.schema_version): $($manifestPath.FullName)"
    }
    if ($manifest.visuals.Count -eq 0) {
        throw "清单没有 visual mesh: $($manifestPath.FullName)"
    }
    foreach ($visual in $manifest.visuals) {
        $relative = [string]$visual.mesh
        $modelRootFull = [System.IO.Path]::GetFullPath($modelRoot).TrimEnd('\') + '\'
        $meshPath = [System.IO.Path]::GetFullPath((Join-Path $modelRoot $visual.mesh))
        if ([System.IO.Path]::IsPathRooted($relative) -or -not $meshPath.StartsWith($modelRootFull, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "模型路径必须是资源目录内的相对路径: $($visual.mesh)"
        }
        if (-not (Test-Path -LiteralPath $meshPath -PathType Leaf)) {
            throw "模型文件不存在: $meshPath"
        }
        if ($null -ne $visual.albedo_factor -and $visual.albedo_factor.Count -ne 4) {
            throw "albedo_factor 必须包含 RGBA 四个字节: $($manifestPath.FullName)"
        }
    }
    Write-Host ("model=OK path={0} visuals={1}" -f $manifestPath.FullName, $manifest.visuals.Count)
}
