import { Alert, Button, Card, Descriptions, Space, Tag, Typography } from 'antd';

import type { Channel } from '@/api/channels';
import { PlatformTag } from '@/components/PlatformTag';

import type {
  ChannelAuthCallbackState,
  ChannelQuickFilter,
  ChannelQuickFilterId,
  ChannelWorkbenchSummary,
} from './channelWorkbenchModel';

type ChannelsWorkbenchOverviewProps = {
  summary: ChannelWorkbenchSummary;
  callbackState: ChannelAuthCallbackState | null;
};

type ChannelsWorkbenchLeftPanelProps = {
  quickFilters: ChannelQuickFilter[];
  activeQuickFilterId: ChannelQuickFilterId | null;
  activeFilterSummary: string[];
  selectedChannel: Channel | null;
  onSelectQuickFilter: (filterId: ChannelQuickFilterId) => void;
};

type ChannelsWorkbenchRightPanelProps = {
  selectedChannel: Channel | null;
  warehouseMap: Map<string, string>;
  callbackUrl: string | null;
  callbackState: ChannelAuthCallbackState | null;
  onCopyCallbackUrl: () => void;
  onOpenShops: () => void;
  onOpenSyncJobs: () => void;
};

const statusMeta = {
  ACTIVE: { label: '启用中', color: 'success' },
  DISABLED: { label: '已停用', color: 'default' },
} as const;

const formatDateTime = (value?: string | null) =>
  value ? new Date(value).toLocaleString('zh-CN', { hour12: false }) : '-';

const resolveCallbackAlert = (callbackState: ChannelAuthCallbackState) => {
  if (callbackState.hasError) {
    return {
      type: 'error' as const,
      title: '平台授权回调返回错误',
      description:
        callbackState.errorDescription || callbackState.error || '请检查平台授权链路后重试。',
    };
  }

  if (callbackState.hasCode) {
    return {
      type: 'success' as const,
      title: '平台已回跳到渠道目录',
      description: '系统已识别本次授权回调，请继续在店铺接入页面完成后续绑定。',
    };
  }

  return {
    type: 'info' as const,
    title: '当前地址可作为平台授权回调地址',
    description: '系统会按当前渠道自动生成标准回调地址，平台授权完成后统一回到本页。',
  };
};

const SummaryMetricCard = ({
  label,
  value,
  accent,
}: {
  label: string;
  value: number;
  accent?: 'positive' | 'muted';
}) => (
  <div
    className={`channels-overview-card${accent ? ` is-${accent}` : ''}`}
    data-testid={`channels-overview-card-${label}`}
  >
    <Typography.Text className="channels-overview-label">{label}</Typography.Text>
    <Typography.Title level={4} className="channels-overview-value">
      {value}
    </Typography.Title>
  </div>
);

export const ChannelsWorkbenchOverview = ({
  summary,
  callbackState,
}: ChannelsWorkbenchOverviewProps) => (
  <Space orientation="vertical" size={12} style={{ width: '100%' }}>
    {callbackState ? (
      <Alert showIcon data-testid="channels-callback-alert" {...resolveCallbackAlert(callbackState)} />
    ) : null}
    <div className="channels-workbench-overview" data-testid="channels-workbench-overview">
      <SummaryMetricCard label="渠道总数" value={summary.total} />
      <SummaryMetricCard label="启用渠道" value={summary.active} accent="positive" />
      <SummaryMetricCard label="手工建单" value={summary.manualOrder} />
      <SummaryMetricCard label="已配默认仓" value={summary.defaultWarehouseConfigured} />
    </div>
  </Space>
);

export const ChannelsWorkbenchLeftPanel = ({
  quickFilters,
  activeQuickFilterId,
  activeFilterSummary,
  selectedChannel,
  onSelectQuickFilter,
}: ChannelsWorkbenchLeftPanelProps) => (
  <div className="channels-left-panel" data-testid="channels-left-panel">
    <Card size="small" title="运营视图">
      <Space orientation="vertical" size={8} style={{ width: '100%' }}>
        {quickFilters.map((filter) => (
          <Button
            key={filter.id}
            block
            type={filter.id === activeQuickFilterId ? 'primary' : 'default'}
            className="channels-left-filter-button"
            data-testid={`channels-left-quick-filter-${filter.id}`}
            onClick={() => onSelectQuickFilter(filter.id)}
          >
            <span>{filter.label}</span>
            <span>{filter.count}</span>
          </Button>
        ))}
      </Space>
    </Card>

    <Card size="small" title="当前筛选">
      <Space orientation="vertical" size={8} style={{ width: '100%' }}>
        {activeFilterSummary.length ? (
          activeFilterSummary.map((item) => (
            <Tag key={item} className="channels-filter-tag">
              {item}
            </Tag>
          ))
        ) : (
          <Typography.Text type="secondary">当前没有额外筛选条件。</Typography.Text>
        )}
      </Space>
    </Card>

    <Card size="small" title="当前活动渠道">
      {selectedChannel ? (
        <Space orientation="vertical" size={6} style={{ width: '100%' }}>
          <PlatformTag platform={selectedChannel.platform} />
          <Typography.Text strong>{selectedChannel.name}</Typography.Text>
          <Typography.Text type="secondary">
            {selectedChannel.owner || selectedChannel.department || '待补充负责人'}
          </Typography.Text>
        </Space>
      ) : (
        <Typography.Text type="secondary">请选择渠道查看详情和授权信息。</Typography.Text>
      )}
    </Card>

    <Card size="small" title="授权说明">
      <Typography.Paragraph type="secondary" className="channels-panel-paragraph">
        渠道目录负责统一展示授权回调地址和接入说明，实际店铺授权发起动作仍在店铺接入链路完成。
      </Typography.Paragraph>
    </Card>
  </div>
);

