import type { Product } from '@/api/products';

export type ProductSpecForm = {
  spec_code?: string;
  spec_name?: string;
  cost?: number;
  color?: string;
  size?: string;
  ingredient?: string;
  barcode?: string;
  weight?: number;
  unit?: string;
  remark?: string;
  tag?: string;
  second_unit?: string;
};

export type IndustryProfile = 'GENERAL' | 'FOOD' | 'APPAREL' | 'DIGITAL';

export type ShelfLifeUnit = 'DAY' | 'MONTH' | 'YEAR';

export type VariantMode = 'STANDARD' | 'APPAREL_MATRIX';

export type ApparelSizeSystem = 'CN' | 'EU' | 'US' | 'INTL';

export type SerialTrackingMode = 'NONE' | 'ITEM' | 'DEVICE';

export type ProductIndustryFormValues = {
  batch_enabled?: boolean;
  shelf_life_days?: number;
  forbid_sale_days?: number;
  serial_enabled?: boolean;
  serial_push_platform?: boolean;
  industry_profile?: IndustryProfile;
  production_date_enabled?: boolean;
  shelf_life_unit?: ShelfLifeUnit;
  near_expiry_warning_days?: number;
  variant_mode?: VariantMode;
  apparel_size_system?: ApparelSizeSystem;
  serial_tracking_mode?: SerialTrackingMode;
  serial_outbound_required?: boolean;
  serial_inbound_unique_required?: boolean;
  specs?: ProductSpecForm[];
};

export const industryProfileOptions = [
  { value: 'GENERAL', label: '通用' },
  { value: 'FOOD', label: '食品' },
  { value: 'APPAREL', label: '鞋服' },
  { value: 'DIGITAL', label: '数码' },
] satisfies Array<{ value: IndustryProfile; label: string }>;

export const shelfLifeUnitOptions = [
  { value: 'DAY', label: '天' },
  { value: 'MONTH', label: '月' },
  { value: 'YEAR', label: '年' },
] satisfies Array<{ value: ShelfLifeUnit; label: string }>;

export const apparelSizeSystemOptions = [
  { value: 'CN', label: '国标' },
  { value: 'EU', label: '欧码' },
  { value: 'US', label: '美码' },
  { value: 'INTL', label: '国际通码' },
] satisfies Array<{ value: ApparelSizeSystem; label: string }>;

export const serialTrackingModeOptions = [
  { value: 'NONE', label: '不追踪' },
  { value: 'ITEM', label: '单品追踪' },
  { value: 'DEVICE', label: '设备追踪' },
] satisfies Array<{ value: SerialTrackingMode; label: string }>;

export const productIndustryAttrKeys = [
  'industry_profile',
  'production_date_enabled',
  'shelf_life_unit',
  'near_expiry_warning_days',
  'variant_mode',
  'apparel_size_system',
  'serial_tracking_mode',
  'serial_outbound_required',
  'serial_inbound_unique_required',
] as const;

export const readProductAttrs = (record?: Product | null) =>
  record?.attrs && typeof record.attrs === 'object'
    ? (record.attrs as Record<string, unknown>)
    : {};

export const readStringAttr = <T extends string>(
  attrs: Record<string, unknown>,
  key: string,
  fallback: T,
): T => {
  const value = attrs[key];
  return typeof value === 'string' ? (value as T) : fallback;
};

export const readBooleanAttr = (
  attrs: Record<string, unknown>,
  key: string,
  fallback = false,
) => {
  const value = attrs[key];
  return typeof value === 'boolean' ? value : fallback;
};

export const readNumberAttr = (attrs: Record<string, unknown>, key: string) => {
  const value = attrs[key];
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
};

export const omitProductIndustryAttrs = (attrs: Record<string, unknown>) => {
  const next = { ...attrs };
  productIndustryAttrKeys.forEach((key) => {
    delete next[key];
  });
  return next;
};

export const getIndustryProfileLabel = (value?: string | null) =>
  industryProfileOptions.find((option) => option.value === value)?.label ?? '通用';

export const getShelfLifeUnitLabel = (value?: string | null) =>
  shelfLifeUnitOptions.find((option) => option.value === value)?.label ?? '天';

export const getApparelSizeSystemLabel = (value?: string | null) =>
  apparelSizeSystemOptions.find((option) => option.value === value)?.label ?? '国标';

export const getSerialTrackingModeLabel = (value?: string | null) =>
  serialTrackingModeOptions.find((option) => option.value === value)?.label ?? '不追踪';

export const getProductIndustryProfile = (record?: Product | null) =>
  readStringAttr(readProductAttrs(record), 'industry_profile', 'GENERAL');

export const getSpecNamePreview = (spec?: ProductSpecForm) => {
  if (!spec) {
    return '录入颜色与尺码后将自动生成鞋服规格预览';
  }
  const segments = [spec.color?.trim(), spec.size?.trim()].filter(Boolean);
  return segments.length ? segments.join(' / ') : '录入颜色与尺码后将自动生成鞋服规格预览';
};

export const buildProductIndustryAttrs = (values: ProductIndustryFormValues) => {
  const industryProfile = values.industry_profile ?? 'GENERAL';
  const attrs: Record<string, unknown> = {
    industry_profile: industryProfile,
  };

  if (industryProfile === 'FOOD') {
    attrs.production_date_enabled = values.production_date_enabled ?? false;
    attrs.shelf_life_unit = values.shelf_life_unit ?? 'DAY';
    if (typeof values.near_expiry_warning_days === 'number') {
      attrs.near_expiry_warning_days = values.near_expiry_warning_days;
    }
  }

  if (industryProfile === 'APPAREL') {
    attrs.variant_mode = values.variant_mode ?? 'APPAREL_MATRIX';
    attrs.apparel_size_system = values.apparel_size_system ?? 'CN';
  }

  if (industryProfile === 'DIGITAL') {
    attrs.serial_tracking_mode = values.serial_tracking_mode ?? 'ITEM';
    attrs.serial_outbound_required = values.serial_outbound_required ?? false;
    attrs.serial_inbound_unique_required = values.serial_inbound_unique_required ?? false;
  }

  return attrs;
};

