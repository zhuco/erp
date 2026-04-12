import type { Sku } from '@/api/skus';

export type SkuScanMatch =
  | { type: 'barcode'; sku: Sku }
  | { type: 'sku_code'; sku: Sku }
  | { type: 'unrecognized'; inferredField: 'barcode' | 'sku_code'; value: string };

export const inferSkuField = (value: string): 'barcode' | 'sku_code' => {
  const trimmed = value.trim();
  if (/^\d{8,14}$/.test(trimmed)) {
    return 'barcode';
  }
  if (/^\d{15,20}$/.test(trimmed)) {
    return 'barcode';
  }
  return 'sku_code';
};

export const resolveSkuScan = (value: string, skus: Sku[]): SkuScanMatch => {
  const trimmed = value.trim();
  const barcodeMatch = skus.find((sku) => sku.barcode === trimmed);
  if (barcodeMatch) {
    return { type: 'barcode', sku: barcodeMatch };
  }
  const skuCodeMatch = skus.find((sku) => sku.sku_code === trimmed);
  if (skuCodeMatch) {
    return { type: 'sku_code', sku: skuCodeMatch };
  }
  return {
    type: 'unrecognized',
    inferredField: inferSkuField(trimmed),
    value: trimmed
  };
};