export const ChannelsWorkbenchRightPanel = ({
  selectedChannel,
  warehouseMap,
  callbackUrl,
  callbackState,
  onCopyCallbackUrl,
  onOpenShops,
  onOpenSyncJobs,
}: ChannelsWorkbenchRightPanelProps) => {
  const callbackAlert = callbackState ? resolveCallbackAlert(callbackState) : null;

  if (!selectedChannel) {
    return (
      <div className="channels-right-panel" data-testid="channels-right-panel">
        <Card size="small" title="渠道详情">
          <Space orientation="vertical" size={10} style={{ width: '100%' }}>
            {callbackAlert ? <Alert showIcon {...callbackAlert} /> : null}
            <Typography.Text type="secondary">
              请选择渠道查看主数据、授权回调地址和下游联动入口。
            </Typography.Text>
          </Space>
        </Card>
      </div>
    );
  }

  return (
    <div className="channels-right-panel" data-testid="channels-right-panel">
      <Card size="small" title="渠道资料">
        <Descriptions size="small" column={1}>
          <Descriptions.Item label="平台">
            <PlatformTag platform={selectedChannel.platform} />
          </Descriptions.Item>
          <Descriptions.Item label="渠道名称">{selectedChannel.name}</Descriptions.Item>
          <Descriptions.Item label="负责部门">{selectedChannel.department || '-'}</Descriptions.Item>
          <Descriptions.Item label="负责人">{selectedChannel.owner || '-'}</Descriptions.Item>
          <Descriptions.Item label="默认仓">
            {selectedChannel.default_warehouse_id
              ? warehouseMap.get(selectedChannel.default_warehouse_id) ||
                selectedChannel.default_warehouse_id
              : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="手工建单">
            {selectedChannel.allow_manual_order ? <Tag color="green">允许</Tag> : <Tag>不允许</Tag>}
          </Descriptions.Item>
          <Descriptions.Item label="手工售后">
            {selectedChannel.allow_manual_after_sales ? (
              <Tag color="green">允许</Tag>
            ) : (
              <Tag>不允许</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="佣金公式">
            {selectedChannel.commission_formula || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="状态">
            <Tag color={statusMeta[selectedChannel.status].color}>
              {statusMeta[selectedChannel.status].label}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="更新时间">
            {formatDateTime(selectedChannel.updated_at || selectedChannel.created_at)}
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Card size="small" title="平台授权">
        <Space orientation="vertical" size={10} style={{ width: '100%' }}>
          {callbackAlert ? <Alert showIcon {...callbackAlert} /> : null}
          <Typography.Text type="secondary">
            系统会按当前选中渠道自动生成回调地址，平台授权完成后统一回到本页。
          </Typography.Text>
          <div className="channels-auth-callback-block">
            <Typography.Text type="secondary">回调地址</Typography.Text>
            <pre className="channels-auth-callback-url" data-testid="channels-auth-callback-url">
              {callbackUrl || '请先选择渠道以生成回调地址'}
            </pre>
          </div>
          <Space wrap>
            <Button onClick={onCopyCallbackUrl}>复制回调地址</Button>
            <Button type="primary" onClick={onOpenShops}>
              去店铺接入
            </Button>
          </Space>
        </Space>
      </Card>

      <Card size="small" title="联动入口">
        <Space orientation="vertical" size={8} style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            渠道目录只承载主数据与授权标准信息，后续任务通过下列入口联动处理。
          </Typography.Text>
          <Space wrap>
            <Button onClick={onOpenSyncJobs}>同步任务</Button>
          </Space>
        </Space>
      </Card>
    </div>
  );
};
