# ErpDataGrid Enterprise 95% 发布检查清单

更新时间：2026-04-10  
负责人：AI-6（集成与门禁）

## 1. 集成范围确认

- [ ] 入口文件已完成统一收口：`apps/erp-web/src/components/ErpDataGrid.tsx`
- [ ] SidePanel 核心与自定义标签页生命周期一致：`SidePanel.tsx` + `useGridSidePanel.ts`
- [ ] Column Menu / Context Menu 行为与权限钩子可配置
- [ ] 自定义标签页配置已做标准化（去空键、去重、稳定顺序）
- [ ] 关键交互审计事件可对外订阅（`erp-data-grid:interaction-trace`）

## 2. 工业级门禁命令

按顺序执行，任一失败即阻断发布：

```bash
pnpm --filter erp-web run typecheck
pnpm --filter erp-web run test:unit:grid-analysis
pnpm --filter erp-web run test:e2e:grid-analysis
pnpm --filter erp-web run test:e2e:grid-performance-tier
pnpm --filter erp-web run test:e2e:grid-enterprise-95
pnpm --filter erp-web run test:gate:grid-analysis
pnpm --filter erp-web run test:gate:grid-enterprise-95
```

## 3. 企业专项 E2E 组成

`test:e2e:grid-enterprise-95` 必须覆盖以下套件：

- [ ] `e2e/erp-data-grid-enterprise-filter-suite.spec.ts`
- [ ] `e2e/erp-data-grid-group-pivot-enterprise.spec.ts`
- [ ] `e2e/erp-data-grid-export-enterprise.spec.ts`
- [ ] `e2e/erp-data-grid-chart-status.spec.ts`
- [ ] `e2e/erp-data-grid-range-editing.spec.ts`
- [ ] `e2e/erp-data-grid-fill-paste-enterprise.spec.ts`
- [ ] `e2e/erp-data-grid-tree-detail-enterprise.spec.ts`
- [ ] `e2e/erp-data-grid-sparkline.spec.ts`
- [ ] `e2e/erp-data-grid-workbench-integration.spec.ts`
- [ ] `e2e/erp-data-grid-enterprise-gate.spec.ts`

## 4. 审计与可观测字段抽检

- [ ] 导出审计：`requestId/status/mode/requestedType/usedType/fileName/rowCount/durationMs`
- [ ] 交互审计：`action/status/source/component/dataTestId/rowModel/timestamp`
- [ ] 上下文菜单关键动作（全选/反选/导出/打印）有执行与阻断记录
- [ ] SidePanel 打开与切换动作可追踪

## 5. 发布证据归档

- [ ] 记录执行命令与时间
- [ ] 保存关键测试日志（含失败重试记录）
- [ ] 记录 commit SHA 与责任人
- [ ] 形成发布结论（允许发布 / 阻断发布）

## 6. AI-6 本轮增量检查（2026-04-10）

- [ ] SidePanel 快捷键 `Ctrl+Shift+L/F/P/E/C` 全部可用，编辑态输入框不误触
- [ ] SidePanel 快捷键在 `Alt` / `Meta` 组合键下不会误触发
- [ ] customTabs 非法 `render` 会被忽略，空白标题会回退为 key
- [ ] Pivot 面板值字段变更后，透视列与服务端 pivot 参数同步更新
- [ ] `erp-data-grid-workbench-integration.spec.ts` 与 `useGridSidePanel.test.ts` 新增用例全部通过
