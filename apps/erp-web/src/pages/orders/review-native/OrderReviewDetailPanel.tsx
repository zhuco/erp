import type { SelectProps } from 'antd';
import { useMemo } from 'react';

import type { OrderTagDefinition } from '@/api/orderTagDefinitions';
import { WorkbenchDetailPanelTemplate } from '@/components/page-templates';
import {
  buildOrderEditableMarkMenuItems,
  resolveBuiltinOrderTag,
  resolveOrderDisplayTags,
  type OrderDisplayTag,
} from '@/features/orderTags';

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
  mapping_missing?: boolean;
  exception_flags?: string[];
  freeze_reason?: string;
  presale?: boolean;
  refund_state?: string | null;
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
  orderTagOptions?: DetailSelectOption;
  orderTagDefinitions?: OrderTagDefinition[];
  currentOrderTags: string[];
  orderTagsApplying: boolean;
  onApplyOrderTags: (tags: string[]) => Promise<boolean>;
  onCreateOrderTagDefinition?: (name: string) => Promise<boolean> | boolean;
  onDeleteOrderTagDefinition?: (tagId: string) => Promise<boolean> | boolean;
  onRefreshOrderTagDefinitions?: () => Promise<void> | void;
  displayOrderTags?: OrderDisplayTag[];
};

const normalizeTag = (value: unknown) => String(value ?? '').trim();

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
  orderTagDefinitions,
  currentOrderTags,
  orderTagsApplying,
  onApplyOrderTags,
  onCreateOrderTagDefinition,
  onDeleteOrderTagDefinition,
  onRefreshOrderTagDefinitions,
  displayOrderTags,
}: OrderReviewDetailPanelProps) => {
  const hasActiveOrder = Boolean(String(detailOrder?.id ?? detailLoadedId).trim());
  const selectDisabled = !hasActiveOrder;

  const normalizedOrderTags = useMemo(
    () => Array.from(new Set(currentOrderTags.map((item) => normalizeTag(item)).filter(Boolean))),
    [currentOrderTags],
  );

  const resolvedOrderTagOptions = useMemo<DetailSelectOption>(() => {
    if (orderTagOptions && orderTagOptions.length > 0) {
      return orderTagOptions;
    }
    return (orderTagDefinitions ?? []).map((tag) => ({
      value: tag.name,
      label: tag.name,
    }));
  }, [orderTagDefinitions, orderTagOptions]);

  const resolvedDisplayOrderTags = useMemo(
    () =>
      displayOrderTags ??
      resolveOrderDisplayTags({
        ...(detailOrder ?? {}),
        tags: normalizedOrderTags,
      }),
    [detailOrder, displayOrderTags, normalizedOrderTags],
  );

  const tagMenuItems = useMemo(
    () => buildOrderEditableMarkMenuItems(resolvedOrderTagOptions ?? [], normalizedOrderTags),
    [normalizedOrderTags, resolvedOrderTagOptions],
  );

  const markLibraryItems = useMemo(() => {
    const seenLabels = new Set<string>();

    return (orderTagDefinitions ?? [])
      .map((definition) => ({
        id: String(definition.tag_id ?? '').trim(),
        label: String(definition.name ?? '').trim(),
        color: definition.color ?? null,
      }))
      .filter((item) => {
        const normalizedLabel = item.label.toUpperCase();
        if (!item.id || !item.label || resolveBuiltinOrderTag(item.label) || seenLabels.has(normalizedLabel)) {
          return false;
        }
        seenLabels.add(normalizedLabel);
        return true;
      });
  }, [orderTagDefinitions]);

  const handleAddTag = async (tagValue: string) => {
    const normalized = normalizeTag(tagValue);
    if (!normalized || selectDisabled || orderTagsApplying) {
      return;
    }
    if (normalizedOrderTags.some((item) => item.toUpperCase() === normalized.toUpperCase())) {
      return;
    }
    await onApplyOrderTags([...normalizedOrderTags, normalized]);
  };

  const handleRemoveTag = async (tagValue: string) => {
    const normalized = normalizeTag(tagValue);
    if (!normalized || selectDisabled || orderTagsApplying) {
      return false;
    }
    const nextTags = normalizedOrderTags.filter((item) => item.toUpperCase() !== normalized.toUpperCase());
    return onApplyOrderTags(nextTags);
  };

  return (
    <WorkbenchDetailPanelTemplate
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
      markTags={resolvedDisplayOrderTags}
      markMenuItems={tagMenuItems}
      onMarkSelect={(key) => {
        void handleAddTag(key);
      }}
      currentMarkValues={normalizedOrderTags}
      markLibraryItems={markLibraryItems}
      onMarkRemove={(key) => handleRemoveTag(key)}
      onMarkCreate={onCreateOrderTagDefinition}
      onMarkDefinitionDelete={onDeleteOrderTagDefinition}
      onMarkLibraryRefresh={onRefreshOrderTagDefinitions}
      onLogisticsChange={onLogisticsChange}
      onWarehouseChange={onWarehouseChange}
    />
  );
};
