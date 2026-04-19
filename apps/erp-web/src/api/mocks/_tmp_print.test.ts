import { describe, it } from 'vitest';
import { listMockOrderExceptions } from './orders';

describe('tmp', () => {
  it('print exceptions', () => {
    const data = listMockOrderExceptions();
    // eslint-disable-next-line no-console
    console.log(JSON.stringify(data.items.map((x, i) => ({ i, order_id: x.order_id, order_no: x.order_no, status: x.exception_status, audit: x.audit_status, blocking: x.blocking_reason })), null, 2));
  });
});
