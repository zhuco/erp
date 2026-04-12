import dayjs from 'dayjs';

import type {
  PurchaseOrderCreatePayload,
  PurchaseOrderDetail,
  PurchaseOrderListQuery,
  PurchaseOrderListResponse,
  PurchaseOrderReceivePayload,
  PurchaseOrderReceiveResponse,
  PurchaseOrderSummary,
  PurchaseOrderUpdatePayload
} from '../purchaseOrders';
import type {
  PurchaseReceiptDetail,
  PurchaseReceiptListQuery,
  PurchaseReceiptListResponse
} from '../purchaseReceipts';
import type {
  PurchaseSuggestionListQuery,
  PurchaseSuggestionListResponse
} from '../purchaseSuggestions';
import type {
  PurchaseReturnConfirmResponse,
  PurchaseReturnCreatePayload,
  PurchaseReturnDetail,
  PurchaseReturnListQuery,
  PurchaseReturnListResponse,
  PurchaseReturnSummary
} from '../purchaseReturns';

import {
  applyPagination,
  buildPurchaseOrderDetail,
  buildPurchaseOrderTotal,
  collectPurchaseReceipts,
  mockPurchaseOrderItems,
  mockPurchaseOrderReceipts,
  mockPurchaseOrders,
  mockPurchaseReceiptItems,
  mockPurchaseReturnItems,
  mockPurchaseReturns,
  mockPurchaseSuggestions,
  mockSuppliers,
  mockWarehouses
} from './data';

