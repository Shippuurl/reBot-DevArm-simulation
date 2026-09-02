param(
    [ValidateSet("start", "mujoco-start", "stop", "restart", "logs", "status")]
    [string]$Action = "start"
)

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ComposeFile = Join-Path $ProjectRoot "docker-compose.gateway.yml"
$MujocoComposeFile = Join-Path $ProjectRoot "docker-compose.mujoco.yml"

if (-not (Test-Path -LiteralPath $ComposeFile)) {
    throw "找不到网关 Compose 文件: $ComposeFile"
}

function Wait-ForGateway {
    param([int]$TimeoutSeconds = 60)

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $client = [System.Net.Sockets.TcpClient]::new()
        try {
            $connect = $client.ConnectAsync("127.0.0.1", 50051)
            if ($connect.Wait(500) -and $client.Connected) {
                Write-Host "gateway=ready endpoint=127.0.0.1:50051"
                return
            }
        }
        catch {
            # The container may still be compiling the gateway.
        }
        finally {
            $client.Dispose()
        }
        Start-Sleep -Seconds 1
    }
    throw "gateway did not become ready within $TimeoutSeconds seconds"
}

$ComposeArgs = @("-f", $ComposeFile)
if ($Action -eq "mujoco-start") {
    if (-not (Test-Path -LiteralPath $MujocoComposeFile)) {
        throw "找不到 MuJoCo Compose 文件: $MujocoComposeFile"
    }
    $ComposeArgs += @("-f", $MujocoComposeFile)
}

switch ($Action) {
    "start" {
        docker compose @ComposeArgs up -d
        Wait-ForGateway
    }
    "mujoco-start" {
        docker compose @ComposeArgs up -d --build
        Wait-ForGateway -TimeoutSeconds 180
    }
    "stop" {
        docker compose @ComposeArgs stop
    }
    "restart" {
        docker compose @ComposeArgs restart
    }
    "logs" {
        docker compose @ComposeArgs logs -f --tail 100
    }
    "status" {
        docker compose @ComposeArgs ps
    }
}
