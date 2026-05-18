import type { KitDetail, SaveKitPayload } from '@/api/kits';
import type {
  Product,
  ProductCreateRequest,
  ProductUpdateRequest,
} from '@/api/products';
import type {
  SkuCreatePayload,
  SkuDetailResponse,
  SkuUpdatePayload,
} from '@/api/skus';

export type GoodsStatus = 'ACTIVE' | 'INACTIVE';

export type GoodsProductFormValues = {
  product_code: string;
  name: string;
  alias: string;
  brand: string;
  category: string;
  base_unit: string;
  second_unit: string;
  default_supplier_id?: string;
  default_warehouse_id?: string;
  primary_barcode: string;
  product_tags: string[];
  remark: string;
  warning_upper?: number;
  warning_lower?: number;
  batch_enabled: boolean;
  shelf_life_days?: number;
  forbid_sale_days?: number;
  serial_enabled: boolean;
  serial_push_platform: boolean;
  allow_negative_stock: boolean;
  status: GoodsStatus;
};

export type GoodsKitComponentFormValue = {
  component_sku_id: string;
  qty: number;
  expire_at?: string | null;
};

export type GoodsKitFormValues = GoodsProductFormValues & {
  sku_code: string;
  sku_name: string;
  sku_barcode: string;
  sku_note: string;
  sku_tags: string[];
  cost_cent?: number;
  sku_status: GoodsStatus;
  components: GoodsKitComponentFormValue[];
};

const trimText = (value?: string | null): string => String(value ?? '').trim();

const normalizeNullableText = (value?: string | null): string | null => {
  const trimmed = trimText(value);
  return trimmed || null;
};

const normalizeOptionalId = (value?: string | null): string | undefined => {
  const trimmed = trimText(value);
  return trimmed || undefined;
};

const normalizeOptionalNumber = (value?: number | null): number | null => {
  if (value == null) {
    return null;
  }
  const normalized = Number(value);
  return Number.isFinite(normalized) ? normalized : null;
};

const normalizeRequiredNumber = (value?: number | null): number => {
  const normalized = Number(value ?? 0);
  return Number.isFinite(normalized) ? normalized : 0;
};

const normalizeTags = (tags?: string[] | null): string[] =>
  Array.from(new Set((tags ?? []).map((tag) => trimText(tag)).filter(Boolean)));

const normalizeWarningEntry = (value?: number | null) => {
  const normalized = normalizeOptionalNumber(value);
  if (normalized == null) {
    return { mode: 'UNLIMITED' as const };
  }
  return {
    mode: 'FIXED' as const,
    value: normalized,
  };
};

const normalizeWarningStock = (upper?: number | null, lower?: number | null) => ({
  upper: normalizeWarningEntry(upper),
  lower: normalizeWarningEntry(lower),
});

const readWarningValue = (
  warningStock: Record<string, unknown> | null | undefined,
  key: 'upper' | 'lower',
): number | undefined => {
  const entry = warningStock?.[key];
  if (!entry || typeof entry !== 'object') {
    return undefined;
  }
  const mode =
    'mode' in entry && typeof entry.mode === 'string' ? entry.mode : undefined;
  const value =
    'value' in entry && typeof entry.value === 'number' ? entry.value : undefined;
  if (mode === 'FIXED' && value != null) {
    return value;
  }
  return undefined;
};

const mapProductCorePayload = (
  values: GoodsProductFormValues,
): Omit<ProductCreateRequest, 'name'> & { name: string } => ({
  product_code: normalizeNullableText(values.product_code),
  name: trimText(values.name),
  alias: normalizeNullableText(values.alias),
  brand: normalizeNullableText(values.brand),
  category: normalizeNullableText(values.category),
  base_unit: normalizeNullableText(values.base_unit),
  second_unit: normalizeNullableText(values.second_unit),
  default_supplier_id: normalizeNullableText(values.default_supplier_id),
  default_warehouse_id: normalizeNullableText(values.default_warehouse_id),
  primary_barcode: normalizeNullableText(values.primary_barcode),
  product_tags: normalizeTags(values.product_tags),
  remark: normalizeNullableText(values.remark),
  warning_stock: normalizeWarningStock(
    values.warning_upper,
    values.warning_lower,
  ),
  batch_enabled: Boolean(values.batch_enabled),
  shelf_life_days: normalizeOptionalNumber(values.shelf_life_days),
  forbid_sale_days: normalizeOptionalNumber(values.forbid_sale_days),
  serial_enabled: Boolean(values.serial_enabled),
  serial_push_platform: Boolean(values.serial_push_platform),
  allow_negative_stock: Boolean(values.allow_negative_stock),
});

