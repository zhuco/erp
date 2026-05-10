# ERP 文档中心

> 更新日期：2026-05-10
> 当前版本：`0.1.9`
> 目标：把原先分散在多个历史目录中的文档，收敛为少量长期维护的权威文档。

## 推荐阅读顺序

1. `README.md`：项目定位、启动方式与仓库结构。
2. `docs/PRODUCT_SCOPE.md`：约定要实现的产品能力、优先级与非目标。
3. `docs/ROADMAP.md`：唯一后续开发计划入口，包含当前评估、目标版本、近期执行队列与验收门禁。
4. `docs/ARCHITECTURE.md`：前后端、事件链路、数据边界与接口契约。
5. `docs/FRONTEND_GUIDE.md`：前端页面、工作台、表格与测试规范。
6. `docs/OPERATIONS.md`：本地运行、验收门禁、发布、安全与运维要求。

## 文档权威层级

同一主题出现冲突时，按以下顺序裁决：

1. 真实代码与契约：`openapi/erp_api_v1.yaml`、`migrations/`、`crates/`、`apps/erp-web/`。
2. 产品与路线图：`docs/PRODUCT_SCOPE.md`、`docs/ROADMAP.md`。
3. 架构与工程规范：`docs/ARCHITECTURE.md`、`docs/FRONTEND_GUIDE.md`、`docs/OPERATIONS.md`。
4. 历史结论：以 Git 历史中的旧文档为准，不再在工作树保留多份快照。

## 合并后的口径

- 项目目标：建设面向多平台、多店铺、多仓、多角色协同的工业级电商 ERP。
- 首个可收费版本：优先打通平台接入、订单、库存、仓储、履约、售后、基础财务与报表的闭环。
- 稳定经营版本：补齐多组织、多商户、采购计划、审批中心、异常中心、质量追溯和财务经营闭环。
- 工业化平台版本：强化可配置流程、开放集成、经营驾驶舱、自动化补偿、SLA 与多形态交付能力。

## 文档维护规则

- 新增总览性结论时，优先更新 `docs/PRODUCT_SCOPE.md` 或 `docs/ROADMAP.md`；后续开发计划只能进入 `docs/ROADMAP.md`，不要再创建新的总览文档。
- 新增技术约定时，优先更新 `docs/ARCHITECTURE.md`、`docs/FRONTEND_GUIDE.md` 或 `docs/OPERATIONS.md`。
- 阶段报告、临时任务卡、并行开发记录、一次性验收快照不再放入 `docs/`；需要留痕时使用 Issue、PR 描述、Git 提交或临时目录。
- 如果确实需要恢复旧文档，请从 Git 历史查询；当前工作树只保留长期维护入口。
