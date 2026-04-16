import { Card, Descriptions, Space, Tag, Typography } from 'antd';

const dimensions = ['原因', '店铺', '平台', '审核人'];
const metrics = ['补偿单量', '补偿金额', '单均补偿', '审核周期', '驳回率'];
const dependencies = ['售后单', '退款单', '审核记录', '财务补偿台账'];

const renderTags = (items: string[]) => (
  <Space size={[8, 8]} wrap>
    {items.map((item) => (
      <Tag key={item}>{item}</Tag>
    ))}
  </Space>
);

export const CompensationPage = () => (
  <div data-testid="insights-service-compensation-page">
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        补偿看板
      </Typography.Title>
      <Card>
        <Typography.Paragraph style={{ marginBottom: 0 }}>
          用于统一观察退款、补差、补贴等补偿支出，帮助售后与财务识别异常成本和审批瓶颈。
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
            当前页面先提供业务定义与指标口径，待售后补偿主题接口完成后开放正式统计能力。
          </Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  </div>
);
