import { Card, Descriptions, Space, Tag, Typography } from 'antd';

const dimensions = ['平台', '店铺', '品类', '品牌', 'SKU'];
const metrics = ['订单量', '售出件数', 'GMV', 'TOP5 贡献率', '长尾占比'];
const dependencies = ['订单明细', 'SKU/品类/品牌主数据'];

const renderTags = (items: string[]) => (
  <Space size={[8, 8]} wrap>
    {items.map((item) => (
      <Tag key={item}>{item}</Tag>
    ))}
  </Space>
);

export const OrderMixPage = () => (
  <div data-testid="insights-revenue-order-mix-page">
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        订单结构
      </Typography.Title>
      <Card>
        <Typography.Paragraph style={{ marginBottom: 0 }}>
          用于观察收入结构是否过度集中，帮助运营识别平台、店铺、品类和商品结构失衡。
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
            当前先提供业务定义与统计口径，待结构分析主题接口完成后开放正式分析能力。
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  </div>
);
