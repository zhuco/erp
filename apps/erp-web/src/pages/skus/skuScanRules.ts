import type { Sku } from '@/api/skus';

export type SkuScanMatch =
  | { type: 'barcode'; sku: Sku }
  | { type: 'sku_code'; sku: Sku }
  | { type: 'unrecognized'; inferredField: 'barcode' | 'sku_code'; value: string };

const normalizeScanValue = (value: string) => value.trim();
const normalizeSkuCode = (value?: string | null) => (value ?? '').trim().toUpperCase();
const normalizeBarcode = (value?: string | null) => (value ?? '').trim();

export const inferSkuField = (value: string): 'barcode' | 'sku_code' => {
  const normalized = normalizeScanValue(value);
  if (/^\d{8,20}$/.test(normalized)) {
    return 'barcode';
  }
  return 'sku_code';
};

export const resolveSkuScan = (value: string, skus: Sku[]): SkuScanMatch => {
  const normalized = normalizeScanValue(value);
  const barcodeMatch = skus.find((sku) => normalizeBarcode(sku.barcode) === normalized);
  if (barcodeMatch) {
    return { type: 'barcode', sku: barcodeMatch };
  }
  const skuCodeMatch = skus.find(
    (sku) => normalizeSkuCode(sku.sku_code) === normalizeSkuCode(normalized),
  );
  if (skuCodeMatch) {
    return { type: 'sku_code', sku: skuCodeMatch };
  }
  return {
    type: 'unrecognized',
    inferredField: inferSkuField(normalized),
    value: normalized,
  };
};
