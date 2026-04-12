import { describe, expect, it, vi } from 'vitest';

import {
  claimMockOrderException,
  getMockOrderDetail,
  listMockOrderExceptions,
  resolveMockOrderException,
  submitMockOrderFulfillment
} from './mocks/orders';
import { mockTenantId } from './mocks/data';

describe('OMS 主链 mock', () => {
  it('异常池只返回 fulfillment_status = NOT_SUBMITTED 的异常单', () => {
    const result = listMockOrderExceptions({ status: 'OPEN' });

    expect(result.items.length).toBeGreaterThan(0);
    expect(result.items.every((item) => item.fulfillment_status === 'NOT_SUBMITTED')).toBe(true);
  });

  it('dryRun 只返回预检结果，不改写主链状态', () => {
    const first = submitMockOrderFulfillment('TEST-ORDER-FULFILLMENT-READY', { dryRun: true });
    const second = submitMockOrderFulfillment('TEST-ORDER-FULFILLMENT-READY', { dryRun: true });

    expect(first.passed).toBe(true);
    expect(first.reserve_status).toBeNull();
    expect(first.fulfillment_status).toBe('NOT_SUBMITTED');
    expect(second.reserve_status).toBeNull();
    expect(second.fulfillment_status).toBe('NOT_SUBMITTED');
  });

  it('提交发货成功后写入 RESERVED 和 WAVE_PENDING', () => {
    const result = submitMockOrderFulfillment('TEST-ORDER-FULFILLMENT-READY');

    expect(result.passed).toBe(true);
    expect(result.reserve_status).toBe('RESERVED');
    expect(result.fulfillment_status).toBe('WAVE_PENDING');
  });

  it('提交发货失败后写入 RESERVE_FAILED 并保持 NOT_SUBMITTED', () => {
    const result = submitMockOrderFulfillment('TEST-ORDER-RESERVE-FAILED');

    expect(result.passed).toBe(false);
    expect(result.reserve_status).toBe('RESERVE_FAILED');
    expect(result.fulfillment_status).toBe('NOT_SUBMITTED');
  });

  it('领取后复核通过会清空已闭环异常的 flags', () => {
    claimMockOrderException('TEST-ORDER-MISSING-SKU', {
      operator_id: 'USER-OMS-003',
      operator_name: '异常复核员'
    });
    const resolved = resolveMockOrderException('TEST-ORDER-MISSING-SKU', {
      operator_id: 'USER-OMS-003',
      operator_name: '异常复核员',
      reason: '已人工补齐商品映射'
    });
    const detail = getMockOrderDetail('TEST-ORDER-MISSING-SKU');

    expect(resolved.exception_status).toBe('RESOLVED');
    expect(detail.order.exception_flags).toEqual([]);
  });

  it('订单详情返回值不应泄漏共享引用', () => {
    const detail = getMockOrderDetail('TEST-ORDER-ADDRESS-INVALID');
    detail.items[0]!.title = '已被污染的商品';
    detail.notes[0]!.note = '已被污染的备注';
    detail.tags[0]!.tag = '已被污染的标记';

    const fresh = getMockOrderDetail('TEST-ORDER-ADDRESS-INVALID');

    expect(fresh.items[0]?.title).not.toBe('已被污染的商品');
    expect(fresh.notes[0]?.note).not.toBe('已被污染的备注');
    expect(fresh.tags[0]?.tag).not.toBe('已被污染的标记');
  });

  it('公共 API 在 mock 模式下返回异常池与提交发货结果', async () => {
    vi.resetModules();
    vi.doMock('@/utils/env', async () => {
      const actual = await vi.importActual<typeof import('@/utils/env')>('@/utils/env');
      return {
        ...actual,
        useMocks: true
      };
    });

    const { listOrderExceptions, submitOrdersForFulfillment } = await import('./orders');

    const exceptionResult = await listOrderExceptions(mockTenantId, { status: 'OPEN' });
    const submitResult = await submitOrdersForFulfillment(mockTenantId, [
      'TEST-ORDER-FULFILLMENT-READY'
    ]);

    expect(exceptionResult.items.length).toBeGreaterThan(0);
    expect(exceptionResult.items.every((item) => item.fulfillment_status === 'NOT_SUBMITTED')).toBe(
      true
    );
    expect(submitResult).toHaveLength(1);
    expect(submitResult[0]?.reserve_status).toBe('RESERVED');
    expect(submitResult[0]?.fulfillment_status).toBe('WAVE_PENDING');

    vi.doUnmock('@/utils/env');
  });
});
