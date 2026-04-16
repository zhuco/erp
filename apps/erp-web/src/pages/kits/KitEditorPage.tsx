import {
  Alert,
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Form,
  InputNumber,
  Row,
  Select,
  Space,
  Table,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs, { type Dayjs } from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';

import type { KitComponent, KitDetail } from '@/api/kits';
import { getKitDetail, saveKitBom } from '@/api/kits';
import type { SkuKind, SkuSearchItem } from '@/api/skus';
import { searchSkus } from '@/api/skus';
import { ErrorState, LoadingState } from '@/components/QueryState';
import { WorkbenchMainTableTemplate } from '@/components/workbench/templates';
import { useTenantId } from '@/features/tenant';

type ComponentFormValues = {
  component_sku_id?: string;
  qty?: number;
  expire_at?: Dayjs | null;
};

type SaveSummary = {
  added: number;
  removed: number;
  updated: number;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatSkuLabel = (sku: SkuSearchItem) => {
  const name = sku.name ? ` ${sku.name}` : '';
  const barcode = sku.barcode ? ` (${sku.barcode})` : '';
  return `${sku.sku_code}${name}${barcode}`;
};

const buildSkuOptions = (skus: SkuSearchItem[]) => {
  const seen = new Set<string>();
  return skus.reduce<{ value: string; label: string }[]>((acc, sku) => {
    if (seen.has(sku.sku_id)) {
      return acc;
    }
    seen.add(sku.sku_id);
    acc.push({ value: sku.sku_id, label: formatSkuLabel(sku) });
    return acc;
  }, []);
};

const diffComponents = (before: KitComponent[], after: KitComponent[]): SaveSummary => {
  const beforeMap = new Map(before.map((item) => [item.component_sku_id, item]));
  const afterMap = new Map(after.map((item) => [item.component_sku_id, item]));
  let added = 0;
  let removed = 0;
  let updated = 0;
  beforeMap.forEach((item, key) => {
    const next = afterMap.get(key);
    if (!next) {
      removed += 1;
      return;
    }
    if (next.qty !== item.qty || next.expire_at !== item.expire_at) {
      updated += 1;
    }
  });
  afterMap.forEach((item, key) => {
    if (!beforeMap.has(key)) {
      added += 1;
    }
  });
  return { added, removed, updated };
};

export const KitEditorPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const { kit_sku_id: kitSkuId } = useParams();
  const [form] = Form.useForm<ComponentFormValues>();
  const [kitSku, setKitSku] = useState<SkuSearchItem | null>(null);
  const [components, setComponents] = useState<KitComponent[]>([]);
  const [savedSnapshot, setSavedSnapshot] = useState<KitDetail | null>(null);
  const [saveSummary, setSaveSummary] = useState<SaveSummary | null>(null);
  const [saving, setSaving] = useState(false);
  const [initialized, setInitialized] = useState(false);
  const [kitOptions, setKitOptions] = useState<SkuSearchItem[]>([]);
  const [componentOptions, setComponentOptions] = useState<SkuSearchItem[]>([]);
  const [kitSearching, setKitSearching] = useState(false);
  const [componentSearching, setComponentSearching] = useState(false);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['kit-detail', tenantId, kitSkuId],
    queryFn: () => getKitDetail(tenantId, kitSkuId ?? ''),
    enabled: Boolean(kitSkuId)
  });
  const detailLoading = (isLoading || isFetching) && !data;

  useEffect(() => {
    setInitialized(false);
    setKitSku(null);
    setComponents([]);
    setSavedSnapshot(null);
    setSaveSummary(null);
  }, [kitSkuId]);

  useEffect(() => {
    if (!data || initialized) {
      return;
    }
    setKitSku(data.kit_sku);
    setComponents(data.components);
    setSavedSnapshot(data);
    setInitialized(true);
  }, [data, initialized]);

  const skuIndex = useMemo(() => {
    const map = new Map<string, SkuSearchItem>();
    [...kitOptions, ...componentOptions, kitSku].forEach((sku) => {
      if (sku) {
        map.set(sku.sku_id, sku);
      }
    });
    return map;
  }, [kitOptions, componentOptions, kitSku]);

  const loadSkuOptions = async (
    keyword: string,
    skuKind: SkuKind | undefined,
    setter: (items: SkuSearchItem[]) => void,
    setLoading: (value: boolean) => void
  ) => {
    setLoading(true);
    try {
      const response = await searchSkus(tenantId, keyword, skuKind);
      setter(response.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'SKU 搜索失败');
    } finally {
      setLoading(false);
    }
  };

  const handleAddComponent = (values: ComponentFormValues) => {
    const componentId = values.component_sku_id;
    const qty = values.qty ?? 0;
    const expireAt = values.expire_at ? values.expire_at.toISOString() : null;
    if (!kitSku) {
      message.error('请先选择 Kit SKU');
      return;
    }
    if (!componentId) {
      message.error('请选择组件 SKU');
      return;
    }
    if (componentId === kitSku.sku_id) {
      message.error('组件 SKU 不能与 Kit SKU 相同');
      return;
    }
    if (components.some((item) => item.component_sku_id === componentId)) {
      message.error('组件 SKU 已存在');
      return;
    }
    if (qty <= 0) {
      message.error('组件数量必须大于 0');
      return;
    }
    const sku = skuIndex.get(componentId);
    if (!sku) {
      message.error('未找到组件 SKU 信息');
      return;
    }
    setComponents((prev) => [
      ...prev,
      {
        component_sku_id: sku.sku_id,
        component_sku_code: sku.sku_code,
        component_name: sku.name ?? null,
        component_barcode: sku.barcode ?? null,
        qty,
        expire_at: expireAt
      }
    ]);
    form.resetFields(['component_sku_id', 'expire_at']);
    form.setFieldsValue({ qty: 1 });
  };

  const handleQtyChange = (componentId: string, qty?: number | null) => {
    if (!qty || qty <= 0) {
      return;
    }
    setComponents((prev) =>
      prev.map((item) =>
        item.component_sku_id === componentId
          ? {
              ...item,
              qty
            }
          : item
      )
    );
  };

  const handleExpireChange = (componentId: string, value?: Dayjs | null) => {
    setComponents((prev) =>
      prev.map((item) =>
        item.component_sku_id === componentId
          ? {
              ...item,
              expire_at: value ? value.toISOString() : null
            }
          : item
      )
    );
  };

  const handleRemove = (componentId: string) => {
    setComponents((prev) => prev.filter((item) => item.component_sku_id !== componentId));
  };

  const handleSave = async () => {
    if (!kitSku) {
      message.error('请选择 Kit SKU');
      return;
    }
    if (components.some((item) => item.component_sku_id === kitSku.sku_id)) {
      message.error('组件 SKU 不能与 Kit SKU 相同');
      return;
    }
    if (components.some((item) => item.qty <= 0)) {
      message.error('组件数量必须大于 0');
      return;
    }
    setSaving(true);
    try {
      const previous = savedSnapshot?.components ?? [];
      const result = await saveKitBom(tenantId, {
        kit_sku_id: kitSku.sku_id,
        components: components.map((item) => ({
          component_sku_id: item.component_sku_id,
          qty: item.qty,
          expire_at: item.expire_at ?? null
        }))
      });
      setSavedSnapshot(result);
      setComponents(result.components);
      setKitSku(result.kit_sku);
      setSaveSummary(diffComponents(previous, result.components));
      message.success('BOM 已保存');
      if (kitSkuId && kitSkuId !== result.kit_sku.sku_id) {
        navigate(`/kits/${result.kit_sku.sku_id}`, { replace: true });
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const componentColumns = useMemo<ColumnsType<KitComponent>>(
    () => [
      {
        title: '组件 SKU',
        dataIndex: 'component_sku_code',
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <span>{record.component_sku_code}</span>
            <span>{record.component_name ?? '-'}</span>
          </Space>
        )
      },
      {
        title: '条码',
        dataIndex: 'component_barcode',
        render: (_, record) => record.component_barcode ?? '-'
      },
      {
        title: '数量',
        dataIndex: 'qty',
        align: 'right',
        render: (_, record) => (
          <InputNumber
            min={1}
            value={record.qty}
            onChange={(value) => handleQtyChange(record.component_sku_id, value)}
          />
        )
      },
      {
        title: '有效期',
        dataIndex: 'expire_at',
        render: (_, record) => (
          <DatePicker
            value={record.expire_at ? dayjs(record.expire_at) : null}
            onChange={(value) => handleExpireChange(record.component_sku_id, value)}
          />
        )
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button size="small" onClick={() => handleRemove(record.component_sku_id)}>
            移除
          </Button>
        )
      }
    ],
    [handleExpireChange, handleQtyChange, handleRemove]
  );

  const savedComponents = savedSnapshot?.components ?? [];
  const savedColumns = useMemo<ColumnsType<KitComponent>>(
    () => [
      {
        title: '组件 SKU',
        dataIndex: 'component_sku_code',
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <span>{record.component_sku_code}</span>
            <span>{record.component_name ?? '-'}</span>
          </Space>
        )
      },
      {
        title: '条码',
        dataIndex: 'component_barcode',
        render: (_, record) => record.component_barcode ?? '-'
      },
      {
        title: '数量',
        dataIndex: 'qty',
        align: 'right'
      },
      {
        title: '有效期',
        dataIndex: 'expire_at',
        render: (_, record) => (record.expire_at ? formatDateTime(record.expire_at) : '-')
      }
    ],
    []
  );

  if (!kitSkuId) {
    return (
      <ErrorState
        title="缂哄皯 Kit SKU"
        description="无法加载组合装详情。"
        hint="请返回组合装列表重新选择。"
        action={<Button onClick={() => navigate('/kits')}>返回列表</Button>}
      />
    );
  }

  if (error) {
    return <ErrorState title="组合装详情加载失败" error={error} onRetry={() => refetch()} />;
  }

  return (
    <WorkbenchMainTableTemplate
      header={
        <Typography.Title level={3} data-testid="kit-editor-title">
          组合装编辑
        </Typography.Title>
      }
      tableContent={
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <LoadingState loading={detailLoading} onRetry={() => refetch()} />
          <Card>
            <Form layout="vertical">
              <Row gutter={16}>
                <Col xs={24} md={12}>
                  <Form.Item label="Kit SKU">
                    <Select
                      showSearch
                      allowClear
                      placeholder="搜索 Kit SKU"
                      options={buildSkuOptions([...kitOptions, ...(kitSku ? [kitSku] : [])])}
                      filterOption={false}
                      loading={kitSearching}
                      onSearch={(value) => loadSkuOptions(value, 'KIT', setKitOptions, setKitSearching)}
                      onFocus={() => loadSkuOptions('', 'KIT', setKitOptions, setKitSearching)}
                      value={kitSku?.sku_id}
                      onChange={(value) => {
                        const next = value ? skuIndex.get(value) ?? null : null;
                        if (next && next.sku_kind && next.sku_kind !== 'KIT') {
                          message.error('请选择 KIT 类型 SKU');
                          return;
                        }
                        setKitSku(next);
                      }}
                      data-testid="kit-sku-select"
                    />
                  </Form.Item>
                </Col>
                <Col xs={24} md={12}>
                  <Form.Item label="当前组件数量">
                    <InputNumber value={components.length} disabled style={{ width: '100%' }} />
                  </Form.Item>
                </Col>
              </Row>
            </Form>
            <Form
              form={form}
              layout="inline"
              onFinish={handleAddComponent}
              initialValues={{ qty: 1 }}
            >
              <Form.Item
                name="component_sku_id"
                rules={[{ required: true, message: '请选择组件 SKU' }]}
              >
                <Select
                  showSearch
                  allowClear
                  placeholder="搜索组件 SKU/条码/名称"
                  options={buildSkuOptions(componentOptions)}
                  filterOption={false}
                  loading={componentSearching}
                  onSearch={(value) =>
                    loadSkuOptions(value, undefined, setComponentOptions, setComponentSearching)
                  }
                  onFocus={() =>
                    loadSkuOptions('', undefined, setComponentOptions, setComponentSearching)
                  }
                  data-testid="kit-component-select"
                />
              </Form.Item>
              <Form.Item
                name="qty"
                rules={[{ required: true, message: '请输入数量' }]}
              >
                <InputNumber min={1} data-testid="kit-component-qty" />
              </Form.Item>
              <Form.Item name="expire_at">
                <DatePicker placeholder="有效期" />
              </Form.Item>
              <Form.Item>
                <Button type="primary" htmlType="submit" data-testid="kit-component-add">
                  添加组件
                </Button>
              </Form.Item>
            </Form>
          </Card>
          <Card title="BOM 组件">
            <Table<KitComponent>
              rowKey="component_sku_id"
              columns={componentColumns}
              dataSource={components}
              pagination={false}
              loading={isLoading || isFetching}
            />
          </Card>
          <Space>
            <Button type="primary" loading={saving} onClick={handleSave} data-testid="kit-save-button">
              保存
            </Button>
            <Button onClick={() => navigate('/kits')}>返回列表</Button>
          </Space>
          {saveSummary ? (
            <Alert
              type="success"
              showIcon
              message="BOM 已保存"
              description={`新增 ${saveSummary.added}，移除 ${saveSummary.removed}，数量调整 ${saveSummary.updated}`}
            />
          ) : null}
          {savedSnapshot ? (
            <Card title="BOM 明细">
              <Descriptions column={2} size="small" bordered>
                <Descriptions.Item label="Kit SKU">
                  {savedSnapshot.kit_sku.sku_code}
                </Descriptions.Item>
                <Descriptions.Item label="组件数量">
                  {savedSnapshot.components.length}
                </Descriptions.Item>
                <Descriptions.Item label="更新时间">
                  {formatDateTime(savedSnapshot.updated_at)}
                </Descriptions.Item>
              </Descriptions>
              <Table<KitComponent>
                rowKey="component_sku_id"
                style={{ marginTop: 12 }}
                columns={savedColumns}
                dataSource={savedComponents}
                pagination={false}
              />
            </Card>
          ) : null}
        </Space>
      }
    />
  );
};

