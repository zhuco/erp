import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const TurnoverVelocityPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-inventory-turnover-velocity-page"
    title="周转流速"
    goal="评估库存流动性，识别流速过慢或异常冻结库存。"
    phase="phase-1.5"
    dimensions={['仓库', 'SKU', '品类', '库存状态']}
    metrics={['周转天数', '流速', '现货', '可用量', '锁定量', '冻结量']}
    dependencies={['库存余额', '库存状态', '出库销量']}
    caliberNotes={[
      '流速 = 近 30 天销量 / 平均库存',
      '周转天数与流速同时展示，避免单指标偏差',
      '冻结与锁定单独呈现，避免“有库存但不可售”误判',
    ]}
    currentNote="当前页面按 Phase 1.5 先拼装上线，后续补充主题化明细接口。"
  />
);
