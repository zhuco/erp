import {
  Button,
  Card,
  Checkbox,
  Col,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { WarehouseArea } from '@/api/warehouseAreas';
import { listWarehouseAreas } from '@/api/warehouseAreas';
import type {
  WarehouseLocation,
  WarehouseLocationBatchItem,
  WarehouseLocationBatchPayload,
  WarehouseLocationCreatePayload,
  WarehouseLocationListQuery,
  WarehouseLocationUpdatePayload
} from '@/api/warehouseLocations';
import {
  batchCreateWarehouseLocations,
  createWarehouseLocation,
  deleteWarehouseLocation,
  listWarehouseLocations,
  updateWarehouseLocation
} from '@/api/warehouseLocations';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  warehouse_id?: string;
  area_id?: string;
  keyword?: string;
  is_picking?: boolean;
  is_disabled?: boolean;
};

type FormValues = {
  warehouse_id: string;
  area_id?: string;
  code: string;
  name?: string;
  barcode?: string;
  volume?: number;
  max_weight?: number;
  is_picking?: boolean;
  is_disabled?: boolean;
};

type BatchFormValues = {
  warehouse_id: string;
  area_id?: string;
  raw: string;
};

const boolOptions = [
  { label: '是', value: true },
  { label: '否', value: false }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const parseBoolean = (value?: string) => {
  if (!value) {
    return undefined;
  }
  const normalized = value.trim().toLowerCase();
  if (['1', 'true', 'yes', 'y'].includes(normalized)) {
    return true;
  }
  if (['0', 'false', 'no', 'n'].includes(normalized)) {
    return false;
  }
  return undefined;
};

const parseNumber = (value?: string) => {
  if (!value) {
    return undefined;
  }
  const parsed = Number(value.trim());
  return Number.isFinite(parsed) ? parsed : undefined;
};

export const WarehouseLocationsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [batchForm] = Form.useForm<BatchFormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [batchOpen, setBatchOpen] = useState(false);
  const [editingLocation, setEditingLocation] = useState<WarehouseLocation | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const drawerWarehouseId = Form.useWatch('warehouse_id', drawerForm);
  const areaWarehouseId = drawerWarehouseId?.trim() || filters.warehouse_id?.trim();

  const areasQuery = useQuery({
    queryKey: ['warehouse-areas', tenantId, areaWarehouseId],
    queryFn: () =>
      listWarehouseAreas(tenantId, {
        warehouse_id: areaWarehouseId,
        offset: 0,
        limit: 200
      }),
    enabled: Boolean(areaWarehouseId)
  });

  const areaOptions = useMemo(
    () =>
      (areasQuery.data?.items ?? []).map((area: WarehouseArea) => ({
        label: `${area.code} ${area.name}`,
        value: area.area_id
      })),
    [areasQuery.data]
  );

  const areaMap = useMemo(() => {
    const map = new Map<string, WarehouseArea>();
    (areasQuery.data?.items ?? []).forEach((area) => {
      map.set(area.area_id, area);
    });
    return map;
  }, [areasQuery.data]);

  const query = useMemo<WarehouseLocationListQuery>(
    () => ({
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      area_id: filters.area_id?.trim() || undefined,
      keyword: filters.keyword?.trim() || undefined,
      is_picking: filters.is_picking,
      is_disabled: filters.is_disabled,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['warehouse-locations', tenantId, query],
    queryFn: () => listWarehouseLocations(tenantId, query),
    keepPreviousData: true
  });

  const deleteMutation = useMutation({
    mutationFn: (locationId: string) => deleteWarehouseLocation(tenantId, locationId),
    onMutate: (locationId) => setDeletingId(locationId),
    onSuccess: () => {
      message.success('货位已删除');
      queryClient.invalidateQueries({ queryKey: ['warehouse-locations', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除货位失败');
    },
    onSettled: () => setDeletingId(null)
  });

  const columns = useMemo<ColumnsType<WarehouseLocation>>(
    () => [
      { title: '货位编码', dataIndex: 'code' },
      { title: '仓库ID', dataIndex: 'warehouse_id' },
      { title: '货位名称', dataIndex: 'name', render: (value) => value ?? '-' },
      {
        title: '库区',
        dataIndex: 'area_id',
        render: (value) => {
          if (!value) {
            return '-';
          }
          const area = areaMap.get(value);
          return area ? `${area.code} ${area.name}` : value;
        }
      },
      { title: '条码', dataIndex: 'barcode', render: (value) => value ?? '-' },
      { title: '容积', dataIndex: 'volume', render: (value) => value ?? '-' },
      { title: '承重', dataIndex: 'max_weight', render: (value) => value ?? '-' },
      {
        title: '拣货位',
        dataIndex: 'is_picking',
        render: (value) => (value ? <Tag color="blue">是</Tag> : '否')
      },
      {
        title: '禁用',
        dataIndex: 'is_disabled',
        render: (value) => (value ? <Tag color="red">禁用</Tag> : '启用')
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setEditingLocation(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  warehouse_id: record.warehouse_id,
                  area_id: record.area_id ?? undefined,
                  code: record.code,
                  name: record.name ?? undefined,
                  barcode: record.barcode ?? undefined,
                  volume: record.volume ?? undefined,
                  max_weight: record.max_weight ?? undefined,
                  is_picking: record.is_picking,
                  is_disabled: record.is_disabled
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除该货位？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.location_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deleteMutation.isPending && deletingId === record.location_id}
              >
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [areaMap, canWrite, deleteMutation, deletingId, drawerForm]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    filterForm.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingLocation(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      is_picking: false,
      is_disabled: false
    });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      setSaving(true);
      if (editingLocation) {
        const payload: WarehouseLocationUpdatePayload = {
          area_id: values.area_id ?? undefined,
          name: values.name?.trim() || undefined,
          barcode: values.barcode?.trim() || undefined,
          volume: values.volume ?? undefined,
          max_weight: values.max_weight ?? undefined,
          is_picking: values.is_picking,
          is_disabled: values.is_disabled
        };
        await updateWarehouseLocation(tenantId, editingLocation.location_id, payload);
        message.success('货位已更新');
      } else {
        const payload: WarehouseLocationCreatePayload = {
          warehouse_id: values.warehouse_id.trim(),
          area_id: values.area_id?.trim() || undefined,
          code: values.code.trim(),
          name: values.name?.trim() || undefined,
          barcode: values.barcode?.trim() || undefined,
          volume: values.volume ?? undefined,
          max_weight: values.max_weight ?? undefined,
          is_picking: values.is_picking ?? false,
          is_disabled: values.is_disabled ?? false
        };
        await createWarehouseLocation(tenantId, payload);
        message.success('货位已创建');
      }
      setDrawerOpen(false);
      setEditingLocation(null);
      await queryClient.invalidateQueries({ queryKey: ['warehouse-locations', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleBatchImport = async () => {
    try {
      const values = await batchForm.validateFields();
      const lines = values.raw
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter(Boolean);
      const locations: WarehouseLocationBatchItem[] = [];
      lines.forEach((line, index) => {
        const parts = line.split(',').map((part) => part.trim());
        if (!parts[0]) {
          throw new Error(`第 ${index + 1} 行缺少货位编码`);
        }
        locations.push({
          code: parts[0],
          name: parts[1] || undefined,
          barcode: parts[2] || undefined,
          volume: parseNumber(parts[3]),
          max_weight: parseNumber(parts[4]),
          is_picking: parseBoolean(parts[5]),
          is_disabled: parseBoolean(parts[6]),
          area_id: parts[7] || undefined
        });
      });
      if (!locations.length) {
        message.error('未解析到有效货位');
        return;
      }
      const payload: WarehouseLocationBatchPayload = {
        warehouse_id: values.warehouse_id.trim(),
        area_id: values.area_id?.trim() || undefined,
        locations
      };
      await batchCreateWarehouseLocations(tenantId, payload);
      message.success('货位批量导入完成');
      setBatchOpen(false);
      batchForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['warehouse-locations', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    }
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        货位目录
      </Typography.Title>
      <Button
        onClick={() => {
          batchForm.resetFields();
          batchForm.setFieldsValue({
            warehouse_id: filters.warehouse_id,
            area_id: filters.area_id
          });
          setBatchOpen(true);
        }}
        disabled={!canWrite}
      >
        批量导入
      </Button>
      <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
        新建货位
      </Button>
    </Space>
  );

  const header = (
    <Card>
      <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item name="warehouse_id" label="仓库ID">
              <Input allowClear placeholder="仓库ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="area_id" label="库区">
              <Select allowClear options={areaOptions} placeholder="选择库区" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="is_picking" label="拣货位">
              <Select allowClear options={boolOptions} placeholder="全部" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="is_disabled" label="禁用">
              <Select allowClear options={boolOptions} placeholder="全部" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="keyword" label="关键词">
              <Input allowClear placeholder="编码/名称/条码" />
            </Form.Item>
          </Col>
          <Col xs={24} md={18} style={{ display: 'flex', alignItems: 'end' }}>
            <Space>
              <Button type="primary" htmlType="submit">
                查询
              </Button>
              <Button onClick={handleReset}>重置</Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Card>
  );

  return (
    <>
      <UiModule03MainTableSection<WarehouseLocation>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="location_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="货位列表加载失败"
        emptyText="暂无货位"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="warehouse-locations.table.columns"
      />
      <Drawer
        title={editingLocation ? '编辑货位' : '新建货位'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="warehouse_id"
            label="仓库ID"
            rules={[{ required: true, message: '请输入仓库ID' }]}
          >
            <Input disabled={Boolean(editingLocation)} />
          </Form.Item>
          <Form.Item name="area_id" label="库区">
            <Select allowClear options={areaOptions} placeholder="选择库区" />
          </Form.Item>
          <Form.Item
            name="code"
            label="货位编码"
            rules={[{ required: true, message: '请输入货位编码' }]}
          >
            <Input disabled={Boolean(editingLocation)} />
          </Form.Item>
          <Form.Item name="name" label="货位名称">
            <Input />
          </Form.Item>
          <Form.Item name="barcode" label="条码">
            <Input />
          </Form.Item>
          <Form.Item name="volume" label="容积">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="max_weight" label="承重">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="is_picking" valuePropName="checked">
            <Checkbox>拣货位</Checkbox>
          </Form.Item>
          <Form.Item name="is_disabled" valuePropName="checked">
            <Checkbox>禁用</Checkbox>
          </Form.Item>
        </Form>
      </Drawer>
      <Modal
        title="批量导入货位"
        open={batchOpen}
        onCancel={() => setBatchOpen(false)}
        onOk={handleBatchImport}
        okText="导入"
        width={720}
      >
        <Form form={batchForm} layout="vertical">
          <Form.Item
            name="warehouse_id"
            label="仓库ID"
            rules={[{ required: true, message: '请输入仓库ID' }]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="area_id" label="默认库区ID">
            <Input placeholder="可选，所有行默认使用该库区" />
          </Form.Item>
          <Form.Item
            name="raw"
            label="货位列表"
            rules={[{ required: true, message: '请输入货位数据' }]}
          >
            <Input.TextArea
              rows={8}
              placeholder="编码,名称,条码,容积,承重,拣货位(1/0),禁用(1/0),库区ID"
            />
          </Form.Item>
          <Typography.Text type="secondary">
            每行一条，字段以英文逗号分隔，库区ID可留空使用默认值。
          </Typography.Text>
        </Form>
      </Modal>
    </>
  );
};
