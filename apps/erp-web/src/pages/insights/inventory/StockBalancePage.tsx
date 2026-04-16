import { Card, Descriptions, Space, Tag, Typography } from 'antd';

const dimensions = ['仓库', 'SKU', '品类', '事务类型'];
const metrics = ['期初量', '入库量', '出库量', '期末量', '净变化量'];
const dependencies = ['库存流水', '仓库主数据', 'SKU 主数据'];

const renderTags = (items: string[]) => (
  <Space size={[8, 8]} wrap>
    {items.map((item) => (
      <Tag key={item}>{item}</Tag>
    ))}
  </Space>
);

export const StockBalancePage = () => (
  <div data-testid="insights-inventory-stock-balance-page">
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        收发存总览
      </Typography.Title>
      <Card>
        <Typography.Paragraph style={{ marginBottom: 0 }}>
          用于回答“期初、收入、发出、结余是如何变化的”，帮助库存负责人快速判断库存波动是否健康。
        </Typography.Paragraph>
      </Card>
      <Card title="接入状态">
        <Descriptions column={1} size="small" bordered>
          <Descriptions.Item label="阶段">
            <Tag color="gold">Phase 1.5 拼装交付</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="建议维度">{renderTags(dimensions)}</Descriptions.Item>
          <Descriptions.Item label="核心指标">{renderTags(metrics)}</Descriptions.Item>
          <Descriptions.Item label="依赖数据源">{renderTags(dependencies)}</Descriptions.Item>
          <Descriptions.Item label="当前说明">
            当前页面先作为洞察入口占位，待 reports 与库存接口拼装完成后切换为正式分析页。
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  </div>
);
