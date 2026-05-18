import type { GoodsKitFormValues, GoodsProductFormValues } from './goodsFormUtils';

export const goodsStatusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'INACTIVE' },
] as const;

export const defaultGoodsProductFormValues: GoodsProductFormValues = {
  product_code: '',
  name: '',
  alias: '',
  brand: '',
  category: '',
  base_unit: '',
  second_unit: '',
  default_supplier_id: undefined,
  default_warehouse_id: undefined,
  primary_barcode: '',
  product_tags: [],
  remark: '',
  warning_upper: undefined,
  warning_lower: undefined,
  batch_enabled: false,
  shelf_life_days: undefined,
  forbid_sale_days: undefined,
  serial_enabled: false,
  serial_push_platform: false,
  allow_negative_stock: false,
  status: 'ACTIVE',
};

export const defaultGoodsKitFormValues: GoodsKitFormValues = {
  ...defaultGoodsProductFormValues,
  sku_code: '',
  sku_name: '',
  sku_barcode: '',
  sku_note: '',
  sku_tags: [],
  cost_cent: undefined,
  sku_status: 'ACTIVE',
  components: [],
};
