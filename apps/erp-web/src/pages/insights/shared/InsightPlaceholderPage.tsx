import { Card, Descriptions, Space, Tag, Typography } from 'antd';

export type InsightDeliveryPhase = 'phase-1' | 'phase-1.5' | 'phase-2';

export type InsightPlaceholderPageProps = {
  'data-testid': string;
  title: string;
  goal: string;
  phase: InsightDeliveryPhase;
  dimensions: string[];
  metrics: string[];
  dependencies: string[];
  caliberNotes?: string[];
  currentNote?: string;
  lastUpdatedText?: string;
};

const phaseMeta: Record<InsightDeliveryPhase, { color: string; label: string }> = {
  'phase-1': { color: 'green', label: 'Phase 1 已接入' },
  'phase-1.5': { color: 'gold', label: 'Phase 1.5 拼装交付' },
  'phase-2': { color: 'volcano', label: 'Phase 2 合同待补齐' },
};

const renderTags = (items: string[]) => (
  <Space size={[8, 8]} wrap>
    {items.map((item) => (
      <Tag key={item}>{item}</Tag>
    ))}
  </Space>
);

export const InsightPlaceholderPage = ({
  'data-testid': testId,
  title,
  goal,
  phase,
  dimensions,
  metrics,
  dependencies,
  caliberNotes = [],
  currentNote = '当前页面先承载业务问题和指标定义，待主题接口接入后切换为正式分析视图。',
  lastUpdatedText = '待主题接口接入',
}: InsightPlaceholderPageProps) => (
  <div data-testid={testId}>
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        {title}
      </Typography.Title>
      <Card>
        <Typography.Paragraph style={{ marginBottom: 0 }}>{goal}</Typography.Paragraph>
      </Card>
      <Card title="接入状态">
        <Descriptions column={1} size="small" bordered>
          <Descriptions.Item label="阶段">
            <Tag color={phaseMeta[phase].color}>{phaseMeta[phase].label}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="建议维度">{renderTags(dimensions)}</Descriptions.Item>
          <Descriptions.Item label="核心指标">{renderTags(metrics)}</Descriptions.Item>
          <Descriptions.Item label="依赖数据源">{renderTags(dependencies)}</Descriptions.Item>
          <Descriptions.Item label="口径说明">
            <Space direction="vertical" size={2}>
              {caliberNotes.length > 0 ? (
                caliberNotes.map((note) => <Typography.Text key={note}>{`- ${note}`}</Typography.Text>)
              ) : (
                <Typography.Text type="secondary">待补充口径说明</Typography.Text>
              )}
            </Space>
          </Descriptions.Item>
          <Descriptions.Item label="数据更新时间">{lastUpdatedText}</Descriptions.Item>
          <Descriptions.Item label="当前说明">{currentNote}</Descriptions.Item>
        </Descriptions>
      </Card>
    </Space>
  </div>
);
