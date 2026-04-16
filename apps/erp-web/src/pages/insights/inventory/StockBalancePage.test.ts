import { describe, expect, it } from 'vitest';

import { StockBalancePage } from './StockBalancePage';

describe('StockBalancePage', () => {
  it('renders the stock balance placeholder shell', () => {
    const element = StockBalancePage();

    expect(element).toBeTruthy();
    expect(element.props['data-testid']).toBe('insights-inventory-stock-balance-page');
  });
});
