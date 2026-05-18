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
  ProcurementInspection,
  ProcurementInspectionCreatePayload,
  ProcurementInspectionDetail,
  ProcurementInspectionItemPayload,
  ProcurementInspectionListQuery,
  ProcurementInspectionStatus,
  ProcurementInspectionUpdatePayload
} from '@/api/procurementInspections';
import {
  createProcurementInspection,
  deleteProcurementInspection,
  getProcurementInspectionDetail,
  listProcurementInspections,
  updateProcurementInspection
} from '@/api/procurementInspections';
import { listSuppliers } from '@/api/suppliers';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  supplier_id?: string;
  warehouse_id?: string;
  status?: ProcurementInspectionStatus;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type FormValues = {
  receipt_id?: string;
  supplier_id?: string;
  warehouse_id?: string;
  status?: ProcurementInspectionStatus;
  inspected_at?: dayjs.Dayjs;
  remark?: string;
  items: ProcurementInspectionItemPayload[];
};

const statusOptions: Array<{ label: string; value: ProcurementInspectionStatus }> = [
  { label: 'PENDING', value: 'PENDING' },
  { label: 'PASSED', value: 'PASSED' },
  { label: 'FAILED', value: 'FAILED' }
];

const statusColors: Record<ProcurementInspectionStatus, string> = {
  PENDING: 'gold',
  PASSED: 'green',
  FAILED: 'red'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const ProcurementInspectionsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const canWrite = useCan('procurement_inspections.write');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingInspection, setEditingInspection] = useState<ProcurementInspectionDetail | null>(
    null
  );
  const [saving, setSaving] = useState(false);
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'procurement-inspections'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'procurement-inspections'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
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

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const query = useMemo<ProcurementInspectionListQuery>(
    () => ({
      supplier_id: filters.supplier_id?.trim() || undefined,
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status,
      created_from: filters.created_range?.[0]?.toISOString(),
      created_to: filters.created_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['procurement-inspections', tenantId, query],
    queryFn: () => listProcurementInspections(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (inspectionId: string) => deleteProcurementInspection(tenantId, inspectionId),
    onSuccess: () => {
      message.success('质检单已删除');
      queryClient.invalidateQueries({ queryKey: ['procurement-inspections', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除质检单失败');
    }
  });

  const saveMutation = useMutation({
    mutationFn: async (payload: {
      inspectionId?: string;
      body: ProcurementInspectionCreatePayload | ProcurementInspectionUpdatePayload;
    }) => {
      if (payload.inspectionId) {
        return updateProcurementInspection(tenantId, payload.inspectionId, payload.body);
      }
      return createProcurementInspection(
        tenantId,
        payload.body as ProcurementInspectionCreatePayload
      );
    },
    onSuccess: () => {
      message.success('质检单已保存');
      setDrawerOpen(false);
      setEditingInspection(null);
      drawerForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['procurement-inspections', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存质检单失败');
    },
    onSettled: () => setSaving(false)
  });

  const handleEdit = async (inspectionId: string) => {
    try {
      const detail = await getProcurementInspectionDetail(tenantId, inspectionId);
      setEditingInspection(detail);
      setDrawerOpen(true);
      drawerForm.setFieldsValue({
        receipt_id: detail.inspection.receipt_id ?? undefined,
        supplier_id: detail.inspection.supplier_id ?? undefined,
        warehouse_id: detail.inspection.warehouse_id ?? undefined,
        status: detail.inspection.status as ProcurementInspectionStatus,
        inspected_at: detail.inspection.inspected_at
          ? dayjs(detail.inspection.inspected_at)
          : undefined,
        remark: detail.inspection.remark ?? undefined,
        items: detail.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          passed_qty: item.passed_qty,
          failed_qty: item.failed_qty,
          result: item.result ?? undefined
        }))
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载质检单失败');
    }
  };

  const handleCreate = () => {
    setEditingInspection(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      items: [{ sku_id: '', qty: 1, passed_qty: 0, failed_qty: 0 }]
    });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      setSaving(true);
      const payload: ProcurementInspectionCreatePayload | ProcurementInspectionUpdatePayload = {
        receipt_id: values.receipt_id?.trim() || null,
        supplier_id: values.supplier_id ?? null,
        warehouse_id: values.warehouse_id ?? null,
        status: values.status ?? undefined,
        inspected_at: values.inspected_at ? values.inspected_at.toISOString() : null,
        remark: values.remark?.trim() || null,
        items: values.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          passed_qty: item.passed_qty,
          failed_qty: item.failed_qty,
          result: item.result ?? null
        }))
      };
      saveMutation.mutate({
        inspectionId: editingInspection?.inspection.inspection_id,
        body: payload
      });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    }
  };

  const columns = useMemo<ColumnsType<ProcurementInspection>>(
    () => [
      { title: '质检单号', dataIndex: 'inspection_id' },
      {
        title: '供应商',
        dataIndex: 'supplier_id',
        render: (value: string | null, record) =>
          record.supplier_name ?? supplierMap.get(value ?? '') ?? value ?? '-'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string | null) => warehouseMap.get(value ?? '') ?? value ?? '-'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: ProcurementInspectionStatus) => (
          <Tag color={statusColors[value] ?? 'default'}>{value}</Tag>
        )
      },
      {
        title: '质检时间',
        dataIndex: 'inspected_at',
        render: formatDateTime
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: formatDateTime
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" onClick={() => handleEdit(record.inspection_id)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除该质检单？"
              onConfirm={() => deleteMutation.mutate(record.inspection_id)}
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
    [canWrite, deleteMutation, supplierMap, warehouseMap]
  );

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        质检单
      </Typography.Title>
    </Space>
  );

  const toolbarRight = (
    <Button type="primary" onClick={handleCreate} disabled={!canWrite}>
    新建质检单
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
          <Form.Item name="warehouse_id" label="仓库">
            <Select options={warehouseOptions} allowClear style={{ width: 200 }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear style={{ width: 160 }} />
          </Form.Item>
          <Form.Item name="created_range" label="创建时间">
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
      <UiModule03MainTableSection<ProcurementInspection>
        toolbarLeft={toolbarLeft}
        toolbarRight={toolbarRight}
        header={header}
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="质检单加载失败"
        emptyText="暂无质检单"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        dataTestId="procurement-inspections-main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        columnSettingsKey="procurement-inspections.table.columns"
        rowKey="inspection_id"
      />
      <Drawer
        title={editingInspection ? '编辑质检单' : '新建质检单'}
        open={drawerOpen}
        width={760}
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
        <Form
          form={drawerForm}
          layout="vertical"
          initialValues={{ items: [{ sku_id: '', qty: 1, passed_qty: 0, failed_qty: 0 }] }}
        >
          <Form.Item name="receipt_id" label="收货单号">
            <Input placeholder="收货单ID" />
          </Form.Item>
          <Form.Item name="supplier_id" label="供应商">
            <Select options={supplierOptions} allowClear />
          </Form.Item>
          <Form.Item name="warehouse_id" label="仓库">
            <Select options={warehouseOptions} allowClear />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear />
          </Form.Item>
          <Form.Item name="inspected_at" label="质检时间">
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={3} placeholder="备注信息" />
          </Form.Item>
          <Form.List name="items">
            {(fields, { add, remove }) => (
              <Space direction="vertical" style={{ width: '100%' }}>
                {fields.map((field, index) => (
                  <Space key={field.key} align="baseline" style={{ display: 'flex' }}>
                    <Form.Item
                      {...field}
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true, message: 'SKU必填' }]}
                    >
                      <Input placeholder="SKU ID" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'qty']}
                      rules={[{ required: true, message: '数量必填' }]}
                    >
                      <InputNumber min={1} />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'passed_qty']}
                      rules={[{ required: true, message: '合格数必填' }]}
                    >
                      <InputNumber min={0} />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'failed_qty']}
                      rules={[{ required: true, message: '不合格数必填' }]}
                    >
                      <InputNumber min={0} />
                    </Form.Item>
                    <Form.Item {...field} name={[field.name, 'result']}>
                      <Input placeholder="结果说明" />
                    </Form.Item>
                    {fields.length > 1 ? (
                      <Button type="link" onClick={() => remove(field.name)}>
                        删除
                      </Button>
                    ) : null}
                    {index === fields.length - 1 ? (
                      <Button
                        type="link"
                        onClick={() =>
                          add({ sku_id: '', qty: 1, passed_qty: 0, failed_qty: 0 })
                        }
                      >
                        添加
                      </Button>
                    ) : null}
                  </Space>
                ))}
              </Space>
            )}
          </Form.List>
        </Form>
      </Drawer>
    </>
  );
};
