# 状态机与安全

状态机是控制网关的唯一权威。UI 显示状态，但不能自行把按钮切换当作状态变更。

## 主状态

```text
DISCONNECTED
    │ Connect + handshake
    ▼
CONNECTED_DISABLED ── Enable ──► ENABLED_IDLE
    ▲                              │
    │ Disconnect                   │ Jog / Execute
    │                              ▼
    └──────────────◄──────── MOVING

任意状态 ── driver fault / limit / watchdog ──► FAULT
任意状态 ── hardware E-stop ──────────────────► ESTOP
FAULT ── validated reset ──► CONNECTED_DISABLED
ESTOP ── physical safety procedure ──► CONNECTED_DISABLED
```

## 状态转移条件

| 当前状态 | 事件 | 必要条件 | 目标状态 |
| --- | --- | --- | --- |
| `DISCONNECTED` | `Connect` | 握手、版本和鉴权通过 | `CONNECTED_DISABLED` |
| `CONNECTED_DISABLED` | `Enable` | 急停释放、无故障、限位有效 | `ENABLED_IDLE` |
| `ENABLED_IDLE` | `Jog/Execute` | 命令校验和看门狗有效 | `MOVING` |
| `MOVING` | 完成 | 驱动报告完成 | `ENABLED_IDLE` |
| 任意 | 驱动故障 | 记录故障码 | `FAULT` |
| 任意 | 硬件急停 | 立即停止输出 | `ESTOP` |

## 看门狗

控制网关维护客户端会话和驱动会话两级看门狗。客户端心跳超时拒绝新命令；驱动反馈超时则触发停止流程。停止流程必须是幂等的，重复发送不会导致再次运动。

## 命令校验顺序

1. 会话和权限。
2. 状态机是否允许该命令。
3. 单位、数值范围和时间戳。
4. 关节/笛卡尔限位、速度和加速度。
5. 碰撞、奇异位形和规划约束。
6. 驱动反馈和执行令牌。

校验失败返回稳定错误码，并记录 `command_id`、来源和失败原因。

## 审计事件

连接、使能、停用、Jog、轨迹确认、停止、故障和复位都应生成审计事件。事件同时写入结构化日志和 Rerun，便于复盘“谁在什么时间以什么模式发出了什么请求”。
