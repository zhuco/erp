# 上线阻塞清单（平台联调 + 内部闭环）
> 基于当前代码与文档核验的上线门槛清单。数据来源：`docs/reports/archive/feature_gap_matrix.md`、`docs/reports/archive/task_list.md`、`docs/ops/*` 与核心代码路由。更新日期：2026-01-17。

## 已核验完成（非阻塞）
- 账号资料/密码修改：`/me`、`/me/password` + `apps/erp-web/src/pages/system/ProfilePage.tsx`
- 权限种子/菜单入口：`docs/dev/seed.sql` + `apps/erp-web/src/app/layout/menu.tsx`
- 导出任务闭环：`/exports/jobs` + `crates/erp-worker/src/jobs/export_jobs.rs` + `/exports`
- 可观测性面板入口：`/metrics` + `/dead-letters` + `/outbox-events` + `/system/observability`

## P0 阻塞项（上线前必须补齐）
| 分类 | 阻塞项 | 现状 | 主要影响 | 证据 |
| --- | --- | --- | --- | --- |
| 平台联调 | 真实平台数据接入（OAuth/验签/拉单/回传/库存回写） | 未完成验证 | 无法用真实订单进行全链路验收 | `docs/reports/archive/platform_integration_report.md` |
| OMS | 审单规则命中 → 异常池处理闭环 | 部分 | 异常订单无法形成可执行闭环 | 功能矩阵 OMS 审单/异常池 |
| OMS | 多包裹出库与包裹明细闭环 | 部分 | 发货单与包裹明细无法稳定落库 | 功能矩阵 OMS 多包裹 |
| WMS | 波次规则/打包员登记闭环 | 缺口任务 | 出库效率与责任追踪受限 | `docs/reports/archive/task_list.md` WMS-002/WMS-003 |
| 售后 | 换货/补发流程 | 部分 | 售后单无法覆盖换货/补发场景 | 功能矩阵 售后换货/补发 |
| 采购财务 | AP 付款/关单流程 | 已补齐 | 应付单完成结算闭环 | `docs/codex/tasks/PROC-001.md` |
| 财务对账 | 账单导入 + 支付对账自动化 | 已补齐 | 自动对账输出差异明细 | `docs/codex/tasks/FIN-001.md` |
| 导出落地 | 导出文件存储/过期策略 | 已补齐 | 导出文件可落地并清理 | `docs/ops/DATA_RETENTION.md` |
| 运维告警 | 告警阈值与通知编排 | 已补齐 | 告警规则与通知编排落地 | `docs/ops/OBSERVABILITY.md` |
| 可靠性门禁 | 乱序/幂等/保留策略验证 | 已补齐 | 门禁清单与验证步骤补齐 | `docs/ops/BACKEND_READINESS_CHECKLIST.md` |

## 里程碑与排期（建议）
> 说明：以 T0 为范围/资源确认日，默认 4 周排期；若平台联调不在本次范围，可跳过 M3。

| 里程碑 | 周期 | 交付内容 | 验收输出 |
| --- | --- | --- | --- |
| M0 准备 | T0~T0+2 天 | 环境/数据/导出存储策略落地；测试账号与权限种子；门禁脚本基线 | `scripts/progress_check.sh` 通过；导出存储路径可用 |
| M1 内部闭环Ⅰ | 第 1 周 | 审单/异常池闭环（OMS-001）；多包裹明细（OMS-002）；波次/打包员（WMS-002/003）；售后换货/补发 | 订单→审单→出库→发货→售后链路可跑通 |
| M2 内部闭环Ⅱ | 第 2 周 | AP 付款/关单（PROC-001）；账单导入+支付对账自动化（FIN-001）；导出过期策略；告警阈值编排；可靠性门禁 | 财务对账闭环可验收；门禁检查项可跑通 |
| M3 平台联调 | 第 3 周 | OAuth/验签/拉单/回传/库存回写；真实订单 E2E | 真实订单全链路验收报告 |
| M4 预发布 | 第 4 周 | 回归/演练/Go-No-Go | `docs/ops/GO_NO_GO_CHECKLIST.md` 完成 |

## P1 建议（上线后 1~4 周内）
- 规划/预测与统计看板：当前为占位页（`apps/erp-web/src/pages/planning/*`、`/stats/*`）
- 系统扩展配置：工作流/自定义字段/集成中心为静态配置（`apps/erp-web/src/pages/system/*`）
