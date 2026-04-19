import type { ShopProductSku } from '@/api/shopProducts';

export type ShopProductWorkbenchSummary = {
  total: number;
  matched: number;
  unmatched: number;
  conflict: number;
  selected: number;
};

type SummarizeShopProductRowsArgs = {
  rows: ShopProductSku[];
  total: number;
  selectedCount: number;
};

export const resolveActiveShopProductSkuRecordId = (
  rows: ShopProductSku[],
  currentActiveSkuRecordId?: string | null,
) => {
  if (!rows.length) {
    return null;
  }

  if (currentActiveSkuRecordId) {
    const existing = rows.find((row) => row.sku_record_id === currentActiveSkuRecordId);
    if (existing) {
      return existing.sku_record_id;
    }
  }

  return rows[0]?.sku_record_id ?? null;
};

export const summarizeShopProductRows = ({
  rows,
  total,
  selectedCount,
}: SummarizeShopProductRowsArgs): ShopProductWorkbenchSummary => {
  const matched = rows.filter((row) => row.mapping_id && row.mapping_status !== 'CONFLICT').length;
  const conflict = rows.filter((row) => row.mapping_status === 'CONFLICT').length;

  return {
    total,
    matched,
    unmatched: rows.length - matched - conflict,
    conflict,
    selected: selectedCount,
  };
};
