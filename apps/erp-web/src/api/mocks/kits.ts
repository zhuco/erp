import dayjs from 'dayjs';

import type {
  KitComponent,
  KitDetail,
  KitListQuery,
  KitSummary,
  SaveKitPayload,
  SkuSearchItem
} from './data';
import { applyPagination, mockKitDetails, mockKitSkus, mockSkuSearchIndex } from './data';

export const getMockKitList = (query: KitListQuery) => {
  const normalized = query.keyword?.trim().toLowerCase();
  const items: KitSummary[] = mockKitSkus.map((kit) => {
    const detail = mockKitDetails.get(kit.sku_id);
    return {
      kit_sku_id: kit.sku_id,
      kit_sku_code: kit.sku_code,
      kit_name: kit.name ?? null,
      component_count: detail?.components.length ?? 0,
      updated_at: detail?.updated_at ?? null
    };
  });
  const filtered = normalized
    ? items.filter((item) => {
        const kitLabel = `${item.kit_sku_code} ${item.kit_name ?? ''}`.toLowerCase();
        return kitLabel.includes(normalized);
      })
    : items;
  const offset = query.offset ?? 0;
  const limit = query.limit ?? 20;
  return {
    items: applyPagination(filtered, offset, limit),
    total: filtered.length
  };
};

export const getMockKitDetail = (kitSkuId: string): KitDetail => {
  return (
    mockKitDetails.get(kitSkuId) ?? {
      kit_sku:
        mockSkuSearchIndex.find((sku) => sku.sku_id === kitSkuId) ??
        ({
          sku_id: kitSkuId,
          sku_code: kitSkuId,
          name: null,
          barcode: null,
          sku_kind: 'KIT'
        } as SkuSearchItem),
      components: [],
      updated_at: dayjs().toISOString()
    }
  );
};

export const saveMockKitBom = (payload: SaveKitPayload): KitDetail => {
  const kit =
    mockSkuSearchIndex.find((sku) => sku.sku_id === payload.kit_sku_id) ??
    ({
      sku_id: payload.kit_sku_id,
      sku_code: payload.kit_sku_id,
      name: null,
      barcode: null,
      sku_kind: 'KIT'
    } as SkuSearchItem);
  const components: KitComponent[] = payload.components.map((component) => {
    const sku =
      mockSkuSearchIndex.find((item) => item.sku_id === component.component_sku_id) ??
      ({
        sku_id: component.component_sku_id,
        sku_code: component.component_sku_id,
        name: null,
        barcode: null
      } as SkuSearchItem);
    return {
      component_sku_id: sku.sku_id,
      component_sku_code: sku.sku_code,
      component_name: sku.name ?? null,
      component_barcode: sku.barcode ?? null,
      qty: component.qty,
      expire_at: component.expire_at ?? null
    };
  });
  const updated: KitDetail = {
    kit_sku: { ...kit, sku_kind: kit.sku_kind ?? 'KIT' },
    components,
    updated_at: dayjs().toISOString()
  };
  mockKitDetails.set(kit.sku_id, updated);
  return updated;
};
