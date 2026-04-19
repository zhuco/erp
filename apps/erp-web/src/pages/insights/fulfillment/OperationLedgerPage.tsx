import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const OperationLedgerPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-fulfillment-operation-ledger-page"
    title="收发作业流水"
    goal="统一查看收货、发货、盘点、调拨等仓内作业量，快速定位负载峰值和异常来源。"
    phase="phase-2"
    dimensions={['单据类型', '业务单据', '仓库', 'SKU', '操作员']}
    metrics={['收货单数', '发货单数', '收货件数', '发货件数', '作业行数']}
    dependencies={['入库单', '出库单', '调拨单', '盘点单', '操作日志']}
    caliberNotes={[
      '明细区支持单据视图与货品视图切换',
      '汇总区按自然日和仓库进行聚合',
      '流水行需支持按单据号追溯到原始业务页面',
    ]}
    currentNote="当前页面先承载统一作业流水定义，待主题接口上线后补齐趋势图和分页明细。"
  />
);
