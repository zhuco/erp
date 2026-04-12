# PDD-only 模式
> 平台接入隔离能力，仅在启用平台对外接入时参考。

## 作用
- 启用后仅允许 `platform=pdd` 的 webhook、适配器、同步、发货与售后流程。
- 非 PDD 平台的请求会返回 4xx，并记录告警日志。
- 默认关闭。

## 配置方式
### 全局（Env）
- 环境变量：`PDD_ONLY_MODE=true`（`1/true/yes/on` 视为启用，其余值视为关闭）
- 该变量一旦设置，将覆盖租户级配置。

### 租户级（DB）
- 仅在 `PDD_ONLY_MODE` 未设置时生效。
- 字段：`tenants.pdd_only_mode`（默认 `false`）
- 示例：
  - `update tenants set pdd_only_mode = true where tenant_id = '...';`

## 行为说明
- Webhook：非 PDD 平台直接返回 `403`。
- 适配器/同步/发货/售后：非 PDD 平台事件会被拦截并记录日志。

## 迁移
- `migrations/0010_pdd_only_mode.sql`
