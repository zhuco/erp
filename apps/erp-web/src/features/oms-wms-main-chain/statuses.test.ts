import { describe, expect, it } from 'vitest';

import {
  auditStatuses,
  auditPassedStatuses,
  deriveReviewStatusLabel,
  exceptionResolvableStatuses,
  exceptionStatuses,
  fulfillmentStatuses,
  handoverStatuses,
  packStatuses,
  pickStatuses,
  reserveStatuses,
  waveStatuses
} from './statuses';

describe('deriveReviewStatusLabel', () => {
  it('AUTO_PASSED 显示为无需复核', () => {
    expect(deriveReviewStatusLabel({ audit_status: 'AUTO_PASSED' })).toBe('无需复核');
  });

  it('REVIEW_REQUIRED 显示为待复核', () => {
    expect(deriveReviewStatusLabel({ audit_status: 'REVIEW_REQUIRED' })).toBe('待复核');
  });

  it('UNDER_REVIEW 显示为待复核', () => {
    expect(deriveReviewStatusLabel({ audit_status: 'UNDER_REVIEW' })).toBe('待复核');
  });

  it('REVIEW_PASSED 显示为复核通过', () => {
    expect(deriveReviewStatusLabel({ audit_status: 'REVIEW_PASSED' })).toBe('复核通过');
  });

  it('REVIEW_REJECTED 显示为复核驳回', () => {
    expect(deriveReviewStatusLabel({ audit_status: 'REVIEW_REJECTED' })).toBe('复核驳回');
  });

  it('未知审核态回落为待复核', () => {
    expect(deriveReviewStatusLabel({ audit_status: 'UNKNOWN_STATUS' })).toBe('待复核');
  });
});

describe('主链状态枚举契约', () => {
  it('审核状态包含 REVIEW_REQUIRED 和 UNDER_REVIEW', () => {
    expect(auditStatuses).toContain('REVIEW_REQUIRED');
    expect(auditStatuses).toContain('UNDER_REVIEW');
  });

  it('审核放行口径仅包含 AUTO_PASSED 和 REVIEW_PASSED', () => {
    expect(auditPassedStatuses).toEqual(['AUTO_PASSED', 'REVIEW_PASSED']);
  });

  it('异常状态覆盖一期异常池语义', () => {
    expect(exceptionStatuses).toEqual(
      expect.arrayContaining([
        'NONE',
        'OPEN',
        'ASSIGNED',
        'PROCESSING',
        'ESCALATED',
        'RESOLVED',
        'CLOSED'
      ])
    );
  });

  it('异常闭环口径仅包含 NONE、RESOLVED、CLOSED', () => {
    expect(exceptionResolvableStatuses).toEqual(['NONE', 'RESOLVED', 'CLOSED']);
  });

  it('履约状态包含一期主链关键状态', () => {
    expect(fulfillmentStatuses).toEqual(
      expect.arrayContaining([
        'NOT_SUBMITTED',
        'WAVE_PENDING',
        'WAVED',
        'PICKING',
        'PACKING',
        'HANDOVER_PENDING',
        'HANDOVERED',
        'SHIPPED'
      ])
    );
  });

  it('履约状态包含发货推单失败状态', () => {
    expect(fulfillmentStatuses).toContain('SHIPPING_PUSH_FAILED');
  });

  it('占库存状态使用 RESERVE_FAILED 口径', () => {
    expect(reserveStatuses).toContain('RESERVE_FAILED');
  });

  it('波次状态使用 CREATED/RELEASED/CANCELED 命名体系', () => {
    expect(waveStatuses).toEqual(expect.arrayContaining(['CREATED', 'RELEASED', 'CANCELED']));
    expect(waveStatuses).not.toEqual(expect.arrayContaining(['WAVE_PENDING', 'WAVED', 'ALLOCATED']));
  });

  it('拣货状态使用 PICKING/PICKED 语义', () => {
    expect(pickStatuses).toEqual(expect.arrayContaining(['PICKING', 'PICKED']));
    expect(pickStatuses).not.toEqual(expect.arrayContaining(['IN_PROGRESS', 'COMPLETED']));
  });

  it('打包状态使用 PACKING/CHECKED/WEIGHED/PACKED 语义', () => {
    expect(packStatuses).toEqual(
      expect.arrayContaining(['PACKING', 'CHECKED', 'WEIGHED', 'PACKED'])
    );
    expect(packStatuses).not.toContain('COMPLETED');
  });

  it('交接状态使用 HANDOVERED/PUSHED 语义', () => {
    expect(handoverStatuses).toEqual(expect.arrayContaining(['HANDOVERED', 'PUSHED']));
  });
});
