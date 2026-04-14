import type { AfterSalesStatus, AfterSalesType, ReturnReceipt } from '@/api/afterSalesOps';
import { describe, expect, test } from 'vitest';

import type { AfterSalesActionContext } from './afterSalesActionGuards';
import { getAfterSalesActionFlags } from './afterSalesActionGuards';

const baseContext: AfterSalesActionContext = {
  status: 'OPEN',
  type: 'REFUND_ONLY',
  sla_status: undefined,
  return_receipt: undefined,
  canWrite: true
};

const createContext = (overrides: Partial<AfterSalesActionContext>): AfterSalesActionContext => ({
  ...baseContext,
  ...overrides
});

describe('afterSalesActionGuards', () => {
  test('approve/reject only available when status is OPEN and write access is granted', () => {
    expect(getAfterSalesActionFlags(baseContext).canApprove).toBe(true);
    expect(getAfterSalesActionFlags(baseContext).canReject).toBe(true);
    expect(getAfterSalesActionFlags(createContext({ status: 'APPROVED' })).canApprove).toBe(false);
    expect(getAfterSalesActionFlags(createContext({ canWrite: false })).canReject).toBe(false);
  });

  test('receive only available when status is APPROVED', () => {
    expect(getAfterSalesActionFlags(createContext({ status: 'APPROVED' })).canReceive).toBe(true);
    expect(getAfterSalesActionFlags(createContext({ status: 'OPEN' })).canReceive).toBe(false);
  });

  test('shipment creation requires APPROVED or RECEIVED status and EXCHANGE/RESEND type', () => {
    expect(
      getAfterSalesActionFlags(createContext({ status: 'APPROVED', type: 'EXCHANGE' })).canCreateShipment
    ).toBe(true);
    expect(
      getAfterSalesActionFlags(createContext({ status: 'RECEIVED', type: 'RESEND' })).canCreateShipment
    ).toBe(true);
    expect(
      getAfterSalesActionFlags(createContext({ status: 'APPROVED', type: 'REFUND_ONLY' })).canCreateShipment
    ).toBe(false);
    expect(
      getAfterSalesActionFlags(createContext({ status: 'OPEN', type: 'RESEND' })).canCreateShipment
    ).toBe(false);
  });

  test('refund is blocked for REJECTED, REFUNDED, and CLOSED statuses', () => {
    expect(getAfterSalesActionFlags(createContext({ status: 'OPEN' })).canRefund).toBe(true);
    expect(getAfterSalesActionFlags(createContext({ status: 'REJECTED' })).canRefund).toBe(false);
    expect(getAfterSalesActionFlags(createContext({ status: 'REFUNDED' })).canRefund).toBe(false);
    expect(getAfterSalesActionFlags(createContext({ status: 'CLOSED' })).canRefund).toBe(false);
  });

  test('cancel/redo/putaway buttons depend on return receipt status', () => {
    const receivingReceipt: ReturnReceipt = {
      id: 'r1',
      after_sales_id: 'a1',
      warehouse_id: 'w1',
      status: 'RECEIVING',
      created_at: '2025-01-01T00:00:00Z',
      updated_at: '2025-01-01T00:00:00Z'
    };
    const cancelledReceipt: ReturnReceipt = {
      ...receivingReceipt,
      status: 'CANCELLED'
    };

    const flagsWithReceiving = getAfterSalesActionFlags(
      createContext({ status: 'APPROVED', return_receipt: receivingReceipt })
    );
    expect(flagsWithReceiving.canCancelReceive).toBe(true);
    expect(flagsWithReceiving.canRedoReceive).toBe(false);
    expect(flagsWithReceiving.canPutaway).toBe(true);

    const flagsWithCancelled = getAfterSalesActionFlags(
      createContext({ status: 'APPROVED', return_receipt: cancelledReceipt })
    );
    expect(flagsWithCancelled.canRedoReceive).toBe(true);
    expect(flagsWithCancelled.canCancelReceive).toBe(false);
    expect(flagsWithCancelled.canPutaway).toBe(false);
  });

  test('escalate only when SLA is OVERDUE', () => {
    expect(getAfterSalesActionFlags(createContext({ sla_status: 'OVERDUE' })).canEscalate).toBe(true);
    expect(getAfterSalesActionFlags(createContext({ sla_status: 'ON_TRACK' })).canEscalate).toBe(false);
  });
});
