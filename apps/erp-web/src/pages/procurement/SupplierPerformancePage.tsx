import {
  Button,
  DatePicker,
  Drawer,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  SupplierPerformance,
  SupplierPerformanceCreatePayload,
  SupplierPerformanceListQuery,
  SupplierPerformanceStatus,
  SupplierPerformanceUpdatePayload
} from '@/api/supplierPerformances';
import {
  createSupplierPerformance,
  deleteSupplierPerformance,
  listSupplierPerformances,
  updateSupplierPerformance
} from '@/api/supplierPerformances';
import { listSuppliers } from '@/api/suppliers';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  supplier_id?: string;
  status?: SupplierPerformanceStatus;
  period_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type FormValues = {
  supplier_id?: string;
  status?: SupplierPerformanceStatus;
  period_range?: [dayjs.Dayjs, dayjs.Dayjs];
  delivery_score: number;
  quality_score: number;
  price_score: number;
  remark?: string;
};

const statusOptions: Array<{ label: string; value: SupplierPerformanceStatus }> = [
  { label: 'ACTIVE', value: 'ACTIVE' }
];

const statusColors: Record<SupplierPerformanceStatus, string> = {
  ACTIVE: 'green'
};

export const SupplierPerformancePage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const canWrite = useCan('supplier_performances.write');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingRecord, setEditingRecord] = useState<SupplierPerformance | null>(null);
  const [saving, setSaving] = useState(false);
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'supplier-performances'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const supplierOptions = useMemo(
    () =>
      supplierQuery.data?.items.map((supplier) => ({
        label: supplier.name,
        value: supplier.supplier_id
      })) ?? [],
    [supplierQuery.data?.items]
  );

  const supplierMap = useMemo(() => {
    const entries = supplierQuery.data?.items ?? [];
    return new Map(entries.map((supplier) => [supplier.supplier_id, supplier.name]));
  }, [supplierQuery.data?.items]);

  const query = useMemo<SupplierPerformanceListQuery>(
    () => ({
      supplier_id: filters.supplier_id?.trim() || undefined,
      status: filters.status,
      period_from: filters.period_range?.[0]?.format('YYYY-MM-DD'),
      period_to: filters.period_range?.[1]?.format('YYYY-MM-DD'),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['supplier-performances', tenantId, query],
    queryFn: () => listSupplierPerformances(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (performanceId: string) => deleteSupplierPerformance(tenantId, performanceId),
    onSuccess: () => {
      message.success('绩效记录已删除');
      queryClient.invalidateQueries({ queryKey: ['supplier-performances', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除绩效记录失败');
    }
  });

  const saveMutation = useMutation({
    mutationFn: async (payload: {
      performanceId?: string;
      body: SupplierPerformanceCreatePayload | SupplierPerformanceUpdatePayload;
    }) => {
      if (payload.performanceId) {
        return updateSupplierPerformance(tenantId, payload.performanceId, payload.body);
      }
      return createSupplierPerformance(tenantId, payload.body as SupplierPerformanceCreatePayload);
    },
    onSuccess: () => {
      message.success('绩效记录已保存');
      setDrawerOpen(false);
      setEditingRecord(null);
      drawerForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['supplier-performances', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存绩效记录失败');
    },
    onSettled: () => setSaving(false)
  });

  const handleEdit = (record: SupplierPerformance) => {
    setEditingRecord(record);
    setDrawerOpen(true);
    drawerForm.setFieldsValue({
      supplier_id: record.supplier_id,
      status: record.status as SupplierPerformanceStatus,
      period_range: [dayjs(record.period_start), dayjs(record.period_end)],
      delivery_score: record.delivery_score,
      quality_score: record.quality_score,
      price_score: record.price_score,
      remark: record.remark ?? undefined
    });
  };

  const handleCreate = () => {
    setEditingRecord(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      setSaving(true);
      const payload: SupplierPerformanceCreatePayload | SupplierPerformanceUpdatePayload = {
        supplier_id: values.supplier_id ?? '',
        period_start: values.period_range?.[0]?.format('YYYY-MM-DD') ?? '',
        period_end: values.period_range?.[1]?.format('YYYY-MM-DD') ?? '',
        delivery_score: values.delivery_score,
        quality_score: values.quality_score,
        price_score: values.price_score,
        status: values.status ?? undefined,
        remark: values.remark?.trim() || null
      };
      saveMutation.mutate({
        performanceId: editingRecord?.performance_id,
        body: payload
      });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    }
  };

  const columns = useMemo<ColumnsType<SupplierPerformance>>(
    () => [
      { title: '绩效编号', dataIndex: 'performance_id' },
      {
        title: '供应商',
        dataIndex: 'supplier_id',
        render: (value: string, record) =>
          record.supplier_name ?? supplierMap.get(value) ?? value ?? '-'
      },
      { title: '期间开始', dataIndex: 'period_start' },
      { title: '期间结束', dataIndex: 'period_end' },
      { title: '交付评分', dataIndex: 'delivery_score' },
      { title: '质量评分', dataIndex: 'quality_score' },
      { title: '价格评分', dataIndex: 'price_score' },
      { title: '综合评分', dataIndex: 'total_score' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: SupplierPerformanceStatus) => (
          <Tag color={statusColors[value] ?? 'default'}>{value}</Tag>
        )
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" onClick={() => handleEdit(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除该绩效记录？"
              onConfirm={() => deleteMutation.mutate(record.performance_id)}
              okButtonProps={{ disabled: !canWrite }}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation, supplierMap]
  );

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        供应商绩效
      </Typography.Title>
    </Space>
  );

  const toolbarRight = (
    <Button type="primary" onClick={handleCreate} disabled={!canWrite}>
    新建绩效
    </Button>
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
        <Form
          form={filterForm}
          layout="inline"
          onFinish={(values) => setFilters(values)}
          initialValues={filters}
        >
          <Form.Item name="supplier_id" label="供应商">
            <Select options={supplierOptions} allowClear style={{ width: 200 }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear style={{ width: 160 }} />
          </Form.Item>
          <Form.Item name="period_range" label="考核期间">
            <DatePicker.RangePicker />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                查询
              </Button>
              <Button
                onClick={() => {
                  filterForm.resetFields();
                  setFilters({});
                }}
              >
                重置
              </Button>
            </Space>
          </Form.Item>
        </Form>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<ProcurementSupplierPerformance>
        toolbarLeft={toolbarLeft}
        toolbarRight={toolbarRight}
        header={header}
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="供应商绩效加载失败"
        emptyText="暂无绩效记录"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        dataTestId="supplier-performance-main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        columnSettingsKey="supplier-performance.table.columns"
        rowKey="performance_id"
      />
      <Drawer
        title={editingRecord ? '编辑绩效' : '新建绩效'}
        open={drawerOpen}
        width={620}
        onClose={() => setDrawerOpen(false)}
        extra={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave} disabled={!canWrite}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="supplier_id"
            label="供应商"
            rules={[{ required: true, message: '请选择供应商' }]}
          >
            <Select options={supplierOptions} allowClear />
          </Form.Item>
          <Form.Item
            name="period_range"
            label="考核期间"
            rules={[{ required: true, message: '请选择考核期间' }]}
          >
            <DatePicker.RangePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="delivery_score"
            label="交付评分"
            rules={[{ required: true, message: '请输入交付评分' }]}
          >
            <InputNumber min={0} max={100} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="quality_score"
            label="质量评分"
            rules={[{ required: true, message: '请输入质量评分' }]}
          >
            <InputNumber min={0} max={100} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="price_score"
            label="价格评分"
            rules={[{ required: true, message: '请输入价格评分' }]}
          >
            <InputNumber min={0} max={100} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear />
          </Form.Item>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={3} placeholder="备注信息" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
