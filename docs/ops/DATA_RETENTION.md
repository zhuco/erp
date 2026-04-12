# DATA_RETENTION

## 范围定义
- **在线查询数据**：用于业务/运营查询、排障、二次处理的热数据。
- **归档数据**：不再需要在线检索，但需保留审计/追溯（对象存储/冷库）。
- **可删除数据**：过期后允许物理删除，不参与审计。

## 推荐保留周期（可按业务调优）

| 数据域 | 在线查询 | 归档保留 | 备注 |
| --- | --- | --- | --- |
| `inbox_events` | 30 天 | 90 天 | 已处理/失败事件用于排障与复盘 |
| `inbox_raw_payloads` | 30 天 | 90 天 | 原始 webhook payload，体积大 |
| `outbox_events` | 30 天 | 180 天 | 便于回放、追踪 |
| `print_jobs` | 90 天 | 180 天 | SUCCESS/FAILED 可清理 |
| `export_jobs` | 30 天 | 90 天 | 文件落地在 `EXPORT_BASE_DIR/<tenant_id>/<job_id>.csv` |
| `orders` | 12-24 个月 | 5-7 年 | 财务/合规审计 |
| `after_sale_cases` | 12-24 个月 | 5-7 年 | 售后合规与客诉追踪 |

## 归档 vs 删除边界
- **必须归档保留（审计）**：`orders`、`after_sale_cases`（以及关联财务/履约明细）。
- **可删除**：`inbox_events`、`inbox_raw_payloads`、`outbox_events`、`print_jobs` 等排障类数据，超过在线/归档周期后可物理删除。

## 导出文件存储
- 导出文件由 `erp-worker` 写入 `EXPORT_BASE_DIR/<tenant_id>/<job_id>.csv`（默认 `exports/`）。
- API 下载地址为 `/exports/jobs/{job_id}/file`，`export_jobs.output_url` 同步保存该 URL。
- 生产建议将 `EXPORT_BASE_DIR` 指向持久卷或对象存储挂载目录，避免容器重启丢失。

## 最小清理机制（cron 触发）
已提供独立二进制 `erp-worker` 的 `purge` 入口，可通过 cron 调度。

- 导出任务文件：`export_jobs.output_url` 保存下载地址 `/exports/jobs/{job_id}/file`，
  文件落地在 `EXPORT_BASE_DIR/<tenant_id>/<job_id>.csv`（默认目录 `exports/`）。
- `OUTBOX_RETENTION_DAYS`：已处理或已终止的 outbox 保留天数（默认 30）。
- `INBOX_RETENTION_DAYS`：已处理/失败的 inbox 保留天数（默认 30，raw payload 随之清理）。
- `PRINT_JOBS_RETENTION_DAYS`：SUCCESS/FAILED 的 print_jobs 保留天数（默认 90）。
- `EXPORT_JOBS_RETENTION_DAYS`：DONE/FAILED 的 export_jobs 保留天数（默认 30，文件随之清理）。
- 设置为 `0` 可禁用对应清理项。

示例：
```
cargo run -p erp-worker --bin purge
```

## 备份策略（概览）
- 使用 `pg_dump` 每日全量备份。
- 轮转保留：`BACKUP_RETENTION_DAYS` 控制保留天数。
- 脚本见 `scripts/backup_postgres.sh`。
