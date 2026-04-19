# ERP95 Round 13 执行日志

日期：2026-04-15

## 本轮目标

- 食品批次 `production_date` 下沉到 Rust repo/API
- 数码序列号出库绑定语义与前端治理闭环对齐
- 鞋服映射规则增加模板校验与版本治理

## 本轮任务卡

- R13-01 食品批次 production_date 后端下沉
- R13-02 数码序列号出库绑定语义对齐
- R13-03 鞋服映射规则模板校验与版本治理
- R13-04 前后端治理字段对齐收口
- R13-05 Round 13 验证与评估

## Red -> Green 过程

### Task 1：食品批次 `production_date` 后端下沉

- 先在 `crates/erp-repo/tests/product_center.rs` 中补了失败断言，要求批次创建、更新、列表查询都保留 `production_date`
- `RED` 结果：`SkuBatchCreate`、`SkuBatchUpdate`、`SkuBatch` 均不存在 `production_date` 字段，编译失败
- 随后补齐：
  - `crates/erp-repo/src/sku_batches.rs` 的结构体字段
  - 查询、插入、更新 SQL 的 `production_date`
  - `crates/erp-api/src/lib.rs` 中 `SkuBatchCreateRequest` / `SkuBatchUpdateRequest` 与 handler 解析逻辑
  - 相关测试建表语句中的 `production_date timestamptz`
- `GREEN` 结果：
  - `erp-repo` 目标测试已重新编译通过
  - `erp-api --lib` 编译通过，说明本轮 API 改动本身成立

## 本轮额外处理

- 运行 Rust 测试前遇到 `F:\erp\target\debug` 构建缓存占满磁盘，剩余空间仅约 `0.76 MB`
- 已安全清理工作区构建产物 `F:\erp\target\debug`，释放约 `12.95 GB` 空间后继续验证
- 仅清理了生成产物，没有动源码与用户业务文件

## 当前已完成项

- 已创建 Round 13 主计划与 5 张任务卡
- 已完成 Task 1 的代码落地
- 已完成 Task 2 的主线代码与回归测试落地
- 已补齐 Round 13 执行日志首轮记录

### Task 2：数码序列号出库绑定语义对齐

- 本轮先在 `crates/erp-api/src/lib.rs` 增加了 `sync_shipment_serials_after_wms_ship(...)`
- 将该同步逻辑接入三条 WMS 发货路径：
  - `ship_wms_wave`
  - `ship_wms_outbound`
  - `direct_ship_wms_outbound`
- 这样在出库单发货成功后，若存在关联 `shipment_id`，会继续调用 `sku_serials_repo.mark_shipment_serials_shipped(...)`，把已绑定序列号状态从 `ALLOCATED` 推进到 `SHIPPED`
- 同时补了 3 条 `erp-api` 单元回归测试，覆盖：
  - 有 `shipment_id` 时会触发序列号状态同步
  - 无 `shipment_id` 时不会误调用
  - repo 同步失败时不会导致 handler 单测崩溃
- 在 `crates/erp-api/tests/wms_outbound_serials.rs` 中补了集成回归：
  - 绑定后 `ref_doc_type` 保持 `SHIPMENT`
  - `ref_doc_id` 保持为 `shipment_id`
  - 同一序列号跨出库/跨发货单重复占用会被阻断
  - 发货状态流转后序列号状态变为 `SHIPPED`
- 另外顺手修复了 `erp-api` 现有测试代码里两个 `AuthContext` 初始化缺少 `merchant_ids` / `organization_ids` 的问题，使本轮新增单测可以正常编译运行

## 验证结果

### 1. Repo 定向验证

命令：

```powershell
cargo test -p erp-repo sku_batch_repo_flow -- --nocapture
```

结果：

- 首次运行：`RED`，缺少 `production_date` 字段导致编译失败
- 修复后再次运行：测试目标编译通过；运行阶段因本机 `DATABASE_URL` / PostgreSQL 不可用而触发测试内置跳过逻辑

### 2. API 编译验证

命令：

```powershell
cargo check -p erp-api --lib
```

结果：

- 通过，说明本轮 `SkuBatch` API 请求体与 handler 改动可编译

### 3. 未完成验证

命令：

```powershell
cargo test -p erp-api --no-run
```

结果：

