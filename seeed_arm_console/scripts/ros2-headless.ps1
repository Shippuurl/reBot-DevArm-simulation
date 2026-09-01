param(
    [ValidateSet('start', 'shell', 'check')]
    [string]$Action = 'check',
    [string]$Container = 'rebot-ros2-jazzy'
)

$ErrorActionPreference = 'Stop'

switch ($Action) {
    'start' {
        docker start $Container
        break
    }
    'shell' {
        docker exec -it $Container bash -lc 'unset DISPLAY WAYLAND_DISPLAY QT_X11_NO_MITSHM LIBGL_ALWAYS_INDIRECT; source /opt/ros/jazzy/setup.bash; exec bash'
        break
    }
    'check' {
        docker exec $Container bash -lc 'unset DISPLAY WAYLAND_DISPLAY QT_X11_NO_MITSHM LIBGL_ALWAYS_INDIRECT; source /opt/ros/jazzy/setup.bash; printf "headless=OK\nros=%s\n" "$ROS_DISTRO"'
        break
    }
}
