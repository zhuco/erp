# ErpDataGrid 企业版体验并行任务说明

## 目标
- 在不更换技术栈的前提下，让 `ErpDataGrid` 逐步达到企业级电商 ERP 的使用体验。
- 优先解决性能稳定、核心效率与一致交互，再补高级分析能力。
- 支持多人并行开发，避免冲突与返工。

## 并行原则
- **仅 AI-0 可修改 `ErpDataGrid.tsx`**（集成入口），其他 AI 仅新增模块或修改指定文件。
- **所有 UI 文案、日志、注释必须为中文**，保持现有风格一致。
- **新增能力通过可选参数暴露**，避免破坏现有页面。
- **提交必须包含验收标准与手测步骤**，方便统一验收。

## 目录规范（建议新增）
```
apps/erp-web/src/components/erp-data-grid/
  components/
  hooks/
  utils/
```

## 交付标准
- 功能完整、交互稳定、性能可接受（至少 5k 行不卡顿）。
- 不引入大型新依赖；如确有必要需说明理由。
- 文档内明确验收标准与测试步骤。

## 并行任务清单
- `AI-0-Integration.md`：集成与接口管理（P0）
- `AI-1-Performance.md`：渲染与性能（P0）
- `AI-2-MultiSort.md`：多列排序（P0）
- `AI-3-ColumnSettings.md`：列管理面板（P0）
- `AI-4-Filtering.md`：快速筛选与条件面板（P0/P1）
- `AI-5-StatusBar.md`：状态栏与汇总（P0）
- `AI-6-ShortcutsClipboard.md`：快捷键与复制（P0）
- `AI-7-Export.md`：导出能力（P0）
- `AI-8-GroupingAggregation.md`：分组与聚合（P1）
- `AI-9-BatchEditUndo.md`：批量编辑与撤销重做（P1）

## 验收共识（P0）
- 列宽拖拽不卡顿，滚动/焦点不跳行。
- 支持多列排序与优先级标识。
- 列管理面板可显隐/拖拽/冻结/保存视图。
- 全局筛选、状态栏汇总、快捷键与复制可用。

## 并行投喂说明
- 每个 `AI-*.md` 文档末尾包含「可直接投喂提示词」，可直接复制到新的 AI 窗口。
- 建议先发 `AI-0-Integration.md` 给集成 AI，其他 AI 各自领取对应任务。
