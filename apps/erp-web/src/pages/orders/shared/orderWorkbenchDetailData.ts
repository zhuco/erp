import type { ShipmentSummary } from '@/api/shipments';

type DetailPayload = Record<string, unknown> | null | undefined;

export type OrderWorkbenchPlatformOrderRow = {
  key: string;
  orderSource: string | null;
  orderType: string | null;
  orderNo: string | null;
  orderStatus: string | null;
  amountCent: number | null;
  time: string | null;
};

export type OrderWorkbenchShipmentRow = {
  key: string;
  shipmentNo: string | null;
  shipmentStatus: string | null;
  shippedAt: string | null;
  warehouseId: string | null;
  logisticsCompany: string | null;
  trackingNo: string | null;
};

export type OrderWorkbenchFeeRow = {
  key: string;
  feeName: string;
  amountCent: number | null;
  remark: string | null;
};

const asRecord = (value: unknown) => (value && typeof value === 'object' ? (value as Record<string, unknown>) : null);

const readOrderRecord = (detailPayload: DetailPayload) => asRecord(detailPayload?.order);

const readString = (record: Record<string, unknown> | null, keys: string[]) => {
  if (!record) {
    return null;
  }
  for (const key of keys) {
    const rawValue = record[key];
    if (rawValue == null) {
      continue;
    }
    const value = String(rawValue).trim();
    if (value) {
      return value;
    }
  }
  return null;
};

const readNumber = (record: Record<string, unknown> | null, keys: string[]) => {
  if (!record) {
    return null;
  }
  for (const key of keys) {
    const rawValue = record[key];
    if (typeof rawValue === 'number' && Number.isFinite(rawValue)) {
      return rawValue;
    }
    if (typeof rawValue === 'string') {
      const parsedValue = Number(rawValue);
      if (Number.isFinite(parsedValue)) {
        return parsedValue;
      }
    }
  }
  return null;
};

const readRemark = (amountCent: number | null) => (amountCent == null ? '待业务接口补充' : null);

export const resolveOrderWorkbenchDetailItems = (detailPayload: DetailPayload) => {
  const items = detailPayload?.items;
  if (!Array.isArray(items)) {
    return [] as Record<string, unknown>[];
  }
  return items;
};

export const resolveOrderWorkbenchPlatformOrderRows = (
  detailPayload: DetailPayload,
): OrderWorkbenchPlatformOrderRow[] => {
  const order = readOrderRecord(detailPayload);
  if (!order) {
    return [];
  }

  const key =
    readString(order, ['id', 'platform_order_id', 'order_no']) ??
    'platform-order';

  return [
    {
      key,
      orderSource: readString(order, ['order_source', 'sales_channel', 'platform_source']),
      orderType: readString(order, ['order_type', 'order_type_name']),
      orderNo: readString(order, ['platform_order_id', 'order_no', 'id']),
      orderStatus: readString(order, ['platform_status', 'oms_status', 'order_status', 'review_status']),
      amountCent: readNumber(order, ['total_amount_cent', 'receivable_amount_cent']),
      time: readString(order, ['order_created_at', 'created_at']),
    },
  ];
};

export const resolveOrderWorkbenchShipmentRows = (
  detailPayload: DetailPayload,
  shipments?: ShipmentSummary[] | null,
): OrderWorkbenchShipmentRow[] => {
  const normalizedShipments = Array.isArray(shipments) ? shipments : [];
  if (normalizedShipments.length > 0) {
    return normalizedShipments.map((shipment, index) => ({
      key: shipment.shipment_id || shipment.shipment_no || shipment.tracking_no || `shipment-${index + 1}`,
      shipmentNo: shipment.shipment_no ?? shipment.shipment_id ?? null,
      shipmentStatus: shipment.status ?? null,
      shippedAt: shipment.shipped_at ?? null,
      warehouseId: shipment.warehouse_id ?? null,
      logisticsCompany: shipment.carrier_code ?? null,
      trackingNo: shipment.tracking_no ?? null,
    }));
  }

  const order = readOrderRecord(detailPayload);
  const shipmentNo = readString(order, ['shipment_no']);
  const trackingNo = readString(order, ['tracking_no']);
  const logisticsCompany = readString(order, ['logistics_company']);
  const warehouseId = readString(order, ['ship_warehouse_id', 'warehouse_id']);
  const shipmentStatus = readString(order, ['shipment_status', 'oms_status']);
  const shippedAt = readString(order, ['shipped_at', 'delivery_time']);

  if (!shipmentNo && !trackingNo && !logisticsCompany && !warehouseId) {
    return [];
  }

  return [
    {
      key: shipmentNo ?? trackingNo ?? readString(order, ['id']) ?? 'shipment-fallback',
      shipmentNo,
      shipmentStatus,
      shippedAt,
      warehouseId,
      logisticsCompany,
      trackingNo,
    },
  ];
};

export const resolveOrderWorkbenchFeeRows = (
  detailPayload: DetailPayload,
): OrderWorkbenchFeeRow[] => {
  const order = readOrderRecord(detailPayload);

  const platformCommission = readNumber(order, ['platform_commission_cent', 'commission_amount_cent']);
  const estimatedPostage = readNumber(order, ['estimated_postage_cent', 'estimated_freight_cent']);
  const postage = readNumber(order, ['postage_amount_cent', 'freight_amount_cent', 'shipping_amount_cent']);
  const goodsAmount = readNumber(order, ['goods_amount_cent', 'items_amount_cent']);
  const discountAmount = readNumber(order, ['discount_amount_cent', 'promotion_amount_cent']);
  const totalAmount = readNumber(order, ['total_amount_cent', 'receivable_amount_cent']);

  return [
    {
      key: 'platformCommission',
      feeName: '平台佣金',
      amountCent: platformCommission,
      remark: readRemark(platformCommission),
    },
    {
      key: 'estimatedPostage',
      feeName: '预估运费',
      amountCent: estimatedPostage,
      remark: readRemark(estimatedPostage),
    },
    {
      key: 'postage',
      feeName: '实际运费',
      amountCent: postage,
      remark: readRemark(postage),
    },
    {
      key: 'goodsAmount',
      feeName: '商品金额',
      amountCent: goodsAmount,
      remark: readRemark(goodsAmount),
    },
    {
      key: 'discountAmount',
      feeName: '优惠金额',
      amountCent: discountAmount,
      remark: readRemark(discountAmount),
    },
    {
      key: 'totalAmount',
      feeName: '订单合计',
      amountCent: totalAmount,
      remark: readRemark(totalAmount),
    },
  ];
};
