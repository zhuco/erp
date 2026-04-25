import type { CellClickedEvent } from 'ag-grid-community';
import { Button, Card, Descriptions, Input, Space, Tag, Typography } from 'antd';

import type { Channel } from '@/api/channels';
import {
  WorkbenchLeftPanelTemplate,
  type WorkbenchLeftRenderRow,
  type WorkbenchLeftStatsItem,
} from '@/components/page-templates';
import { PlatformTag } from '@/components/PlatformTag';

import {
  resolveChannelAuthorizationEnabled,
  resolveChannelPlatformLabel,
  resolveChannelStatusLabel,
  resolveChannelTypeLabel,
  type ChannelDirectoryLeftFilters,
} from './channelDirectoryModel';

type ChannelDirectoryLeftPanelProps = {
  leftTab: 'filter' | 'screen';
  onLeftTabChange: (key: 'filter' | 'screen') => void;
  leftRowsForRender: WorkbenchLeftRenderRow[];
  onLeftRowClick: (event: CellClickedEvent<WorkbenchLeftRenderRow>) => void;
  hasManualFilterActive: boolean;
  filters: ChannelDirectoryLeftFilters;
  summaryItems: WorkbenchLeftStatsItem[];
  pieBackground: string;
  totalCount: number;
  isFetching: boolean;
  onPatchFilters: (patch: Partial<ChannelDirectoryLeftFilters>) => void;
  onResetFilters: () => void;
};

type ChannelDirectoryRightPanelProps = {
  activeChannel: Channel | null;
  warehouseMap: Map<string, string>;
  onOpenServicePurchase: () => void;
  onOpenAuthorize: () => void;
};

export const ChannelDirectoryLeftPanel = ({
  leftTab,
  onLeftTabChange,
  leftRowsForRender,
  onLeftRowClick,
  hasManualFilterActive,
  filters,
  summaryItems,
  pieBackground,
  totalCount,
  isFetching,
  onPatchFilters,
  onResetFilters,
}: ChannelDirectoryLeftPanelProps) => (
  <WorkbenchLeftPanelTemplate
    className="channel-directory-left-panel"
    dataTestId="channel-directory-left-panel"
    leftTab={leftTab}
    onLeftTabChange={onLeftTabChange}
    leftRowsForRender={leftRowsForRender}
    onLeftRowClick={onLeftRowClick}
    leftStatsItems={summaryItems}
    leftStatsPieBackground={pieBackground}
    globalOrderTotal={totalCount}
    statsScopeText={
      isFetching
        ? '统计更新中...'
        : hasManualFilterActive
          ? '口径：当前查询结果与左侧筛选后的渠道'
          : '口径：当前查询结果中的全部销售渠道'
    }
    screenContent={
      <Space direction="vertical" size={10} style={{ width: '100%' }}>
        <div className="channel-directory-screen-field">
          <Typography.Text type="secondary">部门</Typography.Text>
          <Input
            size="small"
            allowClear
            value={filters.departmentKeyword}
            placeholder="筛选部门"
            onChange={(event) => onPatchFilters({ departmentKeyword: event.target.value })}
          />
        </div>
        <div className="channel-directory-screen-field">
          <Typography.Text type="secondary">负责人</Typography.Text>
          <Input
            size="small"
            allowClear
            value={filters.ownerKeyword}
            placeholder="筛选负责人"
            onChange={(event) => onPatchFilters({ ownerKeyword: event.target.value })}
          />
        </div>
        <div className="channel-directory-screen-field">
          <Typography.Text type="secondary">联系人 / 电话 / 地址</Typography.Text>
          <Input
            size="small"
            allowClear
            value={filters.contactKeyword}
            placeholder="筛选联系人信息"
            onChange={(event) => onPatchFilters({ contactKeyword: event.target.value })}
          />
        </div>
        <Button size="small" onClick={onResetFilters}>
          重置左侧筛选
        </Button>
      </Space>
    }
  />
);

export const ChannelDirectoryRightPanel = ({
  activeChannel,
  warehouseMap,
  onOpenServicePurchase,
  onOpenAuthorize,
}: ChannelDirectoryRightPanelProps) => {
  if (!activeChannel) {
    return (
      <div className="channel-directory-right-panel">
        <Card size="small" title="渠道详情">
          <Typography.Text type="secondary">
            请选择一条渠道记录查看详情、服务入口与授权动作。
          </Typography.Text>
        </Card>
      </div>
    );
  }

  const authorizeEnabled = resolveChannelAuthorizationEnabled(activeChannel);

  return (
    <div className="channel-directory-right-panel">
      <Card size="small" title="渠道详情">
        <Descriptions column={1} size="small">
          <Descriptions.Item label="渠道名称">{activeChannel.name}</Descriptions.Item>
          <Descriptions.Item label="渠道类型">
            <Tag color={activeChannel.channel_type === 'OFFLINE' ? 'green' : 'blue'}>
              {resolveChannelTypeLabel(activeChannel.channel_type)}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="平台">
            {activeChannel.platform ? (
              <PlatformTag platform={activeChannel.platform} />
            ) : (
              <Tag>线下渠道</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="联系人">{activeChannel.contact_name || '-'}</Descriptions.Item>
          <Descriptions.Item label="电话">{activeChannel.contact_phone || '-'}</Descriptions.Item>
          <Descriptions.Item label="地址">{activeChannel.contact_address || '-'}</Descriptions.Item>
          <Descriptions.Item label="部门">{activeChannel.department || '-'}</Descriptions.Item>
          <Descriptions.Item label="负责人">{activeChannel.owner || '-'}</Descriptions.Item>
          <Descriptions.Item label="默认发货仓库">
            {activeChannel.default_warehouse_id
              ? warehouseMap.get(activeChannel.default_warehouse_id) ||
                activeChannel.default_warehouse_id
              : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="平台抽佣公式">
            {activeChannel.commission_formula || '-'}
          </Descriptions.Item>
          <Descriptions.Item label="手工销售单">
            {activeChannel.allow_manual_order ? <Tag color="green">允许</Tag> : <Tag>关闭</Tag>}
          </Descriptions.Item>
          <Descriptions.Item label="手工售后单">
            {activeChannel.allow_manual_after_sales ? (
              <Tag color="green">允许</Tag>
            ) : (
              <Tag>关闭</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="状态">
            <Tag color={activeChannel.status === 'ACTIVE' ? 'green' : 'default'}>
              {resolveChannelStatusLabel(activeChannel.status)}
            </Tag>
          </Descriptions.Item>
        </Descriptions>
      </Card>

      <Card size="small" title="渠道服务">
        <Space direction="vertical" size={10} style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            渠道目录只承载统一工作台交互与主数据详情，具体服务订购和授权动作从这里进入。
          </Typography.Text>
          <Space wrap>
            <Button type="primary" onClick={onOpenServicePurchase}>
              去订购
            </Button>
            <Button disabled={!authorizeEnabled} onClick={onOpenAuthorize}>
              去授权
            </Button>
          </Space>
          {!authorizeEnabled ? (
            <Typography.Text type="secondary">
              线下渠道无需授权，可直接维护联系人、仓库和手工建单规则。
            </Typography.Text>
          ) : null}
          <Typography.Text type="secondary">
            当前平台：{resolveChannelPlatformLabel(activeChannel.platform)}
          </Typography.Text>
        </Space>
      </Card>
    </div>
  );
};
