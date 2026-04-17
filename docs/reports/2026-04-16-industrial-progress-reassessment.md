# 工业级进度复评报告（2026-04-16，第二轮更新）

## 1. 评估范围与证据命令

本次为“补齐实施后”的第二轮复评，聚焦以下三类证据：

1. 分包策略有效性（单测 + 构建告警口径）
2. 工程门禁健康度（`accept:web`、`repo:verify`、后端 gate）
3. 工业化评估脚本可重复性（JSON 产出 + 自测）

本地执行命令：

1. `pnpm --filter erp-web test:unit -- src/build/manualChunks.test.ts`（通过）
2. `pnpm --filter erp-web build`（通过，见告警明细）
3. `node --test scripts/ops/industrial_progress_assessment.test.mjs`（通过）
4. `pnpm run assess:industrial-progress`（通过）
5. `pnpm run accept:web`（通过）
6. `pnpm run repo:verify`（通过）
7. `pnpm run test:backend:release-gate`（失败，数据库环境阻塞）

## 2. 实施前 vs 实施后（关键指标）

| 指标 | 实施前 | 实施后（第二轮） | 变化 | 说明 |
| --- | --- | --- | --- | --- |
| `manualChunks` 单测通过率 | `2/10` | `10/10` | `+8` 用例 | 分包规则与测试口径完全对齐 |
| `sales/shop-inventory-sync` 占位依赖 | 有（占位模块导出） | 无（切换到 `InventorySyncJobsPage`） | 改善 | 页面从“占位可见”转为“可执行入口” |
| placeholder 路由引用计数（router） | `1` | `0` | `-1` | 见 `industrial-progress-latest.json` |
| `page-orders-*` 循环 chunk 告警 | 存在 | 已消除 | 风险下降 | 订单域由“细粒度高耦合拆分”回收为“query/detail + page-orders” |
| `accept:web`（lint+typecheck+build） | 通过 | 通过 | 持平 | 无回归 |
| `repo:verify`（含 OpenAPI gate） | 通过 | 通过 | 持平 | 基线健康 |
| 后端 release-gate | 失败（DB 未起） | 失败（DB 未起） | 持平 | 环境阻塞，非代码回归 |
| 工业评估自动化 | 无统一脚本 | 新增脚本 + 测试 + npm 命令 | 新增能力 | 可重复生成 JSON 评估证据 |

## 3. 本轮 5 子任务完成度（5/5）

1. 前端分包治理与稳定化
- 文件：`apps/erp-web/src/build/manualChunks.ts`、`apps/erp-web/src/build/manualChunks.test.ts`
- 结果：保留 `page-orders-query`、`page-orders-detail`，其余订单高耦合模块归并 `page-orders`，去除循环告警。

2. 销售库存同步路由去占位
- 文件：`apps/erp-web/src/app/router.tsx`
- 结果：`sales/shop-inventory-sync` 改为加载 `InventorySyncJobsPage`。

3. 工业级进度评估脚本化
- 文件：`scripts/assess_industrial_progress.mjs`、`scripts/ops/industrial_progress_assessment.test.mjs`、`package.json`
- 结果：`pnpm run assess:industrial-progress` 可直接输出 `docs/reports/_generated/industrial-progress-latest.json`。

4. Codex 多子任务补全文档（上限 5 个）
- 文件：`docs/superpowers/plans/2026-04-16-codex-industrial-gap-closure-fastlane.md`
- 结果：固化子任务边界、验收命令与并行执行口径。

5. 实施后复评闭环
- 文件：本报告
- 结果：形成“命令证据 -> 指标变化 -> 风险剩余 -> 下一轮动作”的完整追踪链路。

## 4. 当前成熟度重估（增量口径）

> 注：这是在 2026-04-13 总评基础上的增量修正，不替代全域重评。

| 维度 | 实施前 | 实施后（第二轮） | 变化 |
| --- | --- | --- | --- |
| 工程治理（门禁/可验证性） | 3.5 / 5 | 3.9 / 5 | +0.4 |
| 前端可交付性（分包/路由可执行） | 2.9 / 5 | 3.4 / 5 | +0.5 |
| 运行风险控制（告警可解释性） | 3.0 / 5 | 3.2 / 5 | +0.2 |
| 综合工业化成熟度（增量） | 43 / 100（参考 2026-04-13） | 48 / 100 | +5 |

## 5. 当前阻塞与剩余风险

1. 后端 release-gate 仍被数据库环境阻塞（`localhost:5432` 连接拒绝），无法给出后端链路全绿证据。
2. 前端构建仍有两类告警待治理：
- `Forbidden.tsx` 的静态 + 动态双重导入告警（不会进入独立 chunk）。
- `vendor-ui` 与 `vendor-grid-core` 大体积 chunk 告警（>500kB）。
3. 本轮主要修复工程交付链路，不涉及财务、组织、多商户等结构性业务缺口。

## 6. 下一轮建议（按收益优先）

1. 治理 `Forbidden.tsx` 混合导入路径，统一为静态或动态入口，减少无效分包意图。
2. 继续分解 `vendor-ui` / `vendor-grid-core`，结合路由级按需加载降低首屏负担。
3. 启动本地或 CI 数据库后重跑 `pnpm run test:backend:release-gate`，补齐发布级后端证据。
4. 将 `assess:industrial-progress` 产物归档进 CI，建立周度趋势追踪。