export const getMockPurchaseOrderList = (
  query: PurchaseOrderListQuery
): PurchaseOrderListResponse => {
  const filtered = mockPurchaseOrders.filter((order) => {
    if (query.supplier_id && order.supplier_id !== query.supplier_id) {
      return false;
    }
    if (query.warehouse_id && order.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.status && order.status !== query.status) {
      return false;
    }
    if (query.created_from && order.created_at) {
      if (dayjs(order.created_at).isBefore(query.created_from)) {
        return false;
      }
    }
    if (query.created_to && order.created_at) {
      if (dayjs(order.created_at).isAfter(query.created_to)) {
        return false;
      }
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockPurchaseOrderDetail = (poId: string): PurchaseOrderDetail => {
  const order =
    mockPurchaseOrders.find((item) => item.po_id === poId) ??
    ({
      po_id: poId,
      supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
      supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
      warehouse_id: mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
      expected_at: null,
      remark: null,
      status: 'DRAFT',
      total_cent: 0,
      created_at: dayjs().toISOString(),
      updated_at: dayjs().toISOString()
    } as PurchaseOrderSummary);
  return buildPurchaseOrderDetail(order);
};

export const createMockPurchaseOrder = (
  payload: PurchaseOrderCreatePayload
): PurchaseOrderDetail => {
  const now = dayjs().toISOString();
  const items = payload.items.map((item) => ({
    sku_id: item.sku_id,
    sku_code: item.sku_id,
    qty: item.qty,
    price_cent: item.price_cent,
    received_qty: 0
  }));
  const totalCent = buildPurchaseOrderTotal(items);
  const supplier = mockSuppliers.find((item) => item.supplier_id === payload.supplier_id);
  const order: PurchaseOrderSummary = {
    po_id: `PO-${Date.now()}`,
    supplier_id: payload.supplier_id,
    supplier_name: supplier?.name ?? null,
    warehouse_id: payload.warehouse_id,
    expected_at: payload.expected_at ?? null,
    remark: payload.remark ?? null,
    status: 'DRAFT',
    total_cent: totalCent,
    created_at: now,
    updated_at: now
  };
  mockPurchaseOrders.unshift(order);
  mockPurchaseOrderItems.set(order.po_id, items);
  mockPurchaseOrderReceipts.set(order.po_id, []);
  return {
    order,
    items,
    receipts: []
  };
};

export const updateMockPurchaseOrder = (
  poId: string,
  payload: PurchaseOrderUpdatePayload
): PurchaseOrderDetail => {
  const now = dayjs().toISOString();
  const index = mockPurchaseOrders.findIndex((item) => item.po_id === poId);
  const base =
    index >= 0
      ? mockPurchaseOrders[index]
      : ({
          po_id: poId,
          supplier_id: payload.supplier_id ?? 'SUP-1',
          supplier_name: null,
          warehouse_id: payload.warehouse_id ?? mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
          expected_at: payload.expected_at ?? null,
          remark: payload.remark ?? null,
          status: 'DRAFT',
          total_cent: 0,
          created_at: now,
          updated_at: now
        } as PurchaseOrderSummary);
  const supplier = mockSuppliers.find((item) => item.supplier_id === payload.supplier_id);
  const items = payload.items
    ? payload.items.map((item) => ({
        sku_id: item.sku_id,
        sku_code: item.sku_id,
        qty: item.qty,
        price_cent: item.price_cent,
        received_qty: 0
      }))
    : mockPurchaseOrderItems.get(poId) ?? [];
  const updated: PurchaseOrderSummary = {
    ...base,
    supplier_id: payload.supplier_id ?? base.supplier_id,
    supplier_name: supplier?.name ?? base.supplier_name ?? null,
    warehouse_id: payload.warehouse_id ?? base.warehouse_id ?? null,
    expected_at: payload.expected_at ?? base.expected_at ?? null,
    remark: payload.remark ?? base.remark ?? null,
    total_cent: buildPurchaseOrderTotal(items),
    updated_at: now
  };
  if (index >= 0) {
    mockPurchaseOrders[index] = updated;
  } else {
    mockPurchaseOrders.unshift(updated);
  }
  mockPurchaseOrderItems.set(poId, items);
  return buildPurchaseOrderDetail(updated);
};

export const deleteMockPurchaseOrder = (poId: string): PurchaseOrderSummary => {
  const index = mockPurchaseOrders.findIndex((item) => item.po_id === poId);
  if (index >= 0) {
    const [removed] = mockPurchaseOrders.splice(index, 1);
    mockPurchaseOrderItems.delete(poId);
    mockPurchaseOrderReceipts.delete(poId);
    return removed;
  }
  return {
    po_id: poId,
    supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
    supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
    warehouse_id: mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
    expected_at: null,
    remark: null,
    status: 'DRAFT',
    total_cent: 0,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };
};

export const sendMockPurchaseOrder = (poId: string): PurchaseOrderSummary => {
  const order = mockPurchaseOrders.find((item) => item.po_id === poId);
  const updatedAt = dayjs().toISOString();
  if (!order) {
    return {
      po_id: poId,
      supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
      supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
      warehouse_id: mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
      expected_at: null,
      remark: null,
      status: 'SENT',
      total_cent: 0,
      created_at: updatedAt,
      updated_at: updatedAt
    };
  }
  order.status = 'SENT';
  order.updated_at = updatedAt;
  return order;
};

export const confirmMockPurchaseOrder = (poId: string): PurchaseOrderSummary => {
  const order = mockPurchaseOrders.find((item) => item.po_id === poId);
  const updatedAt = dayjs().toISOString();
  if (!order) {
    return {
      po_id: poId,
      supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
      supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
      warehouse_id: mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
      expected_at: null,
      remark: null,
      status: 'CONFIRMED',
      total_cent: 0,
      created_at: updatedAt,
      updated_at: updatedAt
    };
  }
  order.status = 'CONFIRMED';
  order.updated_at = updatedAt;
  return order;
};

export const receiveMockPurchaseOrder = (
  poId: string,
  payload: PurchaseOrderReceivePayload
): PurchaseOrderReceiveResponse => {
  const now = dayjs().toISOString();
  const order =
    mockPurchaseOrders.find((item) => item.po_id === poId) ??
    ({
      po_id: poId,
      supplier_id: mockSuppliers[0]?.supplier_id ?? 'SUP-1',
      supplier_name: mockSuppliers[0]?.name ?? 'Supplier',
      warehouse_id: mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
      expected_at: null,
      remark: null,
      status: 'RECEIVING',
      total_cent: 0,
      created_at: now,
      updated_at: now
    } as PurchaseOrderSummary);
  const items = mockPurchaseOrderItems.get(poId) ?? [];
  payload.items.forEach((item) => {
    const matched = items.find((entry) => entry.sku_id === item.sku_id);
    if (matched) {
      matched.received_qty = Math.min(matched.qty, matched.received_qty + item.qty);
    }
  });
  const allReceived = items.length > 0 && items.every((item) => item.received_qty >= item.qty);
  order.status = allReceived ? 'DONE' : 'RECEIVING';
  order.updated_at = now;
  mockPurchaseOrderItems.set(poId, items);
  const receipt = {
    receipt_id: `REC-${Date.now()}`,
    po_id: poId,
    supplier_id: order.supplier_id,
    supplier_name: order.supplier_name ?? null,
    warehouse_id: payload.warehouse_id,
    status: 'CONFIRMED',
    confirmed_at: now,
    created_at: now
  };
  const receipts = mockPurchaseOrderReceipts.get(poId) ?? [];
  receipts.unshift({
    receipt_id: receipt.receipt_id,
    po_id: receipt.po_id,
    supplier_id: order.supplier_id,
    supplier_name: order.supplier_name ?? null,
    warehouse_id: receipt.warehouse_id,
    status: receipt.status,
    confirmed_at: receipt.confirmed_at,
    created_at: receipt.created_at
  });
  mockPurchaseOrderReceipts.set(poId, receipts);
  mockPurchaseReceiptItems.set(
    receipt.receipt_id,
    payload.items.map((item) => ({ sku_id: item.sku_id, qty: item.qty }))
  );
  return {
    order,
    receipt,
    receipt_items: payload.items.map((item) => ({ sku_id: item.sku_id, qty: item.qty })),
    inbound_doc_id: `INB-${Date.now()}`,
    ap_doc_id: `AP-${Date.now()}`
  };
};

export const getMockPurchaseReceiptList = (
  query: PurchaseReceiptListQuery
): PurchaseReceiptListResponse => {
  const filtered = collectPurchaseReceipts().filter((receipt) => {
    if (query.supplier_id && receipt.supplier_id !== query.supplier_id) {
      return false;
    }
    if (query.warehouse_id && receipt.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.status && receipt.status !== query.status) {
      return false;
    }
    if (query.created_from && receipt.created_at) {
      if (dayjs(receipt.created_at).isBefore(query.created_from)) {
        return false;
      }
    }
    if (query.created_to && receipt.created_at) {
      if (dayjs(receipt.created_at).isAfter(query.created_to)) {
        return false;
      }
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockPurchaseReceiptDetail = (receiptId: string): PurchaseReceiptDetail => {
  const receipts = collectPurchaseReceipts();
  const summary = receipts.find((item) => item.receipt_id === receiptId);
  const fallbackOrder = mockPurchaseOrders[0];
  const receipt = {
    receipt_id: receiptId,
    po_id: summary?.po_id ?? fallbackOrder?.po_id ?? receiptId,
    supplier_id: summary?.supplier_id ?? fallbackOrder?.supplier_id ?? 'SUP-1',
    supplier_name: summary?.supplier_name ?? fallbackOrder?.supplier_name ?? null,
    warehouse_id: summary?.warehouse_id ?? fallbackOrder?.warehouse_id ?? 'WH-DEFAULT',
    status: summary?.status ?? 'CONFIRMED',
    confirmed_at: summary?.confirmed_at ?? null,
    created_at: summary?.created_at ?? dayjs().toISOString()
  };
  return {
    receipt,
    items: mockPurchaseReceiptItems.get(receiptId) ?? []
  };
};

export const getMockPurchaseSuggestions = (
  query: PurchaseSuggestionListQuery
): PurchaseSuggestionListResponse => {
  const filtered = mockPurchaseSuggestions.filter((item) => {
    if (query.sku_id && item.sku_id !== query.sku_id) {
      return false;
    }
    if (query.reason && item.reason !== query.reason) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockPurchaseReturnList = (
  query: PurchaseReturnListQuery
): PurchaseReturnListResponse => {
  const filtered = mockPurchaseReturns.filter((item) => {
    if (query.supplier_id && item.supplier_id !== query.supplier_id) {
      return false;
    }
    if (query.warehouse_id && item.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.created_from && item.created_at) {
      if (dayjs(item.created_at).isBefore(query.created_from)) {
        return false;
      }
    }
    if (query.created_to && item.created_at) {
      if (dayjs(item.created_at).isAfter(query.created_to)) {
        return false;
      }
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockPurchaseReturnDetail = (returnId: string): PurchaseReturnDetail => {
  const summary =
    mockPurchaseReturns.find((item) => item.return_id === returnId) ??
    ({
      return_id: returnId,
      po_id: mockPurchaseOrders[0]?.po_id ?? 'PO-1001',
      supplier_id: mockPurchaseOrders[0]?.supplier_id ?? 'SUP-1',
      supplier_name: mockPurchaseOrders[0]?.supplier_name ?? 'Supplier',
      warehouse_id: 'WH-DEFAULT',
      status: 'DRAFT',
      confirmed_at: null,
      created_at: dayjs().toISOString()
    } as PurchaseReturnSummary);
  const items = mockPurchaseReturnItems.get(returnId) ?? [];
  return {
    purchase_return: summary,
    items
  };
};

export const createMockPurchaseReturn = (
  payload: PurchaseReturnCreatePayload
): PurchaseReturnDetail => {
  const now = dayjs().toISOString();
  const order = mockPurchaseOrders.find((item) => item.po_id === payload.po_id);
  const summary: PurchaseReturnSummary = {
    return_id: `RET-${Date.now()}`,
    po_id: payload.po_id,
    supplier_id: order?.supplier_id ?? mockSuppliers[0]?.supplier_id ?? 'SUP-1',
    supplier_name: order?.supplier_name ?? mockSuppliers[0]?.name ?? 'Supplier',
    warehouse_id: payload.warehouse_id,
    status: 'DRAFT',
    confirmed_at: null,
    created_at: now
  };
  mockPurchaseReturns.unshift(summary);
  mockPurchaseReturnItems.set(summary.return_id, payload.items);
  return {
    purchase_return: summary,
    items: payload.items
  };
};

export const confirmMockPurchaseReturn = (returnId: string): PurchaseReturnConfirmResponse => {
  const now = dayjs().toISOString();
  const summary =
    mockPurchaseReturns.find((item) => item.return_id === returnId) ??
    ({
      return_id: returnId,
      po_id: mockPurchaseOrders[0]?.po_id ?? 'PO-1001',
      supplier_id: mockPurchaseOrders[0]?.supplier_id ?? 'SUP-1',
      supplier_name: mockPurchaseOrders[0]?.supplier_name ?? 'Supplier',
      warehouse_id: 'WH-DEFAULT',
      status: 'DRAFT',
      confirmed_at: null,
      created_at: now
    } as PurchaseReturnSummary);
  summary.status = 'DONE';
  summary.confirmed_at = now;
  return {
    purchase_return: summary,
    outbound_doc_id: `OUT-${Date.now()}`
  };
};
