import {
  Button,
  Card,
  Checkbox,
  Col,
  DatePicker,
  Divider,
  Form,
  Input,
  InputNumber,
  Modal,
  Radio,
  Row,
  Select,
  Space,
  Tag,
  TimePicker,
  Typography,
  message
} from 'antd';
import type { MenuProps } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { Key } from 'react';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useSearchParams } from 'react-router-dom';

import { listChannels } from '@/api/channels';
import { listCouriers } from '@/api/couriers';
import { listOrderTagDefinitions } from '@/api/orderTagDefinitions';
import { listWarehouses } from '@/api/warehouses';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { UiModule06StrategyTable } from '@/components/ui-modules';
import {
  loadSalesStrategies,
  saveSalesStrategies,
  type SalesStrategy,
  type SalesStrategyCondition
} from '@/features/salesStrategies';
import { platformOptions } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

import './salesStrategySettingsPage.css';

type ConditionFormValues = {
  field?: string;
  operator?: string;
  value?: string | string[] | Array<number | undefined>;
};

type MatchRuleFormValues = {
  scope?: string;
  areas?: string[];
};

type StrategyFormValues = {
  name: string;
  priority?: number;
  splitCount?: number;
  mergeCount?: number;
  remark?: string;
  salesChannel?: string[];
  salesOrderType?: string;
  salesOrderTag?: string;
  commissionMatchMode?: string;
  commissionAssignee?: string;
  commissionNotePrefix?: string;
  commissionNoteContent?: string;
  commissionNoteSuffix?: string;
  commissionFallbackAssignee?: string;
  commissionExceptionTag?: string;
  parseContent?: string;
  rulePrefix?: string;
  ruleSeparator?: string;
  ruleSuffix?: string;
  priceBasis?: string;
  priceDiscount?: number;
  parseErrorTag?: string;
  parseSuccessTag?: string;
  autoExecute?: boolean;
  logisticsMatchMode?: string;
  logisticsIncludeRules?: MatchRuleFormValues[];
  logisticsExcludeRules?: MatchRuleFormValues[];
  logisticsValidRange?: [dayjs.Dayjs, dayjs.Dayjs];
  logisticsSmartSelect?: boolean;
  warehouseMatchMode?: string;
  warehouseIncludeRules?: MatchRuleFormValues[];
  warehouseExcludeRules?: MatchRuleFormValues[];
  warehouseValidRange?: [dayjs.Dayjs, dayjs.Dayjs];
  warehouseSmartSelect?: boolean;
  executionStrategy?: string;
  executeTime?: dayjs.Dayjs;
  validPeriod?: string;
  conditions?: ConditionFormValues[];
  exclusions?: ConditionFormValues[];
};

type ConditionValueMeta = {
  options: { label: string; value: string }[];
  placeholder: string;
};

const validPeriodOptions = [
  { label: '一天', value: 'day' },
  { label: '一周', value: 'week' },
  { label: '一月', value: 'month' },
  { label: '一年', value: 'year' },
  { label: '永久', value: 'forever' }
];

const operatorOptions = [
  { label: '=', value: '=' },
  { label: '!=', value: '!=' },
  { label: '>', value: '>' },
  { label: '>=', value: '>=' },
  { label: '<', value: '<' },
  { label: '<=', value: '<=' },
  { label: '包含', value: 'contains' },
  { label: '不包含', value: 'not_contains' }
];

const parseContentOptions = [
  { label: '客服备注', value: 'service_remark' },
  { label: '客户备注', value: 'customer_remark' },
  { label: '交易名称(标题)', value: 'trade_title' },
  { label: '交易规格', value: 'trade_spec' },
  { label: '商品编码(网店)', value: 'sku_code' },
  { label: '规格编码(网店)', value: 'spec_code' }
];

const commissionOrderTypeOptions = [
  { label: '普通订单', value: 'STANDARD' },
  { label: '补发订单', value: 'RESEND' },
  { label: '售后订单', value: 'AFTER_SALES' }
];

const commissionMatchOptions = [
  { label: '客户档案业务员', value: 'customer_owner' },
  { label: '渠道负责人', value: 'channel_owner' },
  { label: '指定业务员', value: 'fixed_salesperson' },
  { label: '备注提取', value: 'note_extract' }
];

const commissionNoteContentOptions = [
  { label: '业务员姓名', value: 'name' },
  { label: '业务员工号', value: 'code' },
  { label: '业务员昵称', value: 'nickname' }
];

const priceBasisOptions = [
  { label: '零售价', value: 'retail' },
  { label: '销售价', value: 'sale' },
  { label: '成本价', value: 'cost' }
];

const rangeFieldMeta: Record<
  string,
  { minPlaceholder: string; maxPlaceholder: string; precision: number; step?: number }
> = {
  estimated_weight: { minPlaceholder: '最小重量', maxPlaceholder: '最大重量', precision: 2, step: 0.01 },
  order_amount: { minPlaceholder: '最低货款', maxPlaceholder: '最高货款', precision: 2, step: 0.01 },
  sku_count: { minPlaceholder: '最少样数', maxPlaceholder: '最多样数', precision: 0, step: 1 }
};

const rangeFields = new Set(Object.keys(rangeFieldMeta));

const logisticsMatchModeOptions = [
  { label: '全部满足', value: 'all' },
  { label: '命中任一', value: 'any' },
  { label: '不限制', value: 'none' }
];

const logisticsRegionScopeOptions = [
  { label: '收货省份', value: 'province' },
  { label: '收货城市', value: 'city' },
  { label: '收货区域', value: 'district' },
  { label: '收货街道', value: 'street' }
];

const logisticsAreaOptions = [
  { label: '华东', value: '华东' },
  { label: '华南', value: '华南' },
  { label: '华北', value: '华北' },
  { label: '东北', value: '东北' },
  { label: '西南', value: '西南' },
  { label: '西北', value: '西北' }
];

const conditionFieldOptions = [
  { label: '销售平台', value: 'sales_platform' },
  { label: '销售渠道', value: 'sales_channel' },
  { label: '发货仓库', value: 'warehouse' },
  { label: '物流公司', value: 'logistics_company' },
  { label: '指定货品', value: 'sku' },
  { label: '货品标记', value: 'sku_tag' },
  { label: '订单标记', value: 'order_tag' },
  { label: '客服备注', value: 'service_remark' },
  { label: '客户备注', value: 'customer_remark' },
  { label: '追加备注', value: 'append_remark' },
  { label: '收货地址包含', value: 'shipping_address' },
  { label: '预估重量', value: 'estimated_weight' },
  { label: '货款合计', value: 'order_amount' },
  { label: '货品样数', value: 'sku_count' }
];

const fixedValueFields = new Set([
  'sales_platform',
  'sales_channel',
  'warehouse',
  'logistics_company'
]);
const parseRuleExample =
  '格式示例：货品编码*数量；辅助条码*数量；货品编号,规格*数量';
const commissionNoteExample = '示例：#张三# / #S001#';

const normalizeSalesChannelValue = (
  value?: SalesStrategy['salesChannel'] | string
): string[] | undefined => {
  if (!value) {
    return undefined;
  }
  if (Array.isArray(value)) {
    return value;
  }
  return [value];
};

