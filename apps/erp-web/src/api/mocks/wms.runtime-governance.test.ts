import { describe, expect, it } from 'vitest';

import { generateMockSkuSerials, getMockSkuSerialList } from './skus';
import { createMockWmsOutbound, packMockWmsOutbound, shipMockWmsOutbound } from './wms';

describe('wms runtime governance mocks', () => {
  it('blocks forbidden food batches during outbound packing', () => {
    const outbound = createMockWmsOutbound({
      warehouse_id: 'WH-001',
      doc_type: 'SALES',
      doc_no: `R12-FOOD-${Date.now()}`,
      items: [{ sku_id: 'SKU-FOOD-GOV-001', planned_qty: 1 }],
    });

    expect(outbound.items[0]?.batch_enabled).toBe(true);

    expect(() =>
      packMockWmsOutbound(outbound.doc.outbound_doc_id, {
        package_no: `PKG-FOOD-${Date.now()}`,
        items: [{ sku_id: 'SKU-FOOD-GOV-001', packed_qty: 1 }],
        batches: [{ sku_id: 'SKU-FOOD-GOV-001', batch_no: 'BATCH-FOOD-FORBID-001', qty: 1 }],
      }),
    ).toThrow(/禁售|冻结|批次/);
  });

  it('allocates serials on pack, blocks duplicate outbound binding, and ships them', () => {
    const prefix = `R12SN${Date.now()}`;
    const [serial] = generateMockSkuSerials({
      sku_id: 'SKU-DIGITAL-GOV-001',
      prefix,
      count: 1,
    });

    const outbound = createMockWmsOutbound({
      warehouse_id: 'WH-001',
      doc_type: 'SALES',
      doc_no: `R12-DIGITAL-${Date.now()}`,
      items: [{ sku_id: 'SKU-DIGITAL-GOV-001', planned_qty: 1 }],
    });

    expect(outbound.items[0]?.serial_enabled).toBe(true);

    packMockWmsOutbound(outbound.doc.outbound_doc_id, {
      package_no: `PKG-DIGITAL-${Date.now()}`,
      items: [{ sku_id: 'SKU-DIGITAL-GOV-001', packed_qty: 1 }],
      serials: [{ sku_id: 'SKU-DIGITAL-GOV-001', serial_nos: [serial.serial_no] }],
    });

    const allocated = getMockSkuSerialList({
      sku_id: 'SKU-DIGITAL-GOV-001',
      serial_no: serial.serial_no,
      limit: 20,
    });
    expect(allocated.data[0]?.status).toBe('ALLOCATED');
    expect(allocated.data[0]?.ref_doc_type).toBe('OUTBOUND');
    expect(allocated.data[0]?.ref_doc_id).toBe(outbound.doc.outbound_doc_id);

    const duplicateOutbound = createMockWmsOutbound({
      warehouse_id: 'WH-001',
      doc_type: 'SALES',
      doc_no: `R12-DIGITAL-DUP-${Date.now()}`,
      items: [{ sku_id: 'SKU-DIGITAL-GOV-001', planned_qty: 1 }],
    });

    expect(() =>
      packMockWmsOutbound(duplicateOutbound.doc.outbound_doc_id, {
        package_no: `PKG-DUP-${Date.now()}`,
        items: [{ sku_id: 'SKU-DIGITAL-GOV-001', packed_qty: 1 }],
        serials: [{ sku_id: 'SKU-DIGITAL-GOV-001', serial_nos: [serial.serial_no] }],
      }),
    ).toThrow(/绑定|出库|重复/);

    shipMockWmsOutbound(outbound.doc.outbound_doc_id, {});

    const shipped = getMockSkuSerialList({
      sku_id: 'SKU-DIGITAL-GOV-001',
      serial_no: serial.serial_no,
      limit: 20,
    });
    expect(shipped.data[0]?.status).toBe('SHIPPED');
  });
});
