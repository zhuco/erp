import { describe, expect, it } from 'vitest';

import { DailyBriefPage } from './DailyBriefPage';
import { DiscountInsightPage } from './DiscountInsightPage';
import { OrderMixPage } from './OrderMixPage';
import { RevenueOverviewPage } from './RevenueOverviewPage';
import { TrendRadarPage } from './TrendRadarPage';

describe('revenue insight pages', () => {
  it('exports the phase one wrapper pages with stable test ids', () => {
    expect(RevenueOverviewPage().props['data-testid']).toBe('insights-revenue-overview-page');
    expect(DailyBriefPage().props['data-testid']).toBe('insights-revenue-daily-brief-page');
    expect(TrendRadarPage().props['data-testid']).toBe('insights-revenue-trend-radar-page');
  });

  it('exports the placeholder pages with stable test ids', () => {
    expect(OrderMixPage().props['data-testid']).toBe('insights-revenue-order-mix-page');
    expect(DiscountInsightPage().props['data-testid']).toBe(
      'insights-revenue-discount-insight-page',
    );
  });
});
