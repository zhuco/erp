import { describe, expect, it } from 'vitest';

import { NegativeStockAlertPage } from './NegativeStockAlertPage';
import { StockBalancePage } from './StockBalancePage';
import { StockAgingPage } from './StockAgingPage';
import { TransferBenefitPage } from './TransferBenefitPage';
import { TurnoverVelocityPage } from './TurnoverVelocityPage';

describe('inventory insight pages', () => {
  it('exports inventory insight placeholders with stable test ids', () => {
    expect(StockBalancePage().props['data-testid']).toBe('insights-inventory-stock-balance-page');
    expect(StockAgingPage().props['data-testid']).toBe('insights-inventory-stock-aging-page');
    expect(TurnoverVelocityPage().props['data-testid']).toBe(
      'insights-inventory-turnover-velocity-page',
    );
    expect(TransferBenefitPage().props['data-testid']).toBe(
      'insights-inventory-transfer-benefit-page',
    );
    expect(NegativeStockAlertPage().props['data-testid']).toBe(
      'insights-inventory-negative-stock-alert-page',
    );
  });
});
