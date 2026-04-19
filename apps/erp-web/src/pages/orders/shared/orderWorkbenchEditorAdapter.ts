import type { OrderEditRequest } from '@/api/orders';
import {
  createWorkbenchRecordLineItemDraft,
  type WorkbenchOrderRecordDraft,
} from '@/components/page-templates/workbench/types';

type OrderDetailPackageLike = {
  order?: Record<string, unknown> | null;
  items?: Record<string, unknown>[] | null;
};

const readText = (value: unknown) => String(value ?? '').trim();

const readNumber = (value: unknown, fallback: number) => {
  const normalizedValue = Number(value);
  if (!Number.isFinite(normalizedValue)) {
    return fallback;
  }
  return Math.trunc(normalizedValue);
};

export const validateWorkbenchOrderRecordDraft = (draft: WorkbenchOrderRecordDraft): string[] => {
  const messages: string[] = [];

  if (!readText(draft.receiverName)) {
    messages.push('收件人不能为空');
  }
  if (!readText(draft.receiverPhone)) {
    messages.push('手机号不能为空');
  }
  if (!draft.items.length) {
    messages.push('至少保留 1 行货品明细');
  }

  draft.items.forEach((item, index) => {
    const rowLabel = `第 ${index + 1} 行`;
    if (!readText(item.title)) {
      messages.push(`${rowLabel}货品标题不能为空`);
    }
    if (readNumber(item.quantity, 0) <= 0) {
      messages.push(`${rowLabel}货品数量必须大于 0`);
    }
    if (readNumber(item.priceCent, 0) < 0) {
      messages.push(`${rowLabel}货品单价不能小于 0`);
    }
  });

  return messages;
};

export const buildWorkbenchOrderRecordDraft = (
  detailPackage: OrderDetailPackageLike,
  fallbackOrderId?: string,
): WorkbenchOrderRecordDraft => {
  const order = detailPackage.order ?? {};
  const orderId = readText(order.id) || readText(fallbackOrderId);

  return {
    id: orderId,
    orderNo: orderId,
    platformOrderId: readText(order.platform_order_id) || undefined,
    platform: readText(order.platform) || undefined,
    shopId: readText(order.shop_id) || undefined,
    receiverName: readText(order.receiver_name),
    receiverPhone: readText(order.receiver_phone),
    shipWarehouseId: readText(order.ship_warehouse_id) || undefined,
    logisticsCompany: readText(order.logistics_company) || undefined,
    buyerMessage: readText(order.buyer_message) || undefined,
    sellerRemark: readText(order.seller_remark) || undefined,
    items: (detailPackage.items ?? []).map((item, index) =>
      createWorkbenchRecordLineItemDraft({
        draftKey: readText(item.id) || `${orderId || 'order'}-line-${index + 1}`,
        id: readText(item.id) || undefined,
        platformSkuId: readText(item.platform_sku_id) || null,
        outerSkuId: readText(item.outer_sku_id) || null,
        title: readText(item.title) || null,
        quantity: readNumber(item.quantity, 1),
        priceCent: readNumber(item.price_cent, 0),
      }),
    ),
  };
};

export const buildWorkbenchOrderUpdatePayload = (draft: WorkbenchOrderRecordDraft): OrderEditRequest => ({
  receiver_name: readText(draft.receiverName) || null,
  receiver_phone: readText(draft.receiverPhone) || null,
  ship_warehouse_id: readText(draft.shipWarehouseId) || null,
  logistics_company: readText(draft.logisticsCompany) || null,
  buyer_message: readText(draft.buyerMessage) || null,
  seller_remark: readText(draft.sellerRemark) || null,
  items: draft.items.map((item, index) => ({
    line_id: readText(item.id) || `${draft.id || 'order'}-LINE-${index + 1}`,
    sku_id: readText(item.platformSkuId) || null,
    title: readText(item.title) || null,
    qty: Math.max(1, readNumber(item.quantity, 1)),
    price_cent: Math.max(0, readNumber(item.priceCent, 0)),
  })),
});
