import { describe, expect, it } from 'vitest';

import type { Sku } from '@/api/skus';

import { inferSkuField, resolveSkuScan } from './skuScanRules';

const buildSku = (overrides: Partial<Sku>): Sku => ({
  sku_id: 'SKU-1',
  tenant_id: 'TENANT-1',
  sku_code: 'SKU-ERP-1000',
  barcode: 'BAR-1000',
  name: '测试 SKU',
  is_active: true,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-02T00:00:00Z',
  ...overrides
});

describe('resolveSkuScan', () => {
  it('matches barcode first', () => {
    const sku = buildSku({ barcode: '12345678', sku_code: 'SKU-A' });
    const result = resolveSkuScan('12345678', [sku]);
    expect(result.type).toBe('barcode');
  });

  it('matches barcode with surrounding spaces', () => {
    const sku = buildSku({ barcode: 'TRIM-1000' });
    const result = resolveSkuScan('  TRIM-1000  ', [sku]);
    expect(result.type).toBe('barcode');
  });

  it('matches sku_code when barcode not found', () => {
    const sku = buildSku({ sku_code: 'SKU-ERP-2000', barcode: 'BAR-2000' });
    const result = resolveSkuScan('SKU-ERP-2000', [sku]);
    expect(result.type).toBe('sku_code');
  });

  it('matches sku_code case-insensitively', () => {
    const sku = buildSku({ sku_code: 'Sku-Erp-3000', barcode: 'BAR-3000' });
    const result = resolveSkuScan('sku-erp-3000', [sku]);
    expect(result.type).toBe('sku_code');
  });

  it('prioritizes barcode over sku_code', () => {
    const barcodeSku = buildSku({ sku_id: 'SKU-1', barcode: 'DUP-001', sku_code: 'CODE-1' });
    const codeSku = buildSku({ sku_id: 'SKU-2', barcode: 'BAR-2', sku_code: 'DUP-001' });
    const result = resolveSkuScan('DUP-001', [codeSku, barcodeSku]);
    expect(result.type).toBe('barcode');
  });

  it('infers barcode for numeric long input', () => {
    const result = resolveSkuScan('6901234567890', []);
    expect(result.type).toBe('unrecognized');
    if (result.type === 'unrecognized') {
      expect(result.inferredField).toBe('barcode');
    }
  });

  it('infers sku_code for short numeric input', () => {
    const result = resolveSkuScan('123456', []);
    expect(result.type).toBe('unrecognized');
    if (result.type === 'unrecognized') {
      expect(result.inferredField).toBe('sku_code');
    }
  });

  it('infers sku_code for alphanumeric input', () => {
    const result = resolveSkuScan('SKU-NEW-01', []);
    expect(result.type).toBe('unrecognized');
    if (result.type === 'unrecognized') {
      expect(result.inferredField).toBe('sku_code');
    }
  });

  it('treats 8~20 digit input as barcode', () => {
    expect(inferSkuField('12345678')).toBe('barcode');
    expect(inferSkuField('12345678901234567890')).toBe('barcode');
  });
});
