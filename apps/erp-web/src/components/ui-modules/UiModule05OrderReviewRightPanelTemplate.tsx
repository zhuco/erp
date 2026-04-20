import { DeleteOutlined, PlusOutlined, ReloadOutlined, TagsOutlined } from '@ant-design/icons';
import { Button, Descriptions, Input, List, Popover, Select, Space, Typography } from 'antd';
import type { MenuProps, SelectProps } from 'antd';
import { useMemo, useState } from 'react';

import { OrderTagBadges } from '@/components/OrderTagBadges';
import { resolveBuiltinOrderTag, type OrderDisplayTag } from '@/features/orderTags';

import { UiModule05DetailPanel } from './UiModule05DetailPanel';
import './UiModule05OrderReviewRightPanelTemplate.css';

type DetailSelectOption = NonNullable<SelectProps['options']>;

export type OrderMarkLibraryItem = {
  id: string;
  label: string;
  color?: string | null;
  deletable?: boolean;
};

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
  markMenuItems?: NonNullable<MenuProps['items']>;
  onMarkSelect?: (key: string) => void;
  currentMarkValues?: string[];
  markLibraryItems?: OrderMarkLibraryItem[];
  onMarkRemove?: (key: string) => Promise<boolean> | boolean | void;
  onMarkCreate?: (value: string) => Promise<boolean> | boolean | void;
  onMarkDefinitionDelete?: (id: string) => Promise<boolean> | boolean | void;
  onMarkLibraryRefresh?: () => Promise<void> | void;
  onLogisticsChange: (value: string) => void;
  onWarehouseChange: (value: string) => void;
};

const normalizeText = (value: unknown) => String(value ?? '').trim();

const isActionSuccessful = (value: unknown) => value !== false;