const splitIndustryTokens = (value: string) =>
  value
    .split(/[\n,，/|；;]+/)
    .map((item) => item.trim())
    .filter(Boolean);

const normalizeSpecText = (value?: string) => {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
};

const normalizeSpecDraft = (spec: ProductSpecForm): ProductSpecForm => ({
  spec_code: normalizeSpecText(spec.spec_code),
  spec_name: normalizeSpecText(spec.spec_name),
  cost: spec.cost,
  color: normalizeSpecText(spec.color),
  size: normalizeSpecText(spec.size),
  ingredient: normalizeSpecText(spec.ingredient),
  barcode: normalizeSpecText(spec.barcode),
  weight: spec.weight,
  unit: normalizeSpecText(spec.unit),
  remark: normalizeSpecText(spec.remark),
  tag: normalizeSpecText(spec.tag),
  second_unit: normalizeSpecText(spec.second_unit),
});

const buildApparelSpecKey = (color?: string, size?: string) =>
  `${color?.trim() ?? ''}::${size?.trim() ?? ''}`;

export const buildApparelSpecs = (
  colorsText: string,
  sizesText: string,
  existingSpecs: ProductSpecForm[] = [],
): ProductSpecForm[] => {
  const colors = Array.from(new Set(splitIndustryTokens(colorsText)));
  const sizes = Array.from(new Set(splitIndustryTokens(sizesText)));

  if (!colors.length || !sizes.length) {
    return [];
  }

  const mergedSpecs: ProductSpecForm[] = [];
  const existingByKey = new Map<string, ProductSpecForm>();

  existingSpecs.map(normalizeSpecDraft).forEach((spec) => {
    const key = buildApparelSpecKey(spec.color, spec.size);
    if (spec.color && spec.size && !existingByKey.has(key)) {
      existingByKey.set(key, {
        ...spec,
        spec_name: spec.spec_name ?? `${spec.color} / ${spec.size}`,
      });
      mergedSpecs.push(existingByKey.get(key) as ProductSpecForm);
      return;
    }

    if (spec.spec_code || spec.spec_name) {
      mergedSpecs.push(spec);
    }
  });

  colors.forEach((color) => {
    sizes.forEach((size) => {
      const key = buildApparelSpecKey(color, size);
      if (existingByKey.has(key)) {
        return;
      }
      mergedSpecs.push({
        spec_name: `${color} / ${size}`,
        color,
        size,
      });
    });
  });

  return mergedSpecs;
};

export const getFoodPolicySummary = (record?: Product | null) => {
  if (!record) {
    return '-';
  }
  const attrs = readProductAttrs(record);
  if (getProductIndustryProfile(record) !== 'FOOD') {
    return '-';
  }

  const segments = [
    record.batch_enabled ? '批次管理已启用' : '批次管理未启用',
    readBooleanAttr(attrs, 'production_date_enabled') ? '生产日期必填' : '生产日期可选',
    `保质期 ${record.shelf_life_days ?? 0}${getShelfLifeUnitLabel(
      readStringAttr(attrs, 'shelf_life_unit', 'DAY'),
    )}`,
  ];

  const nearExpiryWarningDays = readNumberAttr(attrs, 'near_expiry_warning_days');
  if (typeof nearExpiryWarningDays === 'number') {
    segments.push(`临期预警 ${nearExpiryWarningDays} 天`);
  }
  if (typeof record.forbid_sale_days === 'number') {
    segments.push(`禁售提前 ${record.forbid_sale_days} 天`);
  }

  return segments.join('，');
};

export const getApparelPolicySummary = (record?: Product | null) => {
  if (!record) {
    return '-';
  }
  const attrs = readProductAttrs(record);
  if (getProductIndustryProfile(record) !== 'APPAREL') {
    return '-';
  }
  const specs = Array.isArray(attrs.specs) ? (attrs.specs as ProductSpecForm[]) : [];
  const colorCount = new Set(
    specs.map((spec) => spec.color?.trim()).filter((value): value is string => Boolean(value)),
  ).size;
  const sizeCount = new Set(
    specs.map((spec) => spec.size?.trim()).filter((value): value is string => Boolean(value)),
  ).size;
  return `尺码体系 ${getApparelSizeSystemLabel(
    readStringAttr(attrs, 'apparel_size_system', 'CN'),
  )}，颜色 ${colorCount} 个，尺码 ${sizeCount} 个，规格 ${specs.length} 条，主规格为颜色 / 尺码`;
};

export const getDigitalPolicySummary = (record?: Product | null) => {
  if (!record) {
    return '-';
  }
  const attrs = readProductAttrs(record);
  if (getProductIndustryProfile(record) !== 'DIGITAL') {
    return '-';
  }

  const segments = [
    record.serial_enabled ? '唯一码管理已启用' : '唯一码管理未启用',
    `追踪模式 ${getSerialTrackingModeLabel(readStringAttr(attrs, 'serial_tracking_mode', 'ITEM'))}`,
    readBooleanAttr(attrs, 'serial_outbound_required') ? '出库需绑定序列号' : '出库无需强绑',
    readBooleanAttr(attrs, 'serial_inbound_unique_required') ? '入库唯一校验' : '入库不校验唯一',
  ];

  if (record.serial_push_platform) {
    segments.push('平台回传已启用');
  }

  return segments.join('，');
};
