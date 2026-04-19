import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const ReturnLoopPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-service-return-loop-page"
    title="退回闭环"
    goal="观察退货从申请到签收、质检、入库、完结是否形成闭环。"
    phase="phase-2"
    dimensions={['退货原因', '仓库', '店铺', '平台', 'SKU', '质检员']}
    metrics={['退货单量', '退货件数', '质检合格率', '平均退回周期', '待入库量']}
    dependencies={['售后单', '退货入库', '质检单', '物流签收记录']}
    caliberNotes={[
      '平均退回周期 = 退货申请时间到售后完结时间',
      '质检合格率 = 质检通过件数 / 质检件数',
      '待入库量为签收后尚未入库的退货件数',
    ]}
  />
);
