import { describe, expect, it } from 'vitest';

import {
  insightDimensionCatalog,
  insightFeatureCatalog,
  insightPhaseOneRoutes,
} from './insightFeatureCatalog';

const fullInsightRoutes = [
  '/insights/revenue/overview',
  '/insights/revenue/daily-brief',
  '/insights/revenue/trend-radar',
  '/insights/revenue/order-mix',
  '/insights/revenue/discount-insight',
  '/insights/fulfillment/lead-time',
  '/insights/fulfillment/order-cycle',
  '/insights/fulfillment/operation-ledger',
  '/insights/fulfillment/packaging-usage',
  '/insights/fulfillment/stocktake-variance',
  '/insights/fulfillment/operation-accuracy',
  '/insights/fulfillment/warehouse-network-efficiency',
  '/insights/catalog/hot-sku',
  '/insights/catalog/slow-sku',
  '/insights/catalog/bundle-combo',
  '/insights/inventory/stock-balance',
  '/insights/inventory/stock-aging',
  '/insights/inventory/turnover-velocity',
  '/insights/inventory/transfer-benefit',
  '/insights/inventory/negative-stock-alert',
  '/insights/service/return-loop',
  '/insights/service/compensation',
  '/insights/service/reship-quality',
  '/insights/service/ticket-efficiency',
  '/insights/workbench/report-center',
  '/insights/workbench/export-queue',
] as const;

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

  it('covers the complete insights menu scope from the design spec', () => {
    const canonicalPaths = insightFeatureCatalog.map((feature) => feature.canonicalPath).sort();
    expect(canonicalPaths).toEqual([...fullInsightRoutes].sort());
  });

  it('keeps the expected rollout split by delivery phase', () => {
    const phaseCounts = insightFeatureCatalog.reduce<Record<string, number>>((accumulator, feature) => {
      accumulator[feature.availabilityPhase] = (accumulator[feature.availabilityPhase] ?? 0) + 1;
      return accumulator;
    }, {});

    expect(phaseCounts).toEqual({
      'phase-1': 9,
      'phase-1.5': 3,
      'phase-2': 14,
    });
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
        expect.objectContaining({
          key: 'inventory-turnover-velocity',
          canonicalPath: '/insights/inventory/turnover-velocity',
          availabilityPhase: 'phase-1.5',
          ownerDomain: 'inventory',
        }),
        expect.objectContaining({
          key: 'service-ticket-efficiency',
          canonicalPath: '/insights/service/ticket-efficiency',
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
