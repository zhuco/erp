import { describe, expect, it } from 'vitest';

import { LeadTimePage } from './LeadTimePage';
import { OperationAccuracyPage } from './OperationAccuracyPage';
import { OperationLedgerPage } from './OperationLedgerPage';
import { OrderCyclePage } from './OrderCyclePage';
import { PackagingUsagePage } from './PackagingUsagePage';
import { StocktakeVariancePage } from './StocktakeVariancePage';
import { WarehouseNetworkEfficiencyPage } from './WarehouseNetworkEfficiencyPage';

describe('fulfillment insight pages', () => {
  it('exports the phase one wrapper pages with stable test ids', () => {
    expect(LeadTimePage().props['data-testid']).toBe('insights-fulfillment-lead-time-page');
    expect(OrderCyclePage().props['data-testid']).toBe('insights-fulfillment-order-cycle-page');
  });

  it('exports the placeholder page with a stable test id', () => {
    expect(OperationLedgerPage().props['data-testid']).toBe(
      'insights-fulfillment-operation-ledger-page',
    );
    expect(PackagingUsagePage().props['data-testid']).toBe(
      'insights-fulfillment-packaging-usage-page',
    );
    expect(StocktakeVariancePage().props['data-testid']).toBe(
      'insights-fulfillment-stocktake-variance-page',
    );
    expect(OperationAccuracyPage().props['data-testid']).toBe(
      'insights-fulfillment-operation-accuracy-page',
    );
    expect(WarehouseNetworkEfficiencyPage().props['data-testid']).toBe(
      'insights-fulfillment-warehouse-network-efficiency-page',
    );
  });
});
