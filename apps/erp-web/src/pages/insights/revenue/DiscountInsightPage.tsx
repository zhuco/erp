import { Card, Descriptions, Space, Tag, Typography } from 'antd';

const dimensions = ['平台', '店铺', '活动', 'SKU', '品类'];
const metrics = ['折扣金额', '折扣率', '折扣订单量', '净销售额', '活动拉动率'];
const dependencies = ['订单明细', '活动信息', 'SKU 主数据', '价格快照'];

const renderTags = (items: string[]) => (
  <Space size={[8, 8]} wrap>
    {items.map((item) => (
      <Tag key={item}>{item}</Tag>
    ))}
  </Space>
);

export const DiscountInsightPage = () => (
  <div data-testid="insights-revenue-discount-insight-page">
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        让利诊断
      </Typography.Title>
      <Card>
        <Typography.Paragraph style={{ marginBottom: 0 }}>
          用于判断促销和折扣是否真实拉动订单增长，还是只是过度侵蚀利润空间。
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
            当前页面先承载业务问题和指标定义，待折扣与活动分析合同上线后再开放正式图表。
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  </div>
);
