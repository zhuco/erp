# ERP 文档中心

> 更新日期：2026-09-09
> 当前版本：`0.1.9`
> 目标：把原先分散在多个历史目录中的文档，收敛为少量长期维护的权威文档。

## 推荐阅读顺序

1. `README.md`：项目定位、启动方式与仓库结构。
2. `docs/PRODUCT_SCOPE.md`：约定要实现的产品能力、优先级与非目标。
3. `docs/ROADMAP.md`：唯一后续开发计划入口，包含当前评估、目标版本、近期执行队列与验收门禁。
4. `docs/ARCHITECTURE.md`：前后端、事件链路、数据边界与接口契约。
5. `docs/FRONTEND_GUIDE.md`：Rust 原生桌面与 WASM Web、格式及测试规范。
6. `docs/OPERATIONS.md`：本地运行、验收门禁、发布、安全与运维要求。
7. [自研组件库开发规范](UI_COMPONENT_LIBRARY.md)：自研数据网格及四类配套组件的唯一详细技术规范，包含 AG Grid 全面禁用规则、功能对标、接口、迁移和验收。

## 文档权威层级

实现现状由代码和可复现证据确定；目标约束由用户最新决定及当前规范确定。旧代码含有 AG Grid 不构成继续使用的授权。目标约束冲突时，先遵循用户最新决定和根目录 [AGENTS.md](../AGENTS.md)，再按以下文档职责查阅：

1. 真实代码与契约：`openapi/erp_api_v1.yaml`、`migrations/`、`crates/`、`apps/erp-web/`。
2. 产品与路线图：`docs/PRODUCT_SCOPE.md`、`docs/ROADMAP.md`。
3. 架构与工程规范：`docs/ARCHITECTURE.md`、`docs/FRONTEND_GUIDE.md`、`docs/OPERATIONS.md`。
   组件库专题的详细约束以 `docs/UI_COMPONENT_LIBRARY.md` 为准，实施进度以 `docs/ROADMAP.md` 为准。
4. 历史结论：以 Git 历史中的旧文档为准，不再在工作树保留多份快照。

## 合并后的口径

- 2026-09-09：全 Rust UI，原生桌面与 WASM Web 共用 egui；固定 Rust 1.98.1、Edition/格式风格 2024。旧 React 路线废止，代码仍在；配置不等于编译和功能验收。

- 当前交付顺序：自己的店铺实用 → 3—5 家商家试用 → 有稳定收入后建设 SaaS；后续各版本能力是长期储备，不是自用前置条件。
- 长期项目目标：建设面向多平台、多店铺、多仓、多角色协同的电商 ERP。
- 首个可收费版本：优先打通平台接入、订单、库存、仓储、履约、售后、基础财务与报表的闭环。
- 稳定经营版本：补齐多组织、多商户、采购计划、审批中心、异常中心、质量追溯和财务经营闭环。
- 工业化平台版本：强化可配置流程、开放集成、经营驾驶舱、自动化补偿、SLA 与多形态交付能力。

## 文档维护规则

- 新增总览性结论时，优先更新 `docs/PRODUCT_SCOPE.md` 或 `docs/ROADMAP.md`；后续开发计划只能进入 `docs/ROADMAP.md`，不要再创建新的总览文档。
- 新增技术约定时，优先更新 `docs/ARCHITECTURE.md`、`docs/FRONTEND_GUIDE.md` 或 `docs/OPERATIONS.md`。
- 用户明确要求建立的组件库专题集中维护在 `docs/UI_COMPONENT_LIBRARY.md`；该文档不另建项目路线图，不复制历史进度表。
- 阶段报告、临时任务卡、并行开发记录、一次性验收快照不再放入 `docs/`；需要留痕时使用 Issue、PR 描述、Git 提交或临时目录。
- 如果确实需要恢复旧文档，请从 Git 历史查询；当前工作树只保留长期维护入口。
