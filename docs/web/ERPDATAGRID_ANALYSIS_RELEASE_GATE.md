# ErpDataGrid 分析域发布门禁（工业级）

更新时间：2026-04-10  
适用范围：`apps/erp-web` 中 ErpDataGrid 分析能力发布（筛选、透视、导出、编辑、树明细、图表、入口集成）

## 1. 门禁分层

- `P0`：阻断发布，未通过不得上线
- `P1`：增强项，可灰度但不得影响 P0 合规性

## 2. P0 阻断项

- [ ] 类型检查通过：`pnpm --filter erp-web run typecheck`
- [ ] 分析域单测通过：`pnpm --filter erp-web run test:unit:grid-analysis`
- [ ] 分析域 E2E 通过：`pnpm --filter erp-web run test:e2e:grid-analysis`
- [ ] 分档性能门禁通过：`pnpm --filter erp-web run test:e2e:grid-performance-tier`
- [ ] 企业专项 E2E 通过：`pnpm --filter erp-web run test:e2e:grid-enterprise-95`
- [ ] 综合门禁通过：`pnpm --filter erp-web run test:gate:grid-analysis`
- [ ] 企业 95% 综合门禁通过：`pnpm --filter erp-web run test:gate:grid-enterprise-95`

## 3. 企业专项 E2E 基线

以下文件必须全部纳入企业门禁执行集：

- `e2e/erp-data-grid-enterprise-filter-suite.spec.ts`
- `e2e/erp-data-grid-group-pivot-enterprise.spec.ts`
- `e2e/erp-data-grid-export-enterprise.spec.ts`
- `e2e/erp-data-grid-chart-status.spec.ts`
- `e2e/erp-data-grid-range-editing.spec.ts`
- `e2e/erp-data-grid-fill-paste-enterprise.spec.ts`
- `e2e/erp-data-grid-tree-detail-enterprise.spec.ts`
- `e2e/erp-data-grid-sparkline.spec.ts`
- `e2e/erp-data-grid-workbench-integration.spec.ts`
- `e2e/erp-data-grid-enterprise-gate.spec.ts`

## 4. AI-6 集成验收点

- [ ] SidePanel customTabs 标准化（空键过滤、重复键去重、顺序稳定）
- [ ] SidePanel 核心标签与 customTabs 生命周期一致
- [ ] Context Menu 权限钩子生效（阻断动作不可执行）
- [ ] 交互审计事件落地：`erp-data-grid:interaction-trace`
- [ ] 发布清单文档已生成并可执行：`docs/web/ERPDATAGRID_ENTERPRISE_95_RELEASE_CHECKLIST.md`

## 5. 发布证据模板

发布时至少归档以下信息：

- [ ] 执行命令与时间戳
- [ ] 测试结果摘要（通过/失败、失败用例、重试次数）
- [ ] 提交 SHA 与分支信息
- [ ] 发布结论（允许发布 / 阻断发布）

## 6. 2026-04-10 增量门禁（AI-6 集成）

- [ ] SidePanel 快捷键覆盖列/筛选/透视/导出/图表：`Ctrl+Shift+L/F/P/E/C`
- [ ] SidePanel 快捷键防误触：`Alt`/`Meta` 组合不触发面板动作
- [ ] SidePanel customTabs 仅接受有效 `render`，并对 `title` 做空白归一
- [ ] Pivot 面板运行态配置对齐（rowGroups/columnGroups/values）与服务端参数一致
- [ ] `src/components/ErpDataGrid.pivot-config.test.ts` 通过，确保透视配置在运行态可预测
