import { Button, Descriptions, Dropdown, Select, Space, Typography } from 'antd';
import type { MenuProps, SelectProps } from 'antd';

import { OrderTagBadges } from '@/components/OrderTagBadges';
import type { OrderDisplayTag } from '@/features/orderTags';

import { UiModule05DetailPanel } from './UiModule05DetailPanel';
import './UiModule05OrderReviewRightPanelTemplate.css';

type DetailSelectOption = NonNullable<SelectProps['options']>;

type UiModule05OrderReviewRightPanelTemplateProps = {
  detailError: unknown;
  detailLoadedId: string;
  orderId?: string;
  platformOrderId?: string;
  platformText?: string;
  shopId?: string;
  receiverName?: string;
  receiverPhone?: string;
  logisticsCompany?: string;
  warehouseId?: string;
  totalAmountText: string;
  courierOptions: DetailSelectOption;
  warehouseOptions: DetailSelectOption;
  logisticsUpdating: boolean;
  warehouseUpdating: boolean;
  detailSelectDisabled: boolean;
  orderTagsApplying: boolean;
  markTags: OrderDisplayTag[];
  markMenuItems: NonNullable<MenuProps['items']>;
  onMarkSelect: (key: string) => void;
  onLogisticsChange: (value: string) => void;
  onWarehouseChange: (value: string) => void;
};

export const UiModule05OrderReviewRightPanelTemplate = ({
  detailError,
  detailLoadedId,
  orderId,
  platformOrderId,
  platformText,
  shopId,
  receiverName,
  receiverPhone,
  logisticsCompany,
  warehouseId,
  totalAmountText,
  courierOptions,
  warehouseOptions,
  logisticsUpdating,
  warehouseUpdating,
  detailSelectDisabled,
  orderTagsApplying,
  markTags,
  markMenuItems,
  onMarkSelect,
  onLogisticsChange,
  onWarehouseChange,
}: UiModule05OrderReviewRightPanelTemplateProps) => (
  <div className="ui-module-05-order-review-right-panel">
    <UiModule05DetailPanel
      panels={[
        {
          key: 'marks',
          label: '订单记号',
          children: (
              <div className="ui-module-05-order-mark-inline">
                <div className="ui-module-05-order-mark-inline-tags">
                  <OrderTagBadges tags={markTags} emptyText="-" />
                </div>
              <Dropdown
                trigger={['click']}
                menu={{
                  items: markMenuItems,
                  onClick: ({ key }) => {
                    if (key === '__empty__') return;
                    onMarkSelect(String(key ?? ''));
                  },
                }}
                disabled={detailSelectDisabled || orderTagsApplying}
              >
                <Button size="small" loading={orderTagsApplying}>
                  添加
                </Button>
              </Dropdown>
            </div>
          ),
        },
        {
          key: 'detail',
          label: '订单详情',
          children: (
            <Space direction="vertical" size={8} style={{ width: '100%' }}>
              {detailError ? <Typography.Text type="danger">详情加载失败</Typography.Text> : null}
              <Descriptions size="small" column={1} bordered className="ui-module-05-order-review-descriptions">
                <Descriptions.Item label="订单号">{orderId ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="平台单号">{platformOrderId ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="平台">{platformText ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="店铺">{shopId ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="收件人">{receiverName ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="手机号">{receiverPhone ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="物流公司">
                  <Select
                    size="small"
                    className="ui-module-05-order-review-inline-select"
                    placeholder="选择物流公司"
                    options={courierOptions}
                    value={logisticsCompany}
                    loading={logisticsUpdating}
                    showSearch
                    optionFilterProp="label"
                    disabled={detailSelectDisabled || warehouseUpdating || logisticsUpdating || orderTagsApplying}
                    onChange={(value) => onLogisticsChange(String(value ?? ''))}
                  />
                </Descriptions.Item>
                <Descriptions.Item label="发货仓库">
                  <Select
                    size="small"
                    className="ui-module-05-order-review-inline-select"
                    placeholder="选择发货仓库"
                    options={warehouseOptions}
                    value={warehouseId}
                    loading={warehouseUpdating}
                    showSearch
                    optionFilterProp="label"
                    disabled={detailSelectDisabled || warehouseUpdating || logisticsUpdating || orderTagsApplying}
                    onChange={(value) => onWarehouseChange(String(value ?? ''))}
                  />
                </Descriptions.Item>
                <Descriptions.Item label="应收合计">{totalAmountText}</Descriptions.Item>
              </Descriptions>
            </Space>
          ),
        },
      ]}
    />

    <span data-testid="order-review-detail-loaded-id" style={{ display: 'none' }}>
      {detailLoadedId}
    </span>
  </div>
);