const deriveLibraryItemsFromMenu = (
  menuItems: NonNullable<MenuProps['items']> | undefined,
): OrderMarkLibraryItem[] => {
  if (!Array.isArray(menuItems)) {
    return [];
  }

  return menuItems
    .map((item) => {
      if (!item || typeof item !== 'object') {
        return null;
      }
      const key = normalizeText('key' in item ? item.key : '');
      const label = normalizeText('label' in item ? item.label : key);
      const disabled = Boolean('disabled' in item ? item.disabled : false);
      if (!key || key === '__empty__' || !label || disabled) {
        return null;
      }
      return {
        id: key,
        label,
      } satisfies OrderMarkLibraryItem;
    })
    .filter((item): item is OrderMarkLibraryItem => item != null);
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
  currentMarkValues,
  markLibraryItems,
  onMarkRemove,
  onMarkCreate,
  onMarkDefinitionDelete,
  onMarkLibraryRefresh,
  onLogisticsChange,
  onWarehouseChange,
}: UiModule05OrderReviewRightPanelTemplateProps) => {
  const [markManagerOpen, setMarkManagerOpen] = useState(false);
  const [newMarkValue, setNewMarkValue] = useState('');
  const [markLibrarySubmitting, setMarkLibrarySubmitting] = useState(false);

  const normalizedCurrentMarkValues = useMemo(() => {
    const baseValues =
      currentMarkValues ??
      markTags
        .map((tag) => (typeof tag === 'string' ? tag : tag.name))
        .filter((tag) => tag && !resolveBuiltinOrderTag(tag));
    return Array.from(new Set(baseValues.map((item) => normalizeText(item)).filter(Boolean)));
  }, [currentMarkValues, markTags]);

  const resolvedMarkLibraryItems = useMemo(() => {
    const sourceItems =
      markLibraryItems && markLibraryItems.length > 0
        ? markLibraryItems
        : deriveLibraryItemsFromMenu(markMenuItems);
    const seenLabels = new Set<string>();

    return sourceItems.filter((item) => {
      const label = normalizeText(item.label);
      const normalizedLabel = label.toUpperCase();
      if (!label || resolveBuiltinOrderTag(label) || seenLabels.has(normalizedLabel)) {
        return false;
      }
      seenLabels.add(normalizedLabel);
      return true;
    });
  }, [markLibraryItems, markMenuItems]);

  const quickAddItems = useMemo(() => {
    const currentValueSet = new Set(normalizedCurrentMarkValues.map((item) => item.toUpperCase()));
    return resolvedMarkLibraryItems.filter((item) => !currentValueSet.has(item.label.toUpperCase()));
  }, [normalizedCurrentMarkValues, resolvedMarkLibraryItems]);

  const handleQuickAdd = async (value: string) => {
    const normalized = normalizeText(value);
    if (!normalized || detailSelectDisabled || orderTagsApplying || !onMarkSelect) {
      return;
    }
    await Promise.resolve(onMarkSelect(normalized));
  };

  const handleMarkRemove = async (value: string) => {
    const normalized = normalizeText(value);
    if (!normalized || !onMarkRemove || detailSelectDisabled || orderTagsApplying) {
      return;
    }
    await Promise.resolve(onMarkRemove(normalized));
  };

  const handleMarkCreate = async () => {
    const normalized = normalizeText(newMarkValue);
    if (!normalized || !onMarkCreate || markLibrarySubmitting) {
      return;
    }
    setMarkLibrarySubmitting(true);
    try {
      const result = await Promise.resolve(onMarkCreate(normalized));
      if (isActionSuccessful(result)) {
        setNewMarkValue('');
      }
    } finally {
      setMarkLibrarySubmitting(false);
    }
  };

  const handleMarkDefinitionDelete = async (id: string) => {
    const normalized = normalizeText(id);
    if (!normalized || !onMarkDefinitionDelete || markLibrarySubmitting) {
      return;
    }
    setMarkLibrarySubmitting(true);
    try {
      await Promise.resolve(onMarkDefinitionDelete(normalized));
    } finally {
      setMarkLibrarySubmitting(false);
    }
  };

  const handleMarkLibraryRefresh = async () => {
    if (!onMarkLibraryRefresh || markLibrarySubmitting) {
      return;
    }
    setMarkLibrarySubmitting(true);
    try {
      await Promise.resolve(onMarkLibraryRefresh());
    } finally {
      setMarkLibrarySubmitting(false);
    }
  };

  const markManagerContent = (
    <div className="ui-module-05-order-mark-manager">
      <section className="ui-module-05-order-mark-manager-section">
        <div className="ui-module-05-order-mark-manager-section-header">
          <Typography.Text strong>输入新记号</Typography.Text>
        </div>
        <div className="ui-module-05-order-mark-manager-create-row">
          <Input
            value={newMarkValue}
            maxLength={32}
            placeholder="输入新记号"
            onChange={(event) => setNewMarkValue(event.target.value)}
            onPressEnter={() => {
              void handleMarkCreate();
            }}
          />
          <Button
            type="primary"
            icon={<PlusOutlined />}
            loading={markLibrarySubmitting}
            disabled={!normalizeText(newMarkValue) || !onMarkCreate}
            onClick={() => {
              void handleMarkCreate();
            }}
          >
            新建
          </Button>
        </div>
      </section>

      <section className="ui-module-05-order-mark-manager-section">
        <div className="ui-module-05-order-mark-manager-section-header">
          <Typography.Text strong>快捷添加</Typography.Text>
        </div>
        {quickAddItems.length > 0 ? (
          <div className="ui-module-05-order-mark-manager-chip-list">
            {quickAddItems.map((item) => (
              <Button
                key={item.id}
                size="small"
                disabled={detailSelectDisabled || orderTagsApplying || !onMarkSelect}
                onClick={() => {
                  void handleQuickAdd(item.label);
                }}
              >
                {item.label}
              </Button>
            ))}
          </div>
        ) : (
          <Typography.Text type="secondary" className="ui-module-05-order-mark-manager-empty">
            暂无可快捷添加的记号
          </Typography.Text>
        )}
      </section>

      <section className="ui-module-05-order-mark-manager-section">
        <div className="ui-module-05-order-mark-manager-section-header">
          <Typography.Text strong>当前订单</Typography.Text>
        </div>
        {normalizedCurrentMarkValues.length > 0 ? (
          <div className="ui-module-05-order-mark-manager-current-list">
            {normalizedCurrentMarkValues.map((item) => (
              <div key={item} className="ui-module-05-order-mark-manager-current-item">
                <span>{item}</span>
                <Button
                  size="small"
                  type="text"
                  danger
                  icon={<DeleteOutlined />}
                  disabled={detailSelectDisabled || orderTagsApplying || !onMarkRemove}
                  onClick={() => {
                    void handleMarkRemove(item);
                  }}
                />
              </div>
            ))}
          </div>
        ) : (
          <Typography.Text type="secondary" className="ui-module-05-order-mark-manager-empty">
            当前订单暂无自定义记号
          </Typography.Text>
        )}
      </section>

      <section className="ui-module-05-order-mark-manager-section">
        <div className="ui-module-05-order-mark-manager-section-header">
          <Typography.Text strong>记号库</Typography.Text>
          <Button
            size="small"
            icon={<ReloadOutlined />}
            loading={markLibrarySubmitting}
            disabled={!onMarkLibraryRefresh}
            onClick={() => {
              void handleMarkLibraryRefresh();
            }}
          >
            刷新
          </Button>
        </div>
        <List
          size="small"
          className="ui-module-05-order-mark-manager-library"
          dataSource={resolvedMarkLibraryItems}
          locale={{ emptyText: '暂无记号定义' }}
          renderItem={(item) => (
            <List.Item
              actions={[
                <Button
                  key={`add-${item.id}`}
                  size="small"
                  type="link"
                  disabled={detailSelectDisabled || orderTagsApplying || !onMarkSelect}
                  onClick={() => {
                    void handleQuickAdd(item.label);
                  }}
                >
                  添加
                </Button>,
                <Button
                  key={`delete-${item.id}`}
                  size="small"
                  type="link"
                  danger
                  disabled={item.deletable === false || !onMarkDefinitionDelete}
                  onClick={() => {
                    void handleMarkDefinitionDelete(item.id);
                  }}
                >
                  删除
                </Button>,
              ]}
            >
              <div className="ui-module-05-order-mark-manager-library-item">
                <span
                  className="ui-module-05-order-mark-manager-library-dot"
                  style={{ background: item.color || '#d9d9d9' }}
                />
                <span>{item.label}</span>
              </div>
            </List.Item>
          )}
        />
      </section>
    </div>
  );

  return (
    <div className="ui-module-05-order-review-right-panel">
      <UiModule05DetailPanel
        panels={[
          {
            key: 'marks',
            label: '订单记号',
            children: (
              <div className="ui-module-05-order-mark-inline">
                <div className="ui-module-05-order-mark-inline-tags">
                  <OrderTagBadges tags={markTags} emptyText={null} singleLine />
                </div>
                <Popover
                  trigger={['click']}
                  placement="bottomRight"
                  open={markManagerOpen}
                  overlayClassName="ui-module-05-order-mark-manager-popover"
                  destroyTooltipOnHide
                  content={markManagerContent}
                  onOpenChange={setMarkManagerOpen}
                >
                  <Button
                    size="small"
                    icon={<TagsOutlined />}
                    className="ui-module-05-order-mark-manager-trigger"
                  >
                    管理记号
                  </Button>
                </Popover>
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
};
