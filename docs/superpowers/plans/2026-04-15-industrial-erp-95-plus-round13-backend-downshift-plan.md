# Industrial ERP Backend Downshift Round 13 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将食品批次生产日期、数码序列号出库绑定、鞋服映射规则三条治理链路继续从前端 mock 下沉到 Rust API 与 repo 层，缩小与工业级电商 ERP 95% 的差距。

**Architecture:** 本轮优先补足 `erp-repo + erp-api` 的真实字段、校验与响应语义，让前端现有治理界面可以逐步切换到真实后端而不改交互模型。实现顺序遵循“先补失败测试，再补最小实现，再做接口/文档收口”的 TDD 节奏。

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL, React, Vitest, Playwright

---

## 文件结构与职责

- `crates/erp-repo/src/sku_batches.rs`
  - 食品批次的真实字段与持久化查询、创建、更新
- `crates/erp-repo/src/sku_serials.rs`
  - 数码序列号生成、唯一校验、出库绑定、发货状态流转
- `crates/erp-repo/src/mapping_rules.rs`
  - 映射规则的持久化、启停版本切换与规则读取
- `crates/erp-api/src/lib.rs`
  - 对外请求体、响应体、权限控制、审计日志、路由 handler
- `crates/erp-repo/tests/product_center.rs`
  - 商品中心相关 repo 集成测试，补批次与序列号回归
- `crates/erp-api/tests/wms_outbound_serials.rs`
  - WMS 出库链路集成测试，补序列号绑定与状态流转
- `docs/superpowers/plans/2026-04-15-industrial-erp-95-plus-round13-backend-downshift-plan.md`
  - 本轮主计划
- `docs/ops/tasks/2026-04-15/erp95-r13-0*.md`
  - 本轮 5 张任务卡
- `docs/ops/tasks/2026-04-15/erp95-round13-exec-log.md`
  - 执行日志与完成度评估

## Task 1: 食品批次 `production_date` 下沉到 Rust Repo/API

**Files:**
- Modify: `crates/erp-repo/src/sku_batches.rs`
- Modify: `crates/erp-api/src/lib.rs`
- Test: `crates/erp-repo/tests/product_center.rs`

- [ ] **Step 1: 写失败测试**

在 `crates/erp-repo/tests/product_center.rs` 增加批次创建/更新测试，断言 `production_date` 能真实写入并读出。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p erp-repo product_repo_create_update_list -- --nocapture`
Expected: 因 `production_date` 字段或断言缺失而失败

- [ ] **Step 3: 补最小实现**

- 在 `SkuBatch`、`SkuBatchCreate`、`SkuBatchUpdate` 中加入 `production_date`
- 查询、插入、更新 SQL 补齐 `production_date`
- API 请求体 `SkuBatchCreateRequest`、`SkuBatchUpdateRequest` 与审计日志补齐该字段

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p erp-repo product_repo_create_update_list -- --nocapture`
Expected: PASS

- [ ] **Step 5: 记录进度**

将实现结果补入 `docs/ops/tasks/2026-04-15/erp95-round13-exec-log.md`

## Task 2: 数码序列号绑定语义与前端治理闭环对齐

**Files:**
- Modify: `crates/erp-repo/src/sku_serials.rs`
- Modify: `crates/erp-api/src/lib.rs`
- Test: `crates/erp-api/tests/wms_outbound_serials.rs`

- [ ] **Step 1: 写失败测试**

补一条 API 级测试，断言打包/出库绑定后序列号引用单据语义与前端治理闭环一致，且重复占用会被阻断。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p erp-api wms_outbound_serials -- --nocapture`
Expected: 因绑定语义或重复占用校验不一致而失败

- [ ] **Step 3: 补最小实现**

- 对齐 `ref_doc_type` / `ref_doc_id` 的绑定语义
- 明确已发货、已分配、跨单占用的错误返回
- 保持现有前端页面可直接消费

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p erp-api wms_outbound_serials -- --nocapture`
Expected: PASS

- [ ] **Step 5: 记录进度**

将实现结果补入执行日志

## Task 3: 鞋服映射规则增加行业模板校验与版本治理

**Files:**
- Modify: `crates/erp-repo/src/mapping_rules.rs`
- Modify: `crates/erp-api/src/lib.rs`
- Test: `crates/erp-repo/src/mapping_rules.rs` 或相关 repo 测试文件

- [ ] **Step 1: 写失败测试**

补规则创建/更新测试，覆盖 `SHOP_PRODUCT` 场景下鞋服模板所需的 `industry_profile/template_name/color_dict/size_dict` 基础合法性。

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p erp-repo mapping_rules -- --nocapture`
Expected: 非法规则未被拦截或版本启停语义不满足预期

- [ ] **Step 3: 补最小实现**

- 在 API 层补 `SHOP_PRODUCT` 规则基本校验
- 在 repo 层保持同平台同实体只有一个启用版本
- 为 worker 后续消费保留稳定 JSON 结构

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test -p erp-repo mapping_rules -- --nocapture`
Expected: PASS

- [ ] **Step 5: 记录进度**

将实现结果补入执行日志

## Task 4: API 返回与前端治理字段对齐收口

**Files:**
- Modify: `apps/erp-web/src/api/skuBatches.ts`
- Modify: `apps/erp-web/src/api/skuSerials.ts`
- Modify: `apps/erp-web/src/api/mappingRules.ts`
- Modify: `apps/erp-web/src/pages/skuBatches/SkuBatchesPage.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx`

- [ ] **Step 1: 写失败测试**

补前端 API 或页面层断言，确认真实后端字段接入后不会破坏现有治理 UI。

- [ ] **Step 2: 运行测试确认失败**

Run: `pnpm --filter erp-web exec vitest run src/api/mocks/skus.runtime-governance.test.ts src/api/mocks/shopProducts.match-governance.test.ts`
Expected: 至少一处字段适配或回退逻辑失败

- [ ] **Step 3: 补最小实现**

- 对齐 `production_date`
- 对齐序列号绑定展示字段
- 对齐鞋服规则摘要依赖的数据结构

- [ ] **Step 4: 运行测试确认通过**

Run: `pnpm --filter erp-web exec vitest run src/api/mocks/skus.runtime-governance.test.ts src/api/mocks/shopProducts.match-governance.test.ts`
Expected: PASS

- [ ] **Step 5: 记录进度**

将实现结果补入执行日志

## Task 5: Round 13 fresh 验证与完成度评估

**Files:**
- Modify: `docs/ops/tasks/2026-04-15/erp95-round13-exec-log.md`

- [ ] **Step 1: 汇总本轮变更点**

列出真实后端下沉了哪些字段、校验和接口语义

- [ ] **Step 2: 运行 Rust 验证**

Run: `cargo test -p erp-repo product_center -- --nocapture`
Expected: PASS

Run: `cargo test -p erp-api wms_outbound_serials -- --nocapture`
Expected: PASS

- [ ] **Step 3: 运行前端验证**

Run: `pnpm --filter erp-web exec vitest run src/api/mocks/skus.runtime-governance.test.ts src/api/mocks/wms.runtime-governance.test.ts src/api/mocks/shopProducts.match-governance.test.ts`
Expected: PASS

- [ ] **Step 4: 更新完成度评估**

补充当前项目离工业级电商 ERP 95% 的剩余差距

- [ ] **Step 5: 记录下一轮建议**

给出 Round 14 优先级，继续收口真实持久层与任务中心能力