const navItems = [
  { key: 'logistics', label: '物流匹配策略' },
  { key: 'warehouse', label: '仓库匹配策略' },
  { key: 'sales', label: '业务员匹配策略' },
  { key: 'auto-review', label: '自动审核策略' },
  { key: 'auto-split', label: '自动拆分策略' },
  { key: 'auto-merge', label: '自动合并策略' },
  { key: 'goods', label: '货品解析策略' },
  { key: 'order-tags', label: '订单标记策略' },
  { key: 'sales-commission', label: '销售提成策略' },
  { key: 'shipping-commission', label: '发货提成策略' }
];

const buildStrategyId = () => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `ss-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

const buildDefaultStrategyName = (label: string, items: SalesStrategy[]) => {
  const baseLabel = label?.trim() || '策略';
  const prefix = `${baseLabel}-`;
  let maxIndex = 0;
  items.forEach((item) => {
    if (!item.name?.startsWith(prefix)) {
      return;
    }
    const suffix = item.name.slice(prefix.length);
    const parsed = Number(suffix);
    if (!Number.isNaN(parsed)) {
      maxIndex = Math.max(maxIndex, parsed);
    }
  });
  return `${baseLabel}-${maxIndex + 1}`;
};

const normalizeConditionValue = (
  field: string | undefined,
  value?: ConditionFormValues['value']
): string => {
  if (field && rangeFields.has(field)) {
    if (Array.isArray(value)) {
      const [min, max] = value;
      const format = (item?: number) =>
        item == null || Number.isNaN(item) ? '' : String(item);
      const serialized = [format(min), format(max)];
      return serialized.some(Boolean) ? serialized.join('~') : '';
    }
    return (value ?? '').toString().trim();
  }
  if (Array.isArray(value)) {
    return value
      .map((item) => (item == null ? '' : String(item)).trim())
      .filter(Boolean)
      .join(',');
  }
  return (value ?? '').toString().trim();
};

const mapConditionValueToForm = (field?: string, value?: string): ConditionFormValues['value'] => {
  if (!value) {
    return undefined;
  }
  if (field && rangeFields.has(field)) {
    const separator = value.includes('~') ? '~' : ',';
    const parts = value.split(separator);
    const range = parts.slice(0, 2).map((item) => {
      const trimmed = item.trim();
      if (!trimmed) {
        return undefined;
      }
      const parsed = Number(trimmed);
      return Number.isNaN(parsed) ? undefined : parsed;
    });
    return range.some((item) => item != null) ? range : undefined;
  }
  if (field && fixedValueFields.has(field)) {
    return value
      .split(',')
      .map((item) => item.trim())
      .filter(Boolean);
  }
  return value;
};

const mapConditionsForForm = (values?: SalesStrategyCondition[]): ConditionFormValues[] =>
  (values ?? []).map((item) => ({
    field: item.field,
    operator: item.operator,
    value: mapConditionValueToForm(item.field, item.value)
  }));

const normalizeConditions = (values?: ConditionFormValues[]): SalesStrategyCondition[] =>
  (values ?? [])
    .map((item) => ({
      field: (item.field ?? '').trim(),
      operator: item.operator ?? '',
      value: normalizeConditionValue(item.field, item.value)
    }))
    .filter((item) => item.field || item.operator || item.value);

const normalizeMatchRules = (values?: MatchRuleFormValues[]) =>
  (values ?? [])
    .map((item) => {
      const scope = item.scope?.trim();
      const areas = (item.areas ?? []).map((value) => value.trim()).filter(Boolean);
      if (!scope && !areas.length) {
        return null;
      }
      return {
        scope: scope || undefined,
        areas: areas.length ? areas : undefined
      };
    })
    .filter((item): item is { scope?: string; areas?: string[] } => item !== null);

const formatConditionList = (values?: SalesStrategyCondition[]) => {
  if (!values?.length) {
    return '-';
  }
  const items = values
    .map((item) => [item.field, item.operator, item.value].filter(Boolean).join(' '))
    .filter(Boolean);
  return items.length ? items.join('；') : '-';
};

export const SalesStrategySettingsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('orders.write');
  const { setLeftPanelVisible, setRightPanelVisible, setBottomPanelVisible, clearPanels } =
    useWorkbenchPanels();
  const [searchParams] = useSearchParams();
  const tabParam = searchParams.get('tab');
  const resolveNavKey = (value?: string | null) =>
    navItems.some((item) => item.key === value) ? value : navItems[0].key;
  const [strategies, setStrategies] = useState<SalesStrategy[]>(() => loadSalesStrategies());
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [activeNavKey, setActiveNavKey] = useState(() => resolveNavKey(tabParam));
  const [modalOpen, setModalOpen] = useState(false);
  const [editingStrategy, setEditingStrategy] = useState<SalesStrategy | null>(null);
  const [form] = Form.useForm<StrategyFormValues>();
  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'options'],
    queryFn: () =>
      listWarehouses(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200
      }),
    enabled: tenantReady
  });
  const channelsQuery = useQuery({
    queryKey: ['channels', tenantId, 'options'],
    queryFn: () =>
      listChannels(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200
      }),
    enabled: tenantReady
  });
  const couriersQuery = useQuery({
    queryKey: ['couriers', tenantId, 'options'],
    queryFn: () =>
      listCouriers(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200
      }),
    enabled: tenantReady
  });
  const orderTagsQuery = useQuery({
    queryKey: ['order-tag-definitions', tenantId, 'options'],
    queryFn: () =>
      listOrderTagDefinitions(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200
      }),
    enabled: tenantReady
  });

  const warehouseOptions = useMemo(
    () =>
      (warehousesQuery.data?.items ?? []).map((warehouse) => ({
        label: `${warehouse.name} (${warehouse.code})`,
        value: warehouse.name
      })),
    [warehousesQuery.data?.items]
  );

  const channelOptions = useMemo(
    () =>
      (channelsQuery.data?.items ?? []).map((channel) => ({
        label: channel.name,
        value: channel.name
      })),
    [channelsQuery.data?.items]
  );

  const logisticsCompanyOptions = useMemo(
    () =>
      (couriersQuery.data?.items ?? []).map((courier) => ({
        label: `${courier.name} (${courier.code})`,
        value: courier.name
      })),
    [couriersQuery.data?.items]
  );

  const orderTagOptions = useMemo(
    () =>
      (orderTagsQuery.data?.items ?? []).map((tag) => ({
        label: tag.name,
        value: tag.name
      })),
    [orderTagsQuery.data?.items]
  );

  const conditionValueMeta = useMemo<Record<string, ConditionValueMeta>>(
    () => ({
      sales_platform: {
        options: platformOptions,
        placeholder: '选择销售平台'
      },
      sales_channel: {
        options: channelOptions,
        placeholder: '选择销售渠道'
      },
      warehouse: {
        options: warehouseOptions,
        placeholder: '选择发货仓库'
      },
      logistics_company: {
        options: logisticsCompanyOptions,
        placeholder: '选择物流公司'
      }
    }),
    [channelOptions, logisticsCompanyOptions, warehouseOptions, platformOptions]
  );

  useEffect(() => {
    if (!tabParam) {
      return;
    }
    setActiveNavKey(resolveNavKey(tabParam));
  }, [tabParam]);

  useEffect(() => {
    setLeftPanelVisible(false);
    setRightPanelVisible(false);
    setBottomPanelVisible(false);
    clearPanels();
    return () => clearPanels();
  }, [clearPanels, setBottomPanelVisible, setLeftPanelVisible, setRightPanelVisible]);

  useEffect(() => {
    saveSalesStrategies(strategies);
  }, [strategies]);

  const activeNavLabel = useMemo(
    () => navItems.find((item) => item.key === activeNavKey)?.label ?? '',
    [activeNavKey]
  );

  const validPeriodLabelMap = useMemo(
    () => new Map(validPeriodOptions.map((option) => [option.value, option.label])),
    []
  );

  const visibleStrategies = useMemo(
    () => strategies.filter((strategy) => strategy.type === activeNavKey),
    [activeNavKey, strategies]
  );
  const selectedStrategies = useMemo(
    () => strategies.filter((strategy) => selectedRowKeys.includes(strategy.id)),
    [selectedRowKeys, strategies]
  );

  const isGoodsStrategy = activeNavKey === 'goods';
  const isLogisticsStrategy = activeNavKey === 'logistics';
  const isWarehouseStrategy = activeNavKey === 'warehouse';
  const isAutoSplitStrategy = activeNavKey === 'auto-split';
  const isAutoMergeStrategy = activeNavKey === 'auto-merge';
  const isSalesCommissionStrategy = activeNavKey === 'sales-commission';
  const commissionMatchMode = Form.useWatch('commissionMatchMode', form);
  const logisticsMatchMode = Form.useWatch('logisticsMatchMode', form);
  const warehouseMatchMode = Form.useWatch('warehouseMatchMode', form);
  const logisticsMatchUnlimited = logisticsMatchMode === 'none';
  const warehouseMatchUnlimited = warehouseMatchMode === 'none';

  const handleConditionFieldChange = (listName: 'conditions' | 'exclusions', index: number) => {
    form.setFieldValue([listName, index, 'value'], undefined);
  };

  const ConditionValueField = ({
    listName,
    fieldIndex
  }: {
    listName: 'conditions' | 'exclusions';
    fieldIndex: number;
  }) => {
    const selectedField = Form.useWatch([listName, fieldIndex, 'field'], form);
    const meta = selectedField ? conditionValueMeta[selectedField] : undefined;
    const rangeMeta = selectedField ? rangeFieldMeta[selectedField] : undefined;
    if (rangeMeta) {
      return (
        <Form.Item className="sales-strategy-settings-condition-value">
          <Space.Compact style={{ width: '100%' }}>
            <Form.Item name={[fieldIndex, 'value', 0]} noStyle>
              <InputNumber
                min={0}
                precision={rangeMeta.precision}
                step={rangeMeta.step}
                placeholder={rangeMeta.minPlaceholder}
                style={{ width: '100%' }}
              />
            </Form.Item>
            <Form.Item name={[fieldIndex, 'value', 1]} noStyle>
              <InputNumber
                min={0}
                precision={rangeMeta.precision}
                step={rangeMeta.step}
                placeholder={rangeMeta.maxPlaceholder}
                style={{ width: '100%' }}
              />
            </Form.Item>
          </Space.Compact>
        </Form.Item>
      );
    }
    if (meta) {
      return (
        <Form.Item className="sales-strategy-settings-condition-value" name={[fieldIndex, 'value']}>
          <Select
            allowClear
            mode="multiple"
            maxTagCount="responsive"
            options={meta.options}
            placeholder={meta.placeholder}
            showSearch
            optionFilterProp="label"
          />
        </Form.Item>
      );
    }
    return (
      <Form.Item className="sales-strategy-settings-condition-value" name={[fieldIndex, 'value']}>
        <Input placeholder="值" />
      </Form.Item>
    );
  };

  const columns = useMemo<ColumnsType<SalesStrategy>>(() => {
    const baseColumns: ColumnsType<SalesStrategy> = [
      { title: '策略名称', dataIndex: 'name' },
      { title: '优先级', dataIndex: 'priority' }
    ];
    if (isAutoSplitStrategy) {
      baseColumns.push({
        title: '拆分数量',
        dataIndex: 'splitCount',
        render: (value) => value ?? '-'
      });
    }
    if (isAutoMergeStrategy) {
      baseColumns.push({
        title: '合并数量',
        dataIndex: 'mergeCount',
        render: (value) => value ?? '-'
      });
    }
    baseColumns.push(
      {
        title: '备注',
        dataIndex: 'remark',
        ellipsis: true,
        render: (value) => value || '-'
      },
      {
        title: '满足条件',
        dataIndex: 'conditions',
        ellipsis: true,
        render: (_, record) => formatConditionList(record.conditions)
      },
      {
        title: '排除条件',
        dataIndex: 'exclusions',
        ellipsis: true,
        render: (_, record) => formatConditionList(record.exclusions)
      },
      {
        title: '执行时间',
        dataIndex: 'executeTime',
        render: (value) => value || '-'
      },
      {
        title: '有效期',
        dataIndex: 'validPeriod',
        render: (value) => validPeriodLabelMap.get(value) ?? value
      },
      {
        title: '状态',
        dataIndex: 'enabled',
        render: (value) => (value ? <Tag color="green">启用</Tag> : <Tag>停用</Tag>)
      },
      {
        title: '更新时间',
        dataIndex: 'updatedAt',
        render: (value) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-')
      }
    );
    return baseColumns;
  }, [isAutoMergeStrategy, isAutoSplitStrategy, validPeriodLabelMap]);

  const openCreateModal = () => {
    setEditingStrategy(null);
    form.resetFields();
    const nextValues: StrategyFormValues = {
      name: buildDefaultStrategyName(activeNavLabel, visibleStrategies),
      priority: 1,
      validPeriod: 'forever',
      conditions: [],
      exclusions: []
    };
    if (isGoodsStrategy) {
      nextValues.parseContent = 'trade_title';
      nextValues.priceBasis = 'retail';
      nextValues.priceDiscount = 100;
      nextValues.autoExecute = true;
    }
    if (isSalesCommissionStrategy) {
      nextValues.commissionMatchMode = 'customer_owner';
      nextValues.commissionNotePrefix = '#';
      nextValues.commissionNoteContent = 'name';
      nextValues.commissionNoteSuffix = '#';
    }
    if (isLogisticsStrategy) {
      nextValues.logisticsMatchMode = 'all';
      nextValues.logisticsIncludeRules = [];
      nextValues.logisticsExcludeRules = [];
      nextValues.logisticsSmartSelect = false;
    }
    if (isWarehouseStrategy) {
      nextValues.warehouseMatchMode = 'all';
      nextValues.warehouseIncludeRules = [];
      nextValues.warehouseExcludeRules = [];
      nextValues.warehouseSmartSelect = false;
    }
    if (isAutoSplitStrategy) {
      nextValues.splitCount = 2;
    }
    if (isAutoMergeStrategy) {
      nextValues.mergeCount = 2;
    }
    form.setFieldsValue(nextValues);
    setModalOpen(true);
  };

  const openEditModal = (strategy?: SalesStrategy) => {
    if (!strategy) {
      if (!selectedRowKeys.length) {
        message.warning('请选择策略');
        return;
      }
      if (selectedRowKeys.length > 1) {
        message.warning('一次只能修改一个策略');
        return;
      }
    }
    const targetStrategy = strategy ?? strategies.find((item) => item.id === selectedRowKeys[0]);
    if (!targetStrategy) {
      message.error('未找到策略');
      return;
    }
    setEditingStrategy(targetStrategy);
    form.resetFields();
    form.setFieldsValue({
      name: targetStrategy.name,
      priority: targetStrategy.priority,
      splitCount: isAutoSplitStrategy ? targetStrategy.splitCount ?? 2 : undefined,
      mergeCount: isAutoMergeStrategy ? targetStrategy.mergeCount ?? 2 : undefined,
      remark: targetStrategy.remark,
      salesChannel: normalizeSalesChannelValue(targetStrategy.salesChannel),
      salesOrderType: targetStrategy.salesOrderType,
      salesOrderTag: targetStrategy.salesOrderTag,
      commissionMatchMode: targetStrategy.commissionMatchMode,
      commissionAssignee: targetStrategy.commissionAssignee,
      commissionNotePrefix: targetStrategy.commissionNotePrefix,
      commissionNoteContent: targetStrategy.commissionNoteContent,
      commissionNoteSuffix: targetStrategy.commissionNoteSuffix,
      commissionFallbackAssignee: targetStrategy.commissionFallbackAssignee,
      commissionExceptionTag: targetStrategy.commissionExceptionTag,
      parseContent: targetStrategy.parseContent,
      rulePrefix: targetStrategy.rulePrefix,
      ruleSeparator: targetStrategy.ruleSeparator,
      ruleSuffix: targetStrategy.ruleSuffix,
      priceBasis: targetStrategy.priceBasis,
      priceDiscount: targetStrategy.priceDiscount,
      parseErrorTag: targetStrategy.parseErrorTag,
      parseSuccessTag: targetStrategy.parseSuccessTag,
      autoExecute: targetStrategy.autoExecute,
      logisticsMatchMode: targetStrategy.logisticsMatchMode,
      logisticsIncludeRules: targetStrategy.logisticsIncludeRules ?? [],
      logisticsExcludeRules: targetStrategy.logisticsExcludeRules ?? [],
      logisticsSmartSelect: targetStrategy.logisticsSmartSelect,
      logisticsValidRange:
        targetStrategy.logisticsValidFrom && targetStrategy.logisticsValidTo
          ? [
              dayjs(targetStrategy.logisticsValidFrom),
              dayjs(targetStrategy.logisticsValidTo)
            ]
          : undefined,
      warehouseMatchMode: targetStrategy.warehouseMatchMode,
      warehouseIncludeRules: targetStrategy.warehouseIncludeRules ?? [],
      warehouseExcludeRules: targetStrategy.warehouseExcludeRules ?? [],
      warehouseSmartSelect: targetStrategy.warehouseSmartSelect,
      warehouseValidRange:
        targetStrategy.warehouseValidFrom && targetStrategy.warehouseValidTo
          ? [
              dayjs(targetStrategy.warehouseValidFrom),
              dayjs(targetStrategy.warehouseValidTo)
            ]
          : undefined,
      executionStrategy: targetStrategy.executionStrategy,
      validPeriod: targetStrategy.validPeriod,
      executeTime: targetStrategy.executeTime
        ? dayjs(targetStrategy.executeTime, 'HH:mm')
        : undefined,
      conditions: mapConditionsForForm(targetStrategy.conditions),
      exclusions: mapConditionsForForm(targetStrategy.exclusions)
    });
    setModalOpen(true);
  };

  const handleDisable = () => {
    if (!selectedRowKeys.length) {
      message.warning('请选择策略');
      return;
    }
    setStrategies((prev) =>
      prev.map((item) =>
        selectedRowKeys.includes(item.id)
          ? { ...item, enabled: false, updatedAt: new Date().toISOString() }
          : item
      )
    );
    message.success('策略已停用');
  };

  const handleEnable = () => {
    if (!selectedRowKeys.length) {
      message.warning('请选择策略');
      return;
    }
    setStrategies((prev) =>
      prev.map((item) =>
        selectedRowKeys.includes(item.id)
          ? { ...item, enabled: true, updatedAt: new Date().toISOString() }
          : item
      )
    );
    message.success('策略已启用');
  };

  const handleDelete = () => {
    if (!selectedRowKeys.length) {
      message.warning('请选择策略');
      return;
    }
    const hasEnabled = selectedStrategies.some((item) => item.enabled);
    if (hasEnabled) {
      message.warning('只有停用状态的策略可以删除');
      return;
    }
    setStrategies((prev) => prev.filter((item) => !selectedRowKeys.includes(item.id)));
    setSelectedRowKeys([]);
    message.success('策略已删除');
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      const now = new Date().toISOString();
      const trimmedName = values.name?.trim();
      const resolvedName =
        trimmedName || editingStrategy?.name || buildDefaultStrategyName(activeNavLabel, visibleStrategies);
      const resolvedPriority = values.priority ?? editingStrategy?.priority ?? 1;
      const resolvedSplitCount = isAutoSplitStrategy
        ? values.splitCount ?? editingStrategy?.splitCount ?? 2
        : undefined;
      const resolvedMergeCount = isAutoMergeStrategy
        ? values.mergeCount ?? editingStrategy?.mergeCount ?? 2
        : undefined;
      const normalizedSalesChannels = (values.salesChannel ?? [])
        .map((item) => item.trim())
        .filter(Boolean);
      const normalizedIncludeRules = normalizeMatchRules(values.logisticsIncludeRules);
      const normalizedExcludeRules = normalizeMatchRules(values.logisticsExcludeRules);
      const normalizedWarehouseIncludeRules = normalizeMatchRules(values.warehouseIncludeRules);
      const normalizedWarehouseExcludeRules = normalizeMatchRules(values.warehouseExcludeRules);
      const [logisticsValidFromValue, logisticsValidToValue] = values.logisticsValidRange ?? [];
      const logisticsValidFrom = logisticsValidFromValue
        ? logisticsValidFromValue.format('YYYY-MM-DD')
        : undefined;
      const logisticsValidTo = logisticsValidToValue
        ? logisticsValidToValue.format('YYYY-MM-DD')
        : undefined;
      const [warehouseValidFromValue, warehouseValidToValue] = values.warehouseValidRange ?? [];
      const warehouseValidFrom = warehouseValidFromValue
        ? warehouseValidFromValue.format('YYYY-MM-DD')
        : undefined;
      const warehouseValidTo = warehouseValidToValue
        ? warehouseValidToValue.format('YYYY-MM-DD')
        : undefined;
      const nextStrategy: SalesStrategy = {
        id: editingStrategy?.id ?? buildStrategyId(),
        type: editingStrategy?.type ?? activeNavKey,
        name: resolvedName,
        priority: resolvedPriority,
        remark: values.remark?.trim() || undefined,
        salesChannel: normalizedSalesChannels.length ? normalizedSalesChannels : undefined,
        salesOrderType: values.salesOrderType?.trim() || undefined,
        salesOrderTag: values.salesOrderTag?.trim() || undefined,
        commissionMatchMode: values.commissionMatchMode?.trim() || undefined,
        commissionAssignee: values.commissionAssignee?.trim() || undefined,
        commissionNotePrefix: values.commissionNotePrefix?.trim() || undefined,
        commissionNoteContent: values.commissionNoteContent?.trim() || undefined,
        commissionNoteSuffix: values.commissionNoteSuffix?.trim() || undefined,
        commissionFallbackAssignee: values.commissionFallbackAssignee?.trim() || undefined,
        commissionExceptionTag: values.commissionExceptionTag?.trim() || undefined,
        parseContent: values.parseContent?.trim() || undefined,
        rulePrefix: values.rulePrefix?.trim() || undefined,
        ruleSeparator: values.ruleSeparator?.trim() || undefined,
        ruleSuffix: values.ruleSuffix?.trim() || undefined,
        priceBasis: values.priceBasis?.trim() || undefined,
        priceDiscount: values.priceDiscount ?? undefined,
        parseErrorTag: values.parseErrorTag?.trim() || undefined,
        parseSuccessTag: values.parseSuccessTag?.trim() || undefined,
        autoExecute: values.autoExecute ?? false,
        splitCount: isAutoSplitStrategy ? resolvedSplitCount : undefined,
        mergeCount: isAutoMergeStrategy ? resolvedMergeCount : undefined,
        logisticsMatchMode: isLogisticsStrategy
          ? values.logisticsMatchMode?.trim() || undefined
          : undefined,
        logisticsIncludeRules: isLogisticsStrategy
          ? normalizedIncludeRules.length
            ? normalizedIncludeRules
            : undefined
          : undefined,
        logisticsExcludeRules: isLogisticsStrategy
          ? normalizedExcludeRules.length
            ? normalizedExcludeRules
            : undefined
          : undefined,
        logisticsValidFrom: isLogisticsStrategy ? logisticsValidFrom : undefined,
        logisticsValidTo: isLogisticsStrategy ? logisticsValidTo : undefined,
        logisticsSmartSelect: isLogisticsStrategy ? values.logisticsSmartSelect ?? false : undefined,
        warehouseMatchMode: isWarehouseStrategy
          ? values.warehouseMatchMode?.trim() || undefined
          : undefined,
        warehouseIncludeRules: isWarehouseStrategy
          ? normalizedWarehouseIncludeRules.length
            ? normalizedWarehouseIncludeRules
            : undefined
          : undefined,
        warehouseExcludeRules: isWarehouseStrategy
          ? normalizedWarehouseExcludeRules.length
            ? normalizedWarehouseExcludeRules
            : undefined
          : undefined,
        warehouseValidFrom: isWarehouseStrategy ? warehouseValidFrom : undefined,
        warehouseValidTo: isWarehouseStrategy ? warehouseValidTo : undefined,
        warehouseSmartSelect: isWarehouseStrategy ? values.warehouseSmartSelect ?? false : undefined,
        executionStrategy: values.executionStrategy?.trim() || undefined,
        executeTime: values.executeTime ? values.executeTime.format('HH:mm') : undefined,
        validPeriod: values.validPeriod ?? 'forever',
        enabled: editingStrategy?.enabled ?? true,
        conditions: normalizeConditions(values.conditions),
        exclusions: normalizeConditions(values.exclusions),
        updatedAt: now
      };
      setStrategies((prev) => {
        if (editingStrategy) {
          return prev.map((item) => (item.id === editingStrategy.id ? nextStrategy : item));
        }
        return [...prev, nextStrategy];
      });
      setModalOpen(false);
      message.success(editingStrategy ? '策略已更新' : '策略已添加');
    } catch {
      // ignore validation error
    }
  };

  const handleNavClick = (key: string) => {
    setActiveNavKey(key);
    setSelectedRowKeys([]);
  };

  const menuItems = useMemo<MenuProps['items']>(() => {
    const hasSelection = selectedRowKeys.length > 0;
    const canEditSelection = selectedRowKeys.length === 1;
    const hasEnabled = selectedStrategies.some((item) => item.enabled);
    const hasDisabled = selectedStrategies.some((item) => !item.enabled);
    return [
      { key: 'create', label: '新建', disabled: !canWrite },
      { key: 'edit', label: '修改', disabled: !canWrite || !canEditSelection },
      { key: 'delete', label: '删除', disabled: !canWrite || !hasSelection || hasEnabled },
      { key: 'disable', label: '停用', disabled: !canWrite || !hasSelection || !hasEnabled },
      { key: 'enable', label: '启用', disabled: !canWrite || !hasSelection || !hasDisabled }
    ];
  }, [canWrite, selectedRowKeys.length, selectedStrategies]);

  const handleMenuClick: MenuProps['onClick'] = ({ key }) => {
    if (key === 'create') {
      openCreateModal();
    }
    if (key === 'edit') {
      openEditModal();
    }
    if (key === 'delete') {
      handleDelete();
    }
    if (key === 'disable') {
      handleDisable();
    }
    if (key === 'enable') {
      handleEnable();
    }
  };

  return (
    <div className="sales-strategy-settings-page">
      <div className="sales-strategy-settings-nav">
        <Card>
          <Typography.Text strong>销售策略</Typography.Text>
          <div className="sales-strategy-settings-nav-list">
            {navItems.map((item) => (
              <Button
                key={item.key}
                type={item.key === activeNavKey ? 'primary' : 'text'}
                onClick={() => handleNavClick(item.key)}
                block
              >
                {item.label}
              </Button>
            ))}
          </div>
        </Card>
      </div>
      <div className="sales-strategy-settings-main">
        <Card>
          <div className="sales-strategy-settings-header">
            <Space wrap>
              <Typography.Text strong>{activeNavLabel}</Typography.Text>
              <Button type="primary" onClick={openCreateModal} disabled={!canWrite}>
                添加
              </Button>
              <Button
                onClick={openEditModal}
                disabled={!canWrite || selectedRowKeys.length !== 1}
              >
                修改
              </Button>
              <Button danger onClick={handleDisable} disabled={!canWrite}>
                停用
              </Button>
            </Space>
          </div>
          <UiModule06StrategyTable
            rowKey="id"
            columns={columns}
            data={visibleStrategies}
            selectedRowKeys={selectedRowKeys}
            onSelectionChange={setSelectedRowKeys}
            onRowClick={(record) => setSelectedRowKeys([record.id])}
            onRowDoubleClick={(record) => {
              setSelectedRowKeys([record.id]);
              openEditModal(record);
            }}
            onRowContextMenu={(record) => {
              if (!selectedRowKeys.includes(record.id)) {
                setSelectedRowKeys([record.id]);
              }
            }}
            menuItems={menuItems}
            onMenuClick={handleMenuClick}
            emptyText="暂无策略"
          />
        </Card>
      </div>
      <Modal
        title={editingStrategy ? '修改策略' : '添加策略'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSave}
        okText="保存"
        cancelText="取消"
        width={865}
        className="sales-strategy-settings-modal"
        okButtonProps={{ disabled: !canWrite }}
      >
        <Form
          layout="horizontal"
          form={form}
          labelAlign="left"
          labelCol={{ span: 8 }}
          wrapperCol={{ span: 16 }}
        >
          {isGoodsStrategy && (
            <>
              <Divider orientation="left">解析配置</Divider>
              <Row gutter={12}>
                <Col span={12}>
                  <Form.Item label="销售渠道" name="salesChannel">
                    <Select
                      allowClear
                      mode="multiple"
                      maxTagCount="responsive"
                      options={channelOptions}
                      placeholder="选择销售渠道"
                      showSearch
                      optionFilterProp="label"
                    />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item
                    name="autoExecute"
                    valuePropName="checked"
                    label="建单时自动执行"
                  >
                    <Checkbox />
                  </Form.Item>
                </Col>
              </Row>
              <Form.Item label="解析内容" name="parseContent">
                <Radio.Group
                  options={parseContentOptions}
                  optionType="button"
                  buttonStyle="solid"
                />
              </Form.Item>
              <Row gutter={12}>
                <Col span={8}>
                  <Form.Item label="前缀符号" name="rulePrefix">
                    <Input placeholder="例如 [" />
                  </Form.Item>
                </Col>
                <Col span={8}>
                  <Form.Item label="分隔符" name="ruleSeparator">
                    <Input placeholder="例如 ;" />
                  </Form.Item>
                </Col>
                <Col span={8}>
                  <Form.Item label="后缀符号" name="ruleSuffix">
                    <Input placeholder="例如 ]" />
                  </Form.Item>
                </Col>
              </Row>
              <Typography.Text type="secondary" className="sales-strategy-settings-section-hint">
                {parseRuleExample}
              </Typography.Text>
              <Divider orientation="left">执行设置</Divider>
              <Typography.Text type="secondary" className="sales-strategy-settings-inline-note">
                解析完成后按设置替换货品明细并标记订单。
              </Typography.Text>
              <Row gutter={12}>
                <Col span={12}>
                  <Form.Item label="基准价格" name="priceBasis">
                    <Select options={priceBasisOptions} />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="折扣(%)" name="priceDiscount">
                    <InputNumber
                      min={0}
                      max={100}
                      precision={0}
                      formatter={(value) => `${value}%`}
                      parser={(value) => value?.replace('%', '') ?? ''}
                      style={{ width: '100%' }}
                    />
                  </Form.Item>
                </Col>
              </Row>
              <Row gutter={12}>
                <Col span={12}>
                  <Form.Item label="失败标记" name="parseErrorTag">
                    <Select
                      allowClear
                      options={orderTagOptions}
                      placeholder="选择失败标记"
                      showSearch
                      optionFilterProp="label"
                    />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="成功标记" name="parseSuccessTag">
                    <Select
                      allowClear
                      options={orderTagOptions}
                      placeholder="选择成功标记"
                      showSearch
                      optionFilterProp="label"
                    />
                  </Form.Item>
                </Col>
              </Row>
            </>
          )}
          {isSalesCommissionStrategy && (
            <>
              <Divider orientation="left">提成条件</Divider>
              <Row gutter={16}>
                <Col span={12}>
                  <Form.Item label="适用渠道" name="salesChannel">
                    <Select
                      allowClear
                      mode="multiple"
                      maxTagCount="responsive"
                      options={channelOptions}
                      placeholder="选择销售渠道"
                      showSearch
                      optionFilterProp="label"
                    />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="订单类型" name="salesOrderType">
                    <Select
                      allowClear
                      options={commissionOrderTypeOptions}
                      placeholder="选择订单类型"
                    />
                  </Form.Item>
                </Col>
              </Row>
              <Row gutter={16}>
                <Col span={12}>
                  <Form.Item label="订单标记" name="salesOrderTag">
                    <Select
                      allowClear
                      options={orderTagOptions}
                      placeholder="选择订单标记"
                      showSearch
                      optionFilterProp="label"
                    />
                  </Form.Item>
                </Col>
              </Row>
              <Divider orientation="left">归属规则</Divider>
              <Form.Item label="归属方式" name="commissionMatchMode">
                <Radio.Group>
                  <Space direction="vertical">
                    {commissionMatchOptions.map((option) => (
                      <Radio key={option.value} value={option.value}>
                        {option.label}
                      </Radio>
                    ))}
                  </Space>
                </Radio.Group>
              </Form.Item>
              {commissionMatchMode === 'fixed_salesperson' && (
                <Form.Item label="指定业务员" name="commissionAssignee">
                  <Input placeholder="输入业务员名称" />
                </Form.Item>
              )}
              {commissionMatchMode === 'note_extract' && (
                <>
                  <Row gutter={12}>
                    <Col span={8}>
                      <Form.Item label="前缀" name="commissionNotePrefix">
                        <Input placeholder="#" />
                      </Form.Item>
                    </Col>
                    <Col span={8}>
                      <Form.Item label="字段" name="commissionNoteContent">
                        <Select
                          options={commissionNoteContentOptions}
                          placeholder="选择字段"
                        />
                      </Form.Item>
                    </Col>
                    <Col span={8}>
                      <Form.Item label="后缀" name="commissionNoteSuffix">
                        <Input placeholder="#" />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Typography.Text type="secondary" className="sales-strategy-settings-section-hint">
                    {commissionNoteExample}
                  </Typography.Text>
                </>
              )}
              <Form.Item label="默认业务员" name="commissionFallbackAssignee">
                <Input placeholder="未命中时使用" />
              </Form.Item>
              <Divider orientation="left">异常处理</Divider>
              <Form.Item label="异常标记" name="commissionExceptionTag">
                <Select
                  allowClear
                  options={orderTagOptions}
                  placeholder="选择异常标记"
                  showSearch
                  optionFilterProp="label"
                />
              </Form.Item>
            </>
          )}
          {isLogisticsStrategy && (
            <>
              <Divider orientation="left">匹配规则</Divider>
              <Form.Item label="命中方式" name="logisticsMatchMode">
                <Radio.Group options={logisticsMatchModeOptions} />
              </Form.Item>
              <Typography.Text type="secondary" className="sales-strategy-settings-section-hint">
                不限制时可跳过区域规则，按执行策略直接匹配。
              </Typography.Text>
              <div className="sales-strategy-settings-logistics-block">
                <div className="sales-strategy-settings-condition-title">
                  <Typography.Text strong>包含区域</Typography.Text>
                  <Typography.Text type="secondary" className="sales-strategy-settings-condition-hint">
                    命中后进入物流匹配
                  </Typography.Text>
                </div>
                <Form.List name="logisticsIncludeRules">
                  {(fields, { add, remove }) => (
                    <Space direction="vertical" size={8} style={{ width: '100%' }}>
                      {fields.map((field) => (
                        <div className="sales-strategy-settings-logistics-rule-row" key={field.key}>
                          <Form.Item
                            className="sales-strategy-settings-logistics-rule-field"
                            name={[field.name, 'scope']}
                          >
                            <Select
                              options={logisticsRegionScopeOptions}
                              placeholder="区域维度"
                              disabled={logisticsMatchUnlimited}
                            />
                          </Form.Item>
                          <Form.Item
                            className="sales-strategy-settings-logistics-rule-value"
                            name={[field.name, 'areas']}
                          >
                            <Select
                              allowClear
                              mode="tags"
                              maxTagCount="responsive"
                              options={logisticsAreaOptions}
                              placeholder="输入或选择区域"
                              showSearch
                              optionFilterProp="label"
                              disabled={logisticsMatchUnlimited}
                            />
                          </Form.Item>
                          <Button
                            type="text"
                            danger
                            disabled={logisticsMatchUnlimited}
                            onClick={() => remove(field.name)}
                          >
                            删除
                          </Button>
                        </div>
                      ))}
                      <Button
                        type="dashed"
                        onClick={() => add({})}
                        block
                        disabled={logisticsMatchUnlimited}
                      >
                        添加区域
                      </Button>
                    </Space>
                  )}
                </Form.List>
              </div>
              <div className="sales-strategy-settings-logistics-block">
                <div className="sales-strategy-settings-condition-title">
                  <Typography.Text strong>排除区域</Typography.Text>
                  <Typography.Text type="secondary" className="sales-strategy-settings-condition-hint">
                    命中即跳过匹配
                  </Typography.Text>
                </div>
                <Form.List name="logisticsExcludeRules">
                  {(fields, { add, remove }) => (
                    <Space direction="vertical" size={8} style={{ width: '100%' }}>
                      {fields.map((field) => (
                        <div className="sales-strategy-settings-logistics-rule-row" key={field.key}>
                          <Form.Item
                            className="sales-strategy-settings-logistics-rule-field"
                            name={[field.name, 'scope']}
                          >
                            <Select
                              options={logisticsRegionScopeOptions}
                              placeholder="区域维度"
                              disabled={logisticsMatchUnlimited}
                            />
                          </Form.Item>
                          <Form.Item
                            className="sales-strategy-settings-logistics-rule-value"
                            name={[field.name, 'areas']}
                          >
                            <Select
                              allowClear
                              mode="tags"
                              maxTagCount="responsive"
                              options={logisticsAreaOptions}
                              placeholder="输入或选择区域"
                              showSearch
                              optionFilterProp="label"
                              disabled={logisticsMatchUnlimited}
                            />
                          </Form.Item>
                          <Button
                            type="text"
                            danger
                            disabled={logisticsMatchUnlimited}
                            onClick={() => remove(field.name)}
                          >
                            删除
                          </Button>
                        </div>
                      ))}
                      <Button
                        type="dashed"
                        onClick={() => add({})}
                        block
                        disabled={logisticsMatchUnlimited}
                      >
                        添加区域
                      </Button>
                    </Space>
                  )}
                </Form.List>
              </div>
              <Row gutter={12} className="sales-strategy-settings-section">
                <Col span={12}>
                  <Form.Item label="有效期范围" name="logisticsValidRange">
                    <DatePicker.RangePicker style={{ width: '100%' }} />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item
                    label="参与智能选快递"
                    name="logisticsSmartSelect"
                    valuePropName="checked"
                  >
                    <Checkbox />
                  </Form.Item>
                </Col>
              </Row>
            </>
          )}
          {isWarehouseStrategy && (
            <>
              <Divider orientation="left">仓库匹配</Divider>
              <Form.Item label="匹配方式" name="warehouseMatchMode">
                <Radio.Group options={logisticsMatchModeOptions} />
              </Form.Item>
              <Typography.Text type="secondary" className="sales-strategy-settings-section-hint">
                不限制时直接按目标仓库更新订单。
              </Typography.Text>
              <div className="sales-strategy-settings-logistics-block">
                <div className="sales-strategy-settings-condition-title">
                  <Typography.Text strong>适用区域</Typography.Text>
                  <Typography.Text type="secondary" className="sales-strategy-settings-condition-hint">
                    命中后执行仓库调整
                  </Typography.Text>
                </div>
                <Form.List name="warehouseIncludeRules">
                  {(fields, { add, remove }) => (
                    <Space direction="vertical" size={8} style={{ width: '100%' }}>
                      {fields.map((field) => (
                        <div className="sales-strategy-settings-logistics-rule-row" key={field.key}>
                          <Form.Item
                            className="sales-strategy-settings-logistics-rule-field"
                            name={[field.name, 'scope']}
                          >
                            <Select
                              options={logisticsRegionScopeOptions}
                              placeholder="区域维度"
                              disabled={warehouseMatchUnlimited}
                            />
                          </Form.Item>
                          <Form.Item
                            className="sales-strategy-settings-logistics-rule-value"
                            name={[field.name, 'areas']}
                          >
                            <Select
                              allowClear
                              mode="tags"
                              maxTagCount="responsive"
                              options={logisticsAreaOptions}
                              placeholder="选择或输入区域"
                              showSearch
                              optionFilterProp="label"
                              disabled={warehouseMatchUnlimited}
                            />
                          </Form.Item>
                          <Button
                            type="text"
                            danger
                            disabled={warehouseMatchUnlimited}
                            onClick={() => remove(field.name)}
                          >
                            删除
                          </Button>
                        </div>
                      ))}
                      <Button
                        type="dashed"
                        onClick={() => add({})}
                        block
                        disabled={warehouseMatchUnlimited}
                      >
                        添加区域
                      </Button>
                    </Space>
                  )}
                </Form.List>
              </div>
              <div className="sales-strategy-settings-logistics-block">
                <div className="sales-strategy-settings-condition-title">
                  <Typography.Text strong>禁用区域</Typography.Text>
                  <Typography.Text type="secondary" className="sales-strategy-settings-condition-hint">
                    命中即跳过仓库调整
                  </Typography.Text>
                </div>
                <Form.List name="warehouseExcludeRules">
                  {(fields, { add, remove }) => (
                    <Space direction="vertical" size={8} style={{ width: '100%' }}>
                      {fields.map((field) => (
                        <div className="sales-strategy-settings-logistics-rule-row" key={field.key}>
                          <Form.Item
                            className="sales-strategy-settings-logistics-rule-field"
                            name={[field.name, 'scope']}
                          >
                            <Select
                              options={logisticsRegionScopeOptions}
                              placeholder="区域维度"
                              disabled={warehouseMatchUnlimited}
                            />
                          </Form.Item>
                          <Form.Item
                            className="sales-strategy-settings-logistics-rule-value"
                            name={[field.name, 'areas']}
                          >
                            <Select
                              allowClear
                              mode="tags"
                              maxTagCount="responsive"
                              options={logisticsAreaOptions}
                              placeholder="选择或输入区域"
                              showSearch
                              optionFilterProp="label"
                              disabled={warehouseMatchUnlimited}
                            />
                          </Form.Item>
                          <Button
                            type="text"
                            danger
                            disabled={warehouseMatchUnlimited}
                            onClick={() => remove(field.name)}
                          >
                            删除
                          </Button>
                        </div>
                      ))}
                      <Button
                        type="dashed"
                        onClick={() => add({})}
                        block
                        disabled={warehouseMatchUnlimited}
                      >
                        添加区域
                      </Button>
                    </Space>
                  )}
                </Form.List>
              </div>
              <Row gutter={12} className="sales-strategy-settings-section">
                <Col span={12}>
                  <Form.Item label="生效区间" name="warehouseValidRange">
                    <DatePicker.RangePicker style={{ width: '100%' }} />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item
                    label="参与智能选仓"
                    name="warehouseSmartSelect"
                    valuePropName="checked"
                  >
                    <Checkbox />
                  </Form.Item>
                </Col>
              </Row>
            </>
          )}
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <div>
              <div className="sales-strategy-settings-condition-title">
                <Typography.Text strong>满足条件</Typography.Text>
                <Typography.Text type="secondary" className="sales-strategy-settings-condition-hint">
                  满足任意条件
                </Typography.Text>
              </div>
              <Form.List name="conditions">
                {(fields, { add, remove }) => (
                  <Space direction="vertical" size={8} style={{ width: '100%' }}>
                    {fields.map((field) => (
                      <div className="sales-strategy-settings-condition-row" key={field.key}>
                        <Form.Item
                          className="sales-strategy-settings-condition-field"
                          name={[field.name, 'field']}
                        >
                          <Select
                            options={conditionFieldOptions}
                            placeholder="条件字段"
                            onChange={() => handleConditionFieldChange('conditions', field.name)}
                          />
                        </Form.Item>
                        <Form.Item
                          className="sales-strategy-settings-condition-operator"
                          name={[field.name, 'operator']}
                        >
                          <Select options={operatorOptions} placeholder="操作" />
                        </Form.Item>
                        <ConditionValueField listName="conditions" fieldIndex={field.name} />
                        <Button
                          className="sales-strategy-settings-condition-remove"
                          type="text"
                          danger
                          onClick={() => remove(field.name)}
                        >
                          删除
                        </Button>
                      </div>
                    ))}
                    <Button type="dashed" onClick={() => add({})} block>
                      添加条件
                    </Button>
                  </Space>
                )}
              </Form.List>
            </div>
            <div>
              <div className="sales-strategy-settings-condition-title">
                <Typography.Text strong>排除条件</Typography.Text>
                <Typography.Text type="secondary" className="sales-strategy-settings-condition-hint">
                  满足任意条件
                </Typography.Text>
              </div>
              <Form.List name="exclusions">
                {(fields, { add, remove }) => (
                  <Space direction="vertical" size={8} style={{ width: '100%' }}>
                    {fields.map((field) => (
                      <div className="sales-strategy-settings-condition-row" key={field.key}>
                        <Form.Item
                          className="sales-strategy-settings-condition-field"
                          name={[field.name, 'field']}
                        >
                          <Select
                            options={conditionFieldOptions}
                            placeholder="条件字段"
                            onChange={() => handleConditionFieldChange('exclusions', field.name)}
                          />
                        </Form.Item>
                        <Form.Item
                          className="sales-strategy-settings-condition-operator"
                          name={[field.name, 'operator']}
                        >
                          <Select options={operatorOptions} placeholder="操作" />
                        </Form.Item>
                        <ConditionValueField listName="exclusions" fieldIndex={field.name} />
                        <Button
                          className="sales-strategy-settings-condition-remove"
                          type="text"
                          danger
                          onClick={() => remove(field.name)}
                        >
                          删除
                        </Button>
                      </div>
                    ))}
                    <Button type="dashed" onClick={() => add({})} block>
                      添加条件
                    </Button>
                  </Space>
                )}
              </Form.List>
            </div>
            <div>
              <Typography.Text strong>执行策略</Typography.Text>
              {isLogisticsStrategy && (
                <Form.Item name="executionStrategy" style={{ marginBottom: 0, marginTop: 8 }}>
                  <Select
                    allowClear
                    options={logisticsCompanyOptions}
                    placeholder="选择匹配物流公司"
                    showSearch
                    optionFilterProp="label"
                  />
                </Form.Item>
              )}
              {isWarehouseStrategy && (
                <Form.Item name="executionStrategy" style={{ marginBottom: 0, marginTop: 8 }}>
                  <Select
                    allowClear
                    options={warehouseOptions}
                    placeholder="选择目标仓库"
                    showSearch
                    optionFilterProp="label"
                  />
                </Form.Item>
              )}
              {!isLogisticsStrategy && !isWarehouseStrategy && (
                <Form.Item name="executionStrategy" style={{ marginBottom: 0, marginTop: 8 }}>
                  <Input.TextArea rows={3} placeholder="请输入执行策略" />
                </Form.Item>
              )}
            </div>
          </Space>
          <div className="sales-strategy-settings-footer">
            <Row gutter={8}>
              <Col span={10}>
                <Form.Item label="策略名称" name="name">
                  <Input placeholder="策略名称" />
                </Form.Item>
              </Col>
              <Col span={4}>
                <Form.Item
                  label="优先级(越大越优先)"
                  name="priority"
                  rules={[
                    {
                      validator: (_, value) =>
                        value == null || Number.isInteger(value)
                          ? Promise.resolve()
                          : Promise.reject(new Error('优先级只能填写整数'))
                    }
                  ]}
                >
                  <InputNumber min={1} precision={0} step={1} style={{ width: '100%' }} />
                </Form.Item>
              </Col>
              {isAutoSplitStrategy && (
                <Col span={4}>
                  <Form.Item
                    label="拆分数量"
                    name="splitCount"
                    rules={[{ required: true, message: '请输入拆分数量' }]}
                  >
                    <InputNumber min={2} precision={0} style={{ width: '100%' }} />
                  </Form.Item>
                </Col>
              )}
              {isAutoMergeStrategy && (
                <Col span={4}>
                  <Form.Item
                    label="合并数量"
                    name="mergeCount"
                    rules={[{ required: true, message: '请输入合并数量' }]}
                  >
                    <InputNumber min={2} precision={0} style={{ width: '100%' }} />
                  </Form.Item>
                </Col>
              )}
              <Col span={isAutoSplitStrategy || isAutoMergeStrategy ? 3 : 5}>
                <Form.Item label="执行时间" name="executeTime">
                  <TimePicker format="HH:mm" style={{ width: '100%' }} />
                </Form.Item>
              </Col>
              <Col span={isAutoSplitStrategy || isAutoMergeStrategy ? 3 : 5}>
                <Form.Item label="有效期" name="validPeriod">
                  <Select options={validPeriodOptions} />
                </Form.Item>
              </Col>
            </Row>
            <Row gutter={8}>
              <Col span={24}>
                <Form.Item label="备注" name="remark">
                  <Input.TextArea rows={2} placeholder="备注" />
                </Form.Item>
              </Col>
            </Row>
          </div>
        </Form>
      </Modal>
    </div>
  );
};
