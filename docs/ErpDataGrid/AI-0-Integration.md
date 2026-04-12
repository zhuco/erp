# AI-0 集成负责人（P0）

## 目标
- 统一 `ErpDataGrid` 入口、接口与目录结构，保证并行任务可安全集成。
- 保持现有页面不破坏，新增能力均为可选参数。

## 范围
- 只由 AI-0 负责修改 `ErpDataGrid.tsx` 的入口编排与导出。
- 创建并维护 `apps/erp-web/src/components/erp-data-grid/` 模块目录。

## 允许修改文件
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/ErpDataGrid.css`
- `apps/erp-web/src/components/erp-data-grid/**`

## 禁止修改
- 业务页面（如 `OrderReviewPage.tsx`）除非与集成强相关并明确说明。

## 实现策略
- 抽离公共类型（如列状态、排序状态、过滤状态）到 `utils/types.ts`。
- 统一本地存储读写逻辑，集中到 `utils/storage.ts`。
- 在 `ErpDataGrid.tsx` 中仅做模块编排与 props 透传，不写复杂逻辑。
- 对外暴露的 props 仅新增可选项，保持向后兼容。

## 接口变更（汇总入口）
- 多列排序：`sortStates`、`onSortStatesChange`
- 列视图：`columnViewKey`、`defaultColumnState`
- 过滤体系：`globalFilter`、`filterState`、`filterMode`
- 状态栏：`summaryFields`、`summaryResolver`
- 快捷键/复制：`keyboardConfig`、`clipboardConfig`
- 导出：`exportConfig`
- 分组聚合：`groupConfig`
- 批量编辑：`batchEditConfig`

## 验收标准
- 现有页面无功能回退，`ErpDataGrid` 旧 props 可正常工作。
- 新增模块均可通过入口统一启用/关闭。
- 文档与接口定义同步更新。

## 测试建议
- 复跑已有订单审核相关 E2E。
- 新增功能通过手测清单验证。

## 可直接投喂提示词
请在 ERP 项目中担任 ErpDataGrid 集成负责人，完成以下任务：  
【仓库】`F:\ERP`  
【核心目标】统一入口、接口与目录结构，保证并行模块可安全集成。  
【允许修改】`apps/erp-web/src/components/ErpDataGrid.tsx`、`apps/erp-web/src/components/ErpDataGrid.css`、`apps/erp-web/src/components/erp-data-grid/**`  
【禁止修改】业务页面（如 `OrderReviewPage.tsx`），除非与集成强相关并明确说明。  
【实现要求】  
- 对外 props 仅新增可选项，保持向后兼容。  
- 抽离公共类型与存储逻辑到 `apps/erp-web/src/components/erp-data-grid/utils/`。  
- `ErpDataGrid.tsx` 只做编排与透传，不写复杂逻辑。  
【交付物】  
- 修改文件清单  
- 关键实现说明  
- 手动测试步骤（无自动化请注明）