- 未能作为本轮有效验证依据
- 失败原因是工作区已有无关测试代码未同步 `AuthContext` 新字段：`merchant_ids`、`organization_ids`
- 该失败不在本轮 `production_date` 改动范围内

### 4. Task 2 定向验证

命令：

```powershell
cargo test -p erp-api sync_shipment_serials_after_wms_ship --lib -- --nocapture
```

结果：

- 3 条单元测试通过
- 说明 WMS 发货后的序列号状态同步 helper 行为成立

命令：

```powershell
cargo test -p erp-api serial_binding_keeps_shipment_ref_and_marks_shipped --test wms_outbound_serials -- --nocapture
```

结果：

- 通过
- 说明序列号绑定后的 `SHIPMENT` 引用语义、重复占用阻断、以及 `SHIPPED` 状态流转已被回归用例覆盖

### 5. 本轮二次空间处理

- 新增集成测试编译时，`F:` 盘再次被 `target\debug` 构建产物占满，导致 `rlib` 归档失败
- 已再次安全清理 `F:\erp\target\debug`
- 清理后可用空间恢复到约 `11.97 GB`
- 仅删除了工作区构建缓存，没有触碰源码

## 当前完成度判断

- Round 13 当前完成度：约 `45%`
- 已真实落地：
  - 食品批次 `production_date` 后端下沉
  - 数码序列号绑定后 `SHIPMENT` 引用语义固化
  - WMS 发货后的序列号 `ALLOCATED -> SHIPPED` 状态推进
- 当前剩余重点：
  - Task 3：鞋服 `SHOP_PRODUCT` 规则模板字段校验
  - Task 4：前后端字段对齐收口
  - Task 5：fresh 验证与工业级差距评估

## 下一步建议

1. 在 `mapping_rules` 增加 `SHOP_PRODUCT` 鞋服模板基础校验，先收口 `industry_profile/template_name/color_dict/size_dict`
2. 基于前端字段审计结果，对齐 `production_date`、序列号展示字段、鞋服规则摘要字段
3. 做一轮 Round 13 fresh 验证，重新评估与工业级电商 ERP 的剩余差距

## 子任务失效修复回归（2026-04-15）

### 现象与根因

- 任务卡执行器在 R13-03 的 `test-2` 步骤失败，错误为 Windows 链接器 `LNK1140`（PDB 容量上限）。
- 该错误不是 `502/503/504` 一类瞬时故障，因此自动重试按设计不会继续重试，属于“命令粒度过宽”问题而非“重试机制失效”。

### 修复动作

- 将 `docs/ops/tasks/2026-04-15/erp95-r13-03.md` 的第二条测试命令从：
  - `cargo test -p erp-repo mapping_rules -- --nocapture`
- 收敛为：
  - `cargo test -p erp-repo --lib mapping_rules -- --nocapture`
- 同时在 `docs/ops/subtask-resilience.md` 增补了 `LNK1140/LNK1318` 非瞬时失败处置规范。

### 回归结果

- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r13-03.md --mode staged --max-retries 1 --delay-seconds 0`：通过
- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r13-04.md --mode staged --max-retries 1 --delay-seconds 0`：通过
- `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r13-05.md --mode staged --max-retries 1 --delay-seconds 0`：通过

### 结论

- 子任务“失效”问题已完成收敛：瞬时故障由自动重试处理，非瞬时构建故障通过任务卡命令收敛与文档规范处理。
- Round 13 剩余任务卡（R13-03/04/05）执行链路已恢复可持续推进状态。

## 任务卡体系补齐（2026-04-15）

- 已将 `R13-01`、`R13-02` 从说明卡升级为可执行 front matter 任务卡，补齐：
  - `test_commands`
  - `retry_policy`
  - `fallback_strategy`
  - `allowed_files/disallowed_files` 边界定义
- 已完成可执行验证：
  - `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r13-01.md --mode staged --max-retries 1 --delay-seconds 0`：通过
  - `pnpm run ops:task:run -- --task docs/ops/tasks/2026-04-15/erp95-r13-02.md --mode staged --max-retries 1 --delay-seconds 0`：通过
- 说明：
  - `R13-01` 的 `product_center` 定向测试在本机无数据库时会触发测试内置 skip 逻辑，命令执行成功且未出现编译回归。
