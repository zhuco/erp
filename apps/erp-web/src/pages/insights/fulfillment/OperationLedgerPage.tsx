import { Card, Descriptions, Space, Tag, Typography } from 'antd';

const dimensions = ['单据类型', '业务单据', '仓库', 'SKU', '操作员'];
const metrics = ['收货单数', '发货单数', '收货件数', '发货件数', '作业行数'];
const dependencies = ['入库', '出库', '调拨', '盘点', '操作日志'];

const renderTags = (items: string[]) => (
  <Space size={[8, 8]} wrap>
    {items.map((item) => (
      <Tag key={item}>{item}</Tag>
    ))}
  </Space>
);

export const OperationLedgerPage = () => (
  <div data-testid="insights-fulfillment-operation-ledger-page">
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        收发作业流水
      </Typography.Title>
      <Card>
        <Typography.Paragraph style={{ marginBottom: 0 }}>
          用于统一查看收货、发货、盘点、调拨等仓内作业流水，帮助现场管理快速定位负载和异常来源。
        </Typography.Paragraph>
      </Card>
      <Card title="接入状态">
        <Descriptions column={1} size="small" bordered>
          <Descriptions.Item label="阶段">
            <Tag color="volcano">Phase 2 合同待补齐</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="建议维度">{renderTags(dimensions)}</Descriptions.Item>
          <Descriptions.Item label="核心指标">{renderTags(metrics)}</Descriptions.Item>
          <Descriptions.Item label="依赖数据源">{renderTags(dependencies)}</Descriptions.Item>
          <Descriptions.Item label="当前说明">
            当前页面先作为洞察入口占位，待统一作业流水接口完成后切换为正式分析页。
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  </div>
);
