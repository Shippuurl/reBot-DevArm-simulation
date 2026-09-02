param(
    [string]$Endpoint = "127.0.0.1:50051",
    [ValidateSet("mock", "mujoco")]
    [string]$ExpectedSource = "mujoco",
    [switch]$CheckControl
)

$ErrorActionPreference = "Stop"

function Read-Frame {
    param([System.IO.StreamReader]$Reader)

    $line = $Reader.ReadLine()
    if ([string]::IsNullOrWhiteSpace($line)) {
        throw "gateway did not return telemetry before timeout"
    }
    try {
        return $line | ConvertFrom-Json
    }
    catch {
        throw "gateway returned invalid JSON: $line"
    }
}

$client = [System.Net.Sockets.TcpClient]::new()
try {
    $client.Connect($Endpoint.Split(':')[0], [int]$Endpoint.Split(':')[1])
    $stream = $client.GetStream()
    $stream.ReadTimeout = 3000
    $stream.WriteTimeout = 3000
    $reader = [System.IO.StreamReader]::new($stream)
    $writer = [System.IO.StreamWriter]::new($stream)
    $writer.AutoFlush = $true

    $frame = Read-Frame $reader
    if ($frame.source -ne $ExpectedSource) {
        throw "source mismatch: expected $ExpectedSource, got $($frame.source)"
    }
    if ($frame.joint_position_rad.Count -ne 6) {
        throw "joint position count is $($frame.joint_position_rad.Count), expected 6"
    }
    if ($frame.tf.Count -lt 10) {
        throw "TF count is $($frame.tf.Count), expected at least 10 (including both gripper fingers)"
    }
    if ($null -eq $frame.actual_trajectory) {
        throw "actual_trajectory field is missing"
    }

    Write-Host ("telemetry=OK source={0} sequence={1} tf={2} actual={3}" -f `
        $frame.source, $frame.sequence, $frame.tf.Count, $frame.actual_trajectory.Count)

    if ($CheckControl) {
        $writer.WriteLine('{"type":"jog","joint_index":0,"step_rad":0.05}')
        $ack = $null
        for ($index = 0; $index -lt 20; $index++) {
            $candidate = Read-Frame $reader
            if ($candidate.type -eq "ack") {
                $ack = $candidate
                break
            }
        }
        if ($null -eq $ack -or $ack.status -ne "accepted") {
            $status = if ($null -eq $ack) { "missing" } else { $ack.status }
            throw "control command was not acknowledged: $status"
        }
        Write-Host "control=OK jog accepted"
    }
}
finally {
    if ($null -ne $client) {
        $client.Dispose()
    }
}
