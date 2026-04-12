# AI-6 快捷键与复制（P0）
## 目标
- 完善键盘导航与复制体验，提高批量操作效率。
- 复制范围可选“带表头”，粘贴到 Excel 可用。
- 输入控件聚焦时不触发表格快捷键。

## 范围
- 方向键移动焦点。
- Enter/F2 进入编辑，ESC 取消编辑。
- 范围复制支持可选表头。

## 允许修改文件
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/erp-data-grid/hooks/useGridKeyboard.ts`
- `apps/erp-web/src/components/erp-data-grid/utils/clipboard.ts`

## 接口设计
- `keyboardConfig`
  - `enabled?: boolean`：默认启用。
  - `shortcuts?: Partial<Record<Action, Binding[]>>`
  - `Action`：`moveUp`、`moveDown`、`moveLeft`、`moveRight`、`edit`、`cancelEdit`、`copy`
  - `Binding`：`{ key, ctrl?, meta?, shift?, alt? }`
- `clipboardConfig`
  - `enabled?: boolean`：默认启用。
  - `includeHeaders?: boolean`：默认包含表头。

## 实现策略
- `useGridKeyboard` 监听全局 `keydown`，根据激活表格与事件目标判定是否处理，保持输入控件过滤逻辑。
- 方向键优先调用 `navigateUp/Down/Left/Right`，缺失时退化到 `navigateNext/Prev`。
- Enter/F2 调用 `cell.edit()`，ESC 调用 `cancelEdit()`。
- 复制读取 `table.getRanges()`，按 `\t`/`\n` 拼接文本，`includeHeaders` 控制是否加入表头。
- 范围复制失败时回退到 `table.copyToClipboard('range')`。

## 验收标准
- 方向键移动焦点稳定，Enter/F2/ESC 行为正确。
- 复制范围包含表头（可配置），粘贴到 Excel 可用。
- 输入控件聚焦时不触发表格快捷键。

## 测试建议
- 订单审核页进行键盘导航与范围复制验证。

## 可直接投喂提示词
请在 ERP 项目中完成「ErpDataGrid 快捷键与复制」任务：  
【仓库】`F:\ERP`  
【核心目标】键盘导航稳定、复制范围可含表头、避免输入控件抢占快捷键。  
【允许修改】`apps/erp-web/src/components/ErpDataGrid.tsx`、`apps/erp-web/src/components/erp-data-grid/hooks/useGridKeyboard.ts`、`apps/erp-web/src/components/erp-data-grid/utils/clipboard.ts`  
【接口建议】  
- `keyboardConfig`、`clipboardConfig`  
【实现要求】  
- 方向键导航 + Enter/F2 编辑 + ESC 取消。  
- 范围复制可选“带表头”。  
- 输入控件聚焦时不触发表格快捷键。  
【交付物】  
- 修改文件清单  
- 关键实现说明  
- 手动测试步骤  
