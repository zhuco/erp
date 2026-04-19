import { afterEach, describe, expect, it, vi } from 'vitest';

import { mockTenantId } from './mocks/data';

const loadOrdersApiWithMocks = async () => {
  vi.resetModules();
  vi.doMock('@/utils/env', async () => {
    const actual = await vi.importActual<typeof import('@/utils/env')>('@/utils/env');
    return {
      ...actual,
      useMocks: true,
    };
  });

  return import('./orders');
};

afterEach(() => {
  vi.doUnmock('@/utils/env');
  vi.resetModules();
});

describe('order settlement workbench mock API', () => {
  it(
    'returns settlement rules, list data, and settlement detail in mock mode',
    async () => {
      const { getOrderSettlementRule, listOrderSettlements, getOrderSettlementDetail } =
        await loadOrdersApiWithMocks();

      const rule = await getOrderSettlementRule(mockTenantId);
      const settlements = await listOrderSettlements(mockTenantId, { limit: 20 });

      expect(rule.stack_mode).toBe('STACKED');
      expect(settlements.items.length).toBeGreaterThan(0);

      const detail = await getOrderSettlementDetail(mockTenantId, settlements.items[0]!.order_id);

      expect(detail.order_id).toBe(settlements.items[0]!.order_id);
      expect(detail.promotions.length).toBeGreaterThan(0);
    },
    15000,
  );

  it(
    'supports settlement transitions and reconciliation completion in mock mode',
    async () => {
      const {
        listOrderSettlements,
        lockOrderSettlement,
        exportOrderSettlement,
        getOrderSettlementDetail,
        listOrderReconciliations,
        getOrderReconciliationDetail,
        settleOrderReconciliation,
      } = await loadOrdersApiWithMocks();

      const settlements = await listOrderSettlements(mockTenantId, { limit: 20 });
      const target =
        settlements.items.find((item) => item.settlement_status === 'OPEN') ?? settlements.items[0]!;

      const locked = await lockOrderSettlement(mockTenantId, target.order_id, {
        operator_id: 'USER-SETTLEMENT-001',
        operator_name: 'settlement-owner',
      });

      expect(locked.settlement_status).toBe('LOCKED');

      const exported = await exportOrderSettlement(mockTenantId, target.order_id, {
        operator_id: 'USER-SETTLEMENT-001',
        operator_name: 'settlement-owner',
        export_job_id: 'EXPORT-JOB-001',
      });

      expect(exported.settlement_status).toBe('EXPORTED');
      expect(exported.export_job_id).toBe('EXPORT-JOB-001');

      const detail = await getOrderSettlementDetail(mockTenantId, target.order_id);

      expect(detail.settlement_status).toBe('EXPORTED');
      expect(detail.action_logs.some((record) => record.action === 'EXPORT')).toBe(true);

      const reconciliations = await listOrderReconciliations(mockTenantId, { limit: 50 });

      expect(reconciliations.items.length).toBeGreaterThan(0);

      const batch =
        reconciliations.items.find((item) => item.status === 'PENDING') ?? reconciliations.items[0]!;
      const reconciliationDetail = await getOrderReconciliationDetail(mockTenantId, batch.batch_id);

      expect(reconciliationDetail.orders.length).toBeGreaterThan(0);

      const settled = await settleOrderReconciliation(mockTenantId, batch.batch_id, {
        operator_id: 'USER-SETTLEMENT-002',
        operator_name: 'reconciliation-owner',
        note: 'diff reviewed',
      });

      expect(settled.status).toBe('DONE');
    },
    15000,
  );
});