const mapProductToBaseFormValues = (
  product?: Product | null,
): GoodsProductFormValues => ({
  product_code: trimText(product?.product_code),
  name: trimText(product?.name),
  alias: trimText(product?.alias),
  brand: trimText(product?.brand),
  category: trimText(product?.category),
  base_unit: trimText(product?.base_unit),
  second_unit: trimText(product?.second_unit),
  default_supplier_id: normalizeOptionalId(product?.default_supplier_id),
  default_warehouse_id: normalizeOptionalId(product?.default_warehouse_id),
  primary_barcode: trimText(product?.primary_barcode),
  product_tags: normalizeTags(product?.product_tags),
  remark: trimText(product?.remark),
  warning_upper: readWarningValue(product?.warning_stock, 'upper'),
  warning_lower: readWarningValue(product?.warning_stock, 'lower'),
  batch_enabled: Boolean(product?.batch_enabled),
  shelf_life_days:
    normalizeOptionalNumber(product?.shelf_life_days) ?? undefined,
  forbid_sale_days:
    normalizeOptionalNumber(product?.forbid_sale_days) ?? undefined,
  serial_enabled: Boolean(product?.serial_enabled),
  serial_push_platform: Boolean(product?.serial_push_platform),
  allow_negative_stock: Boolean(product?.allow_negative_stock),
  status:
    product?.status === 'INACTIVE' || product?.status === 'DISABLED'
      ? 'INACTIVE'
      : 'ACTIVE',
});

export const mapProductFormToCreatePayload = (
  values: GoodsProductFormValues,
): ProductCreateRequest => mapProductCorePayload(values);

export const mapProductFormToUpdatePayload = (
  values: GoodsProductFormValues,
): ProductUpdateRequest => {
  const payload = mapProductCorePayload(values);
  const nextPayload: ProductUpdateRequest = {
    ...payload,
    status: values.status,
  };
  if (!trimText(values.product_code)) {
    delete nextPayload.product_code;
  }
  return nextPayload;
};

export const mapProductToFormValues = (
  product: Product,
): GoodsProductFormValues => mapProductToBaseFormValues(product);

export const mapKitFormToProductPayload = (
  values: GoodsKitFormValues,
): ProductCreateRequest => mapProductCorePayload(values);

export const mapKitFormToSkuPayload = (
  productId: string,
  values: GoodsKitFormValues,
): SkuCreatePayload => ({
  product_id: productId,
  sku_code: trimText(values.sku_code),
  barcode: normalizeNullableText(values.sku_barcode),
  name: normalizeNullableText(values.sku_name),
  note: normalizeNullableText(values.sku_note),
  tags: normalizeTags(values.sku_tags),
  sku_kind: 'KIT',
  cost_cent: normalizeOptionalNumber(values.cost_cent),
  status: values.sku_status,
  batch_enabled: Boolean(values.batch_enabled),
  serial_enabled: Boolean(values.serial_enabled),
});

export const mapKitFormToSkuUpdatePayload = (
  values: GoodsKitFormValues,
): SkuUpdatePayload => ({
  sku_code: trimText(values.sku_code) || undefined,
  barcode: normalizeNullableText(values.sku_barcode),
  name: normalizeNullableText(values.sku_name),
  note: normalizeNullableText(values.sku_note),
  tags: normalizeTags(values.sku_tags),
  sku_kind: 'KIT',
  cost_cent: normalizeOptionalNumber(values.cost_cent),
  status: values.sku_status,
  batch_enabled: Boolean(values.batch_enabled),
  serial_enabled: Boolean(values.serial_enabled),
});

export const mapKitFormToBomPayload = (
  kitSkuId: string,
  values: GoodsKitFormValues,
): SaveKitPayload => ({
  kit_sku_id: kitSkuId,
  components: values.components
    .map((component) => ({
      component_sku_id: trimText(component.component_sku_id),
      qty: normalizeRequiredNumber(component.qty),
      expire_at: component.expire_at ?? null,
    }))
    .filter((component) => component.component_sku_id && component.qty > 0),
});

export const mapKitDetailToFormValues = (
  detail: KitDetail,
  skuDetail?: SkuDetailResponse | null,
): GoodsKitFormValues => ({
  ...mapProductToBaseFormValues(skuDetail?.product),
  sku_code: trimText(detail.kit_sku.sku_code),
  sku_name: trimText(detail.kit_sku.name),
  sku_barcode: trimText(detail.kit_sku.barcode),
  sku_note: trimText(skuDetail?.sku.note),
  sku_tags: normalizeTags(skuDetail?.sku.tags),
  cost_cent: normalizeOptionalNumber(skuDetail?.sku.cost_cent) ?? undefined,
  sku_status:
    skuDetail?.sku.status === 'INACTIVE' || skuDetail?.sku.status === 'DISABLED'
      ? 'INACTIVE'
      : 'ACTIVE',
  components: detail.components.map((component) => ({
    component_sku_id: trimText(component.component_sku_id),
    qty: normalizeRequiredNumber(component.qty),
    expire_at: component.expire_at ?? null,
  })),
});
