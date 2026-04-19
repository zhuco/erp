import { afterEach, describe, expect, it, vi } from 'vitest';

const loadMockOrders = async () => {
  vi.resetModules();
  const data = await import('./mocks/data');
  const orders = await import('./mocks/orders');

  return {
    ...orders,
    mockOmsMainChainSeeds: data.mockOmsMainChainSeeds,
    specialMockOrderIds: data.specialMockOrderIds,
  };
};

const loadOrdersApiWithMocks = async () => {
  vi.resetModules();
  vi.doMock('@/utils/env', async () => {
    const actual = await vi.importActual<typeof import('@/utils/env')>('@/utils/env');
    return {
      ...actual,
      useMocks: true,
    };
  });

  const data = await import('./mocks/data');
  const orders = await import('./orders');

  return {
    ...orders,
    mockTenantId: data.mockTenantId,
    specialMockOrderIds: data.specialMockOrderIds,
  };
};

afterEach(() => {
  vi.doUnmock('@/utils/env');
  vi.resetModules();
});

describe('OMS main-chain mocks', () => {
  it('lists only open exceptions that are still not submitted for fulfillment', async () => {
    const { listMockOrderExceptions } = await loadMockOrders();

    const result = listMockOrderExceptions({ status: 'OPEN' });

    expect(result.items.length).toBeGreaterThan(0);
    expect(result.items.every((item) => item.fulfillment_status === 'NOT_SUBMITTED')).toBe(true);
  });

  it('keeps dry-run submissions side-effect free for a fulfillment-ready order', async () => {
    const { specialMockOrderIds, submitMockOrderFulfillment } = await loadMockOrders();

    const first = submitMockOrderFulfillment(specialMockOrderIds.fulfillmentReady, { dryRun: true });
    const second = submitMockOrderFulfillment(specialMockOrderIds.fulfillmentReady, { dryRun: true });

    expect(first.passed).toBe(true);
    expect(first.reserve_status).toBeNull();
    expect(first.fulfillment_status).toBe('NOT_SUBMITTED');
    expect(second.passed).toBe(true);
    expect(second.reserve_status).toBeNull();
    expect(second.fulfillment_status).toBe('NOT_SUBMITTED');
  });

  it('submits a fulfillment-ready order into RESERVED and WAVE_PENDING', async () => {
    const { specialMockOrderIds, submitMockOrderFulfillment } = await loadMockOrders();

    const result = submitMockOrderFulfillment(specialMockOrderIds.fulfillmentReady);

    expect(result.passed).toBe(true);
    expect(result.reserve_status).toBe('RESERVED');
    expect(result.fulfillment_status).toBe('WAVE_PENDING');
  });

  it('keeps reserve-failed samples in NOT_SUBMITTED when submission fails', async () => {
    const { mockOmsMainChainSeeds, submitMockOrderFulfillment } = await loadMockOrders();
    const reserveFailedSeed = mockOmsMainChainSeeds.find(
      (seed) =>
        seed.audit_status === 'REVIEW_PASSED' &&
        seed.exception_status === 'NONE' &&
        seed.reserve_precheck_passed === false,
    );

    expect(reserveFailedSeed).toBeDefined();

    const result = submitMockOrderFulfillment(reserveFailedSeed!.order_id);

    expect(result.passed).toBe(false);
    expect(result.reserve_status).toBe('RESERVE_FAILED');
    expect(result.fulfillment_status).toBe('NOT_SUBMITTED');
  });

  it('clears exception flags after an exception is claimed and resolved', async () => {
    const {
      claimMockOrderException,
      getMockOrderDetail,
      resolveMockOrderException,
      specialMockOrderIds,
    } = await loadMockOrders();

    claimMockOrderException(specialMockOrderIds.missingSku, {
      operator_id: 'USER-OMS-003',
      operator_name: 'reviewer',
    });
    const resolved = resolveMockOrderException(specialMockOrderIds.missingSku, {
      operator_id: 'USER-OMS-003',
      operator_name: 'reviewer',
      reason: 'mapping fixed',
    });
    const detail = getMockOrderDetail(specialMockOrderIds.missingSku);

    expect(resolved.exception_status).toBe('RESOLVED');
    expect(detail.order.exception_flags).toEqual([]);
  });

  it('returns cloned order detail payloads instead of leaking shared references', async () => {
    const { getMockOrderDetail, specialMockOrderIds } = await loadMockOrders();

    const detail = getMockOrderDetail(specialMockOrderIds.addressInvalid);
    detail.items[0]!.title = 'mutated item';
    detail.notes[0]!.note = 'mutated note';
    detail.tags[0]!.tag = 'mutated tag';

    const fresh = getMockOrderDetail(specialMockOrderIds.addressInvalid);

    expect(fresh.items[0]?.title).not.toBe('mutated item');
    expect(fresh.notes[0]?.note).not.toBe('mutated note');
    expect(fresh.tags[0]?.tag).not.toBe('mutated tag');
  });

  it(
    'exposes exception list and fulfillment submission through the public mock API',
    async () => {
      const { listOrderExceptions, mockTenantId, specialMockOrderIds, submitOrdersForFulfillment } =
        await loadOrdersApiWithMocks();

      const exceptionResult = await listOrderExceptions(mockTenantId, { status: 'OPEN' });
      const submitResult = await submitOrdersForFulfillment(mockTenantId, [
        specialMockOrderIds.fulfillmentReady,
      ]);

      expect(exceptionResult.items.length).toBeGreaterThan(0);
      expect(
        exceptionResult.items.every((item) => item.fulfillment_status === 'NOT_SUBMITTED'),
      ).toBe(true);
      expect(submitResult).toHaveLength(1);
      expect(submitResult[0]?.reserve_status).toBe('RESERVED');
      expect(submitResult[0]?.fulfillment_status).toBe('WAVE_PENDING');
    },
    15000,
  );
});
