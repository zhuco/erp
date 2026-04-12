import dayjs from 'dayjs';

import type { ShipOrderPayload, ShipmentListQuery, ShipmentSummary } from '../shipments';

import type { LogisticsShipmentListQuery } from './data';
import { applyPagination, mockOrders, mockShipments, pickPlatform } from './data';

export const getMockShipmentList = (query: ShipmentListQuery) => {
  const filtered = mockShipments.filter((shipment) => {
    if (query.platform && shipment.platform !== query.platform) {
      return false;
    }
    if (query.shop_id && shipment.shop_id !== query.shop_id) {
      return false;
    }
    if (query.status && shipment.status !== query.status) {
      return false;
    }
    if (query.platform_order_id && shipment.platform_order_id !== query.platform_order_id) {
      return false;
    }
    if (query.updated_from && shipment.updated_at) {
      if (dayjs(shipment.updated_at).isBefore(query.updated_from)) {
        return false;
      }
    }
    if (query.updated_to && shipment.updated_at) {
      if (dayjs(shipment.updated_at).isAfter(query.updated_to)) {
        return false;
      }
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const getMockLogisticsShipmentList = (query: LogisticsShipmentListQuery) => {
  const normalized = query.tracking_no?.trim();
  const filtered = mockShipments.filter((shipment) => {
    if (normalized && shipment.tracking_no?.includes(normalized) === false) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit).map((shipment) => ({
    shipment_id: shipment.shipment_id,
    carrier_code: shipment.carrier_code ?? null,
    tracking_no: shipment.tracking_no ?? null,
    status: shipment.status,
    weight_gram: shipment.weight_gram ?? null,
    updated_at: shipment.updated_at ?? shipment.requested_at ?? null
  }));
  return {
    items,
    total: filtered.length
  };
};

export const getMockShipmentTracking = (shipmentId: string) => {
  const shipment = mockShipments.find((item) => item.shipment_id === shipmentId);
  return {
    shipment_id: shipmentId,
    carrier_code: shipment?.carrier_code ?? null,
    tracking_no: shipment?.tracking_no ?? null,
    latest_status: shipment?.status ?? 'UNKNOWN',
    latest_description: shipment?.last_error ?? null,
    latest_at: shipment?.updated_at ?? dayjs().toISOString()
  };
};

export const requestMockShipment = (orderId: string, payload: ShipOrderPayload): ShipmentSummary => {
  const matchedOrder = mockOrders.find(
    (order) => order.id === orderId || order.platform_order_id === orderId
  );
  const platform = matchedOrder?.platform ?? pickPlatform(mockShipments.length);
  const nextShipment: ShipmentSummary = {
    shipment_id: `SHIP-${Date.now()}`,
    order_id: orderId,
    platform,
    platform_order_id: matchedOrder?.platform_order_id ?? orderId,
    shop_id: matchedOrder?.shop_id ?? `SHOP-${platform}-1`,
    warehouse_id: payload.warehouse_id,
    carrier_code: payload.carrier_code,
    tracking_no: payload.tracking_no ?? null,
    weight_gram: payload.weight_gram ?? null,
    status: 'REQUESTED',
    requested_at: dayjs().toISOString(),
    shipped_at: null,
    updated_at: dayjs().toISOString(),
    last_error: null
  };
  mockShipments.unshift(nextShipment);
  return nextShipment;
};

export const retryMockShipment = (shipment: ShipmentSummary): ShipmentSummary => {
  const updated: ShipmentSummary = {
    ...shipment,
    status: 'REQUESTED',
    last_error: null,
    updated_at: dayjs().toISOString()
  };
  const index = mockShipments.findIndex((item) => item.shipment_id === shipment.shipment_id);
  if (index >= 0) {
    mockShipments[index] = updated;
  } else {
    mockShipments.unshift(updated);
  }
  return updated;
};
