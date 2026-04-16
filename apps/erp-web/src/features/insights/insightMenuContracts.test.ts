import { describe, expect, it } from 'vitest';

import { flattenMenuItems, appMenuItems } from '@/app/layout/menu';
import { routeAliases } from '@/app/navigation/routeAliases';

import { insightFeatureCatalog } from './insightFeatureCatalog';

describe('insight menu contracts', () => {
  it('keeps insight menu routes aligned with feature catalog canonical routes', () => {
    const insightsModule = appMenuItems.find((item) => item.name === '洞察');
    expect(insightsModule).toBeDefined();

    const menuPaths = flattenMenuItems(insightsModule?.children ?? [])
      .map((item) => item.path)
      .sort();
    const catalogPaths = insightFeatureCatalog
      .map((feature) => feature.canonicalPath)
      .sort();

    expect(menuPaths).toEqual(catalogPaths);
  });

  it('registers every insight legacy path in route aliases', () => {
    const expectedAliases = insightFeatureCatalog
      .flatMap((feature) =>
        feature.legacyPaths.map((legacyPath) => ({
          legacyPath,
          canonicalPath: feature.canonicalPath,
        })),
      )
      .sort((left, right) => left.legacyPath.localeCompare(right.legacyPath));

    const actualAliases = routeAliases
      .filter((alias) => alias.legacyPath.startsWith('/stats/') || alias.legacyPath === '/reports' || alias.legacyPath === '/exports')
      .sort((left, right) => left.legacyPath.localeCompare(right.legacyPath));

    expect(actualAliases).toEqual(expectedAliases);
  });
});
