import type { MenuProps, SelectProps } from 'antd';
import { useMemo } from 'react';

import { UiModule05OrderReviewRightPanelTemplate } from '@/components/ui-modules';

import { money, platformLabel } from './helpers';

type DetailOrder = {
  id?: string;
  platform_order_id?: string;
  platform?: string;
  shop_id?: string;
  receiver_name?: string;
  receiver_phone?: string;
  total_amount_cent?: number | null;
  logistics_company?: string;
  ship_warehouse_id?: string;
};

type DetailSelectOption = NonNullable<SelectProps['options']>;

type OrderReviewDetailPanelProps = {
  detailOrder: DetailOrder | null;
  error: unknown;
  detailLoadedId: string;
  warehouseOptions: DetailSelectOption;
  courierOptions: DetailSelectOption;
  detailWarehouseId?: string;
  detailLogisticsCompany?: string;
  warehouseUpdating: boolean;
  logisticsUpdating: boolean;
  onWarehouseChange: (value: string) => void;
  onLogisticsChange: (value: string) => void;
  orderTagOptions: DetailSelectOption;
  currentOrderTags: string[];
  orderTagsApplying: boolean;
  onApplyOrderTags: (tags: string[]) => Promise<boolean>;
};

export const OrderReviewDetailPanel = ({
  detailOrder,
  error,
  detailLoadedId,
  warehouseOptions,
  courierOptions,
  detailWarehouseId,
  detailLogisticsCompany,
  warehouseUpdating,
  logisticsUpdating,
  onWarehouseChange,
  onLogisticsChange,
  orderTagOptions,
  currentOrderTags,
  orderTagsApplying,
  onApplyOrderTags,
}: OrderReviewDetailPanelProps) => {
  const hasActiveOrder = Boolean(String(detailOrder?.id ?? detailLoadedId).trim());
  const selectDisabled = !hasActiveOrder;

  const normalizedOrderTags = useMemo(
    () => Array.from(new Set(currentOrderTags.map((item) => item.trim()).filter(Boolean))),
    [currentOrderTags],
  );

  const tagMenuItems = useMemo<NonNullable<MenuProps['items']>>(
    () => {
      const items = orderTagOptions
        .map((option) => {
          const key = String(option?.value ?? '').trim();
          if (!key) return null;
          return {
            key,
            label: String(option?.label ?? key).trim() || key,
          };
        })
        .filter((item): item is { key: string; label: string } => item != null);
      if (items.length > 0) {
        return items;
      }
      return [{ key: '__empty__', label: '暂无可选记号', disabled: true }];
    },
    [orderTagOptions],
  );

  const handleAddTag = async (tagValue: string) => {
    const normalized = tagValue.trim();
    if (!normalized || selectDisabled || orderTagsApplying) {
      return;
    }
    await onApplyOrderTags([...normalizedOrderTags, normalized]);
  };

  return (
    <UiModule05OrderReviewRightPanelTemplate
      detailError={error}
      detailLoadedId={detailLoadedId}
      orderId={detailOrder?.id}
      platformOrderId={detailOrder?.platform_order_id}
      platformText={detailOrder?.platform ? platformLabel(detailOrder.platform) : '-'}
      shopId={detailOrder?.shop_id}
      receiverName={detailOrder?.receiver_name}
      receiverPhone={detailOrder?.receiver_phone}
      logisticsCompany={detailLogisticsCompany}
      warehouseId={detailWarehouseId}
      totalAmountText={money(detailOrder?.total_amount_cent ?? null)}
      courierOptions={courierOptions}
      warehouseOptions={warehouseOptions}
      logisticsUpdating={logisticsUpdating}
      warehouseUpdating={warehouseUpdating}
      detailSelectDisabled={selectDisabled}
      orderTagsApplying={orderTagsApplying}
      markTags={normalizedOrderTags}
      markMenuItems={tagMenuItems}
      onMarkSelect={(key) => {
        void handleAddTag(key);
      }}
      onLogisticsChange={onLogisticsChange}
      onWarehouseChange={onWarehouseChange}
    />
  );
};