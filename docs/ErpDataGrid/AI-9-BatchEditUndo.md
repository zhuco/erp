# AI-9 批量编辑与撤销重做（P1）

## 目标
- 支持批量编辑、字段校验与撤销/重做。

## 范围
- 编辑入口、批量应用、撤销栈。

## 允许修改文件
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/erp-data-grid/hooks/useGridEditing.ts`
- `apps/erp-web/src/components/erp-data-grid/utils/validation.ts`

## 实现策略
- 通过 `batchEditConfig` 定义可批量编辑字段与校验规则。
- 使用 Tabulator `history` 模块实现撤销/重做。
- 编辑失败提供中文提示与高亮。

## 验收标准
- 批量编辑可一次应用多行，并支持撤销/重做。
- 校验失败时阻止提交并提示具体原因。

## 测试建议
- 订单审核批量修改仓库/物流字段并撤销验证。

## 可直接投喂提示词
请在 ERP 项目中完成「ErpDataGrid 批量编辑与撤销重做」任务：  
【仓库】`F:\ERP`  
【核心目标】批量编辑可用、校验明确、撤销/重做稳定。  
【允许修改】`apps/erp-web/src/components/ErpDataGrid.tsx`、`apps/erp-web/src/components/erp-data-grid/hooks/useGridEditing.ts`、`apps/erp-web/src/components/erp-data-grid/utils/validation.ts`  
【接口建议】  
- `batchEditConfig`：字段配置、校验规则、编辑入口  
【实现要求】  
- 使用 Tabulator `history` 模块实现撤销/重做。  
- 校验失败给出中文提示与高亮。  
【交付物】  
- 修改文件清单  
- 关键实现说明  
- 手动测试步骤
