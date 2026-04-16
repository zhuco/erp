import { describe, expect, it } from 'vitest';

import { LeadTimePage } from './LeadTimePage';
import { OperationLedgerPage } from './OperationLedgerPage';
import { OrderCyclePage } from './OrderCyclePage';

describe('fulfillment insight pages', () => {
  it('exports the phase one wrapper pages with stable test ids', () => {
    expect(LeadTimePage().props['data-testid']).toBe('insights-fulfillment-lead-time-page');
    expect(OrderCyclePage().props['data-testid']).toBe('insights-fulfillment-order-cycle-page');
  });

  it('exports the placeholder page with a stable test id', () => {
    expect(OperationLedgerPage().props['data-testid']).toBe(
      'insights-fulfillment-operation-ledger-page',
    );
  });
});
