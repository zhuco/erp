import { describe, expect, it } from 'vitest';

import {
  insightDimensionCatalog,
  insightFeatureCatalog,
  insightPhaseOneRoutes,
} from './insightFeatureCatalog';

describe('insightFeatureCatalog', () => {
  it('defines the phase one canonical routes needed for the first delivery slice', () => {
    expect(insightPhaseOneRoutes).toEqual(
      expect.arrayContaining([
        '/insights/revenue/overview',
        '/insights/revenue/daily-brief',
        '/insights/revenue/trend-radar',
        '/insights/fulfillment/lead-time',
        '/insights/fulfillment/order-cycle',
        '/insights/catalog/hot-sku',
        '/insights/catalog/slow-sku',
        '/insights/workbench/report-center',
        '/insights/workbench/export-queue',
      ]),
    );
  });

  it('tracks business-question ownership and rollout phase for insight capabilities', () => {
    expect(insightFeatureCatalog).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          key: 'revenue-overview',
          canonicalPath: '/insights/revenue/overview',
          availabilityPhase: 'phase-1',
          ownerDomain: 'revenue',
        }),
        expect.objectContaining({
          key: 'fulfillment-order-cycle',
          canonicalPath: '/insights/fulfillment/order-cycle',
          availabilityPhase: 'phase-1',
          ownerDomain: 'fulfillment',
        }),
        expect.objectContaining({
          key: 'fulfillment-operation-ledger',
          canonicalPath: '/insights/fulfillment/operation-ledger',
          availabilityPhase: 'phase-2',
          ownerDomain: 'fulfillment',
        }),
        expect.objectContaining({
          key: 'service-compensation',
          canonicalPath: '/insights/service/compensation',
          availabilityPhase: 'phase-2',
          ownerDomain: 'service',
        }),
      ]),
    );
  });

  it('publishes shared dimensions so domain pages can render consistent filters', () => {
    expect(insightDimensionCatalog).toEqual(
      expect.objectContaining({
        platform: expect.objectContaining({
          key: 'platform',
        }),
        shop: expect.objectContaining({
          key: 'shop',
        }),
        warehouse: expect.objectContaining({
          key: 'warehouse',
        }),
        category: expect.objectContaining({
          key: 'category',
        }),
      }),
    );
  });
});
