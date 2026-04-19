export type OrderWorkbenchGoodsRow = {
  __localKey: string;
  id?: string;
  platform_sku_id?: string | null;
  outer_sku_id?: string | null;
  title?: string | null;
  shopProductName?: string | null;
  shopSpec?: string | null;
  quantity?: number | null;
  price_cent?: number | null;
  mappingMissing?: boolean;
};

const readTextField = (record: Record<string, unknown>, keys: string[]) => {
  for (const key of keys) {
    const rawValue = record[key];
    if (rawValue == null) {
      continue;
    }
    const value = String(rawValue).trim();
    if (value) {
      return value;
    }
  }
  return null;
};

const readNumberField = (record: Record<string, unknown>, keys: string[]) => {
  for (const key of keys) {
    const rawValue = record[key];
    if (typeof rawValue === 'number' && Number.isFinite(rawValue)) {
      return rawValue;
    }
    if (typeof rawValue === 'string') {
      const parsedValue = Number(rawValue);
      if (Number.isFinite(parsedValue)) {
        return parsedValue;
      }
    }
  }
  return null;
};

const readSpecField = (record: Record<string, unknown>, keys: string[]) => {
  for (const key of keys) {
    const rawValue = record[key];
    if (Array.isArray(rawValue)) {
      const values = rawValue.map((item) => String(item ?? '').trim()).filter(Boolean);
      if (values.length > 0) {
        return values.join(' / ');
      }
      continue;
    }
    if (rawValue == null) {
      continue;
    }
    const value = String(rawValue).trim();
    if (value) {
      return value;
    }
  }
  return null;
};

export const toOrderWorkbenchGoodsRows = (rows: Record<string, unknown>[]): OrderWorkbenchGoodsRow[] =>
  rows.map((row, index) => {
    const id = row.id == null ? undefined : String(row.id);
    const fallbackId = `${String(row.platform_sku_id ?? row.outer_sku_id ?? 'goods')}-${index}`;
    const shopProductName = readTextField(row, [
      'shop_product_name',
      'product_title',
      'goods_name',
      'item_title',
      'title',
    ]);
    const shopSpec = readSpecField(row, [
      'shop_spec',
      'spec_text',
      'sku_title',
      'sale_attributes',
      'variant_name',
      'spec',
    ]);

    return {
      __localKey: id ?? fallbackId,
      id,
      platform_sku_id: row.platform_sku_id == null ? null : String(row.platform_sku_id),
      outer_sku_id: row.outer_sku_id == null ? null : String(row.outer_sku_id),
      title: shopProductName,
      shopProductName,
      shopSpec,
      quantity: readNumberField(row, ['quantity', 'qty']) ?? 1,
      price_cent: readNumberField(row, ['price_cent']) ?? 0,
      mappingMissing: Boolean(row.mapping_missing),
    };
  });
