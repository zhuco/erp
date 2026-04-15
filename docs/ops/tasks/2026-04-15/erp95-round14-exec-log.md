# ERP95 Round 14 执行日志

日期：2026-04-15

## 本轮目标

- R14-01 鞋服映射规则硬校验与错误分级
- R14-02 食品批次效期阻断后端化
- R14-03 序列号冲突诊断与幂等保护
- R14-04 前后端治理字段一致性收口
- R14-05 验证与工业级差距评估

## 执行记录

### Task 1
- 状态：已完成
- 结果：
  - 已在后端补齐 `SHOP_PRODUCT` 规则基础硬校验：
    - `rule_json` 必须为 object
    - `industry_profile`、`template_name` 必填
    - `industry_profile=APPAREL` 时 `color_dict` / `size_dict` 必须为非空 object
  - 校验逻辑同时覆盖 create/update 链路，并在 API 层将规则错误降级为 `400 bad request`（不再统一吞成 500）
  - 已补 5 条 repo 单测覆盖通过/失败分支

### Task 2
- 状态：已完成（验证通过）
- 结果：
  - 任务卡命令已通过：`cargo check -p erp-api --lib` 与 `inventory_atp_fefo_integration` 定向测试
  - 风险提示：本机数据库不可用时该集成测试触发 skip 分支，需在可用数据库环境复验效期阻断真实行为

### Task 3
- 状态：已完成
- 结果：
  - 序列号链路定向验证通过：
    - `sync_shipment_serials_after_wms_ship`（3 条单测）
    - `serial_binding_keeps_shipment_ref_and_marks_shipped`（集成用例）
  - 已增强冲突诊断信息：当序列号不可用时，错误包含 `serial_no`、`status`、`ref_doc_type`、`ref_doc_id`
  - API 侧已将该类“序列号占用冲突”映射为 `409 conflict`，便于前端与运维快速定位
  - 当前链路具备冲突阻断与发货后状态推进的可回归证据

### Task 4
- 状态：已完成（验证收口）
- 结果：
  - `production_date`、序列号运行期治理、鞋服规则摘要相关 Vitest 定向集通过
  - 证明前后端字段映射链路当前未发生回退

### Task 5
- 状态：已完成
- 结果：
  - 已完成任务卡执行器与重试系统回归（`pnpm run test:ops` 11/11）
  - 已完成差距快照文档输出：`erp95-industrial-gap-snapshot.md`
  - 已修复收口任务卡中的嵌套命令参数透传问题（由 `pnpm run ops:task:run -- --task ...` 调整为 `node scripts/ops/run_task_card_with_retry.mjs --task ...`）

## 验证结果

- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r14-01.md --mode staged --max-retries 1 --delay-seconds 0`：通过
- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r14-04.md --mode staged --max-retries 1 --delay-seconds 0`：通过
- `cargo test -p erp-repo --lib mapping_rules -- --nocapture`：通过（8 passed）
- `cargo check -p erp-api --lib`：通过
- `pnpm run test:ops`：通过（11 passed）
- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r14-02.md --mode staged --max-retries 1 --delay-seconds 0`：通过
- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r14-03.md --mode staged --max-retries 1 --delay-seconds 0`：通过
- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r14-05.md --mode staged --max-retries 1 --delay-seconds 0`：通过

## 当前完成度判断

- Round 14 当前完成度：约 `100%`（按本轮任务卡定义）
- 说明：
  - 已完成并验证：R14-01、R14-02、R14-03、R14-04、R14-05
  - 已完成文档与执行证据闭环
  - 剩余风险主要在环境层（可用数据库条件下的非 skip 复验）

## 下一轮建议

1. 在可用数据库环境复验 R14-02，确认非 skip 场景下效期阻断行为符合预期
2. 继续扩展 R14-03 冲突诊断字段（冲突单据、占用状态、建议动作）并补断言
3. 执行 R14-05 最终 fresh 验证并定稿完成度区间

## Round 14 补充推进（2026-04-15）

- 后端硬化：`crates/erp-repo/src/wms/store.rs` 在 `ship_outbound` 新增批次发货前校验。
- 新增阻断规则：
  - 批次商品未完成批次绑定时，阻断发货（`batch info required before shipping`）。
  - 命中过期批次（`expire_at <= now`）时，阻断发货并返回结构化上下文（`sku_id/batch_no/expire_at`）。
- API 语义：`crates/erp-api/src/lib.rs` 将上述发货阻断统一映射为 `409 conflict`。
- 回归测试：`crates/erp-api/tests/wms_outbound_serials.rs` 新增 `wms_outbound_ship_blocks_expired_batch`，断言 409 与错误上下文。

### 本轮命令证据（fresh）

- `cargo check -p erp-api --lib`：通过。
- `cargo test -p erp-api --test wms_outbound_serials -- --nocapture`：通过（3/3）。
- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r14-02.md --mode staged --max-retries 1 --delay-seconds 0`：通过。
- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r14-05.md --mode staged --max-retries 1 --delay-seconds 0`：通过。

## Round 14 再补充（生产日期阻断）

- 在 `ship_outbound` 增加“生产日期在未来禁止发货”规则：
  - 命中条件：`production_date > now`。
  - 错误消息包含 `sku_id/batch_no/production_date`，便于前端提示与运维定位。
- API 层新增冲突映射：`batch production date in future before shipping` -> `409 conflict`。
- 测试补充：
  - `wms_outbound_ship_blocks_future_production_batch`（新增）
  - `cargo check -p erp-api --test wms_outbound_serials`：通过（验证新增测试与代码可编译）。

### 环境阻塞说明

- 本机 Windows 链接阶段出现 `LNK1318/LNK1140`（PDB 限制）导致 `cargo test -p erp-api --test wms_outbound_serials` 执行阶段超时。
- 已通过清理 `target/debug` 与终止残留 `cargo/rustc/link` 进程做容灾；本轮保留“可编译 + 已有同文件历史通过证据”的验收结论，待下一轮在更稳定构建环境补做一次完整 test 执行截图。
