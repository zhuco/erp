import {
  Button,
  Card,
  Col,
  DatePicker,
  Form,
  Input,
  InputNumber,
  Modal,
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
import type { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type {
  AfterSalesInspection,
  AfterSalesInspectionCreatePayload,
  AfterSalesInspectionListQuery,
  AfterSalesInspectionResult,
  AfterSalesInspectionUpdatePayload
} from '@/api/afterSalesWorkflow';
import {
  createAfterSalesInspection,
  listAfterSalesInspections,
  updateAfterSalesInspection
} from '@/api/afterSalesWorkflow';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  after_sales_id?: string;
  sku_id?: string;
  warehouse_id?: string;
  result_code?: AfterSalesInspectionResult;
  inspected_range?: [Dayjs, Dayjs];
};

type InspectionValues = {
  after_sales_id: string;
  sku_id: string;
  warehouse_id?: string;
  qty: number;
  result_code?: AfterSalesInspectionResult;
  inspector_name?: string;
  inspected_at?: Dayjs;
};

const resultOptions = [
  { label: '待质检', value: 'PENDING' },
  { label: '可二次销售', value: 'RESALE' },
  { label: '返修', value: 'REPAIR' },
  { label: '报废', value: 'SCRAP' }
];

const resultColors: Record<AfterSalesInspectionResult, string> = {
  PENDING: 'default',
  RESALE: 'green',
  REPAIR: 'blue',
  SCRAP: 'red'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const AfterSalesInspectionsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('aftersales.write');
  const [form] = Form.useForm<FilterValues>();
  const [inspectionForm] = Form.useForm<InspectionValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [modalOpen, setModalOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [editingInspection, setEditingInspection] = useState<AfterSalesInspection | null>(null);

  const query = useMemo<AfterSalesInspectionListQuery>(() => {
    const range = filters.inspected_range;
    return {
      after_sales_id: filters.after_sales_id?.trim() || undefined,
      sku_id: filters.sku_id?.trim() || undefined,
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      result_code: filters.result_code,
      inspected_from: range?.[0]?.toISOString(),
      inspected_to: range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isFetching, error, refetch } = useQuery({
    queryKey: ['after-sales-inspections', tenantId, query],
    queryFn: () => listAfterSalesInspections(tenantId, query),
    keepPreviousData: true
  });

  const { data: warehouses } = useQuery({
    queryKey: ['warehouses', tenantId],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    staleTime: 60_000
  });

  const warehouseOptions = useMemo(
    () =>
      (warehouses?.items ?? []).map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })),
    [warehouses?.items]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openModal = (record?: AfterSalesInspection) => {
    if (record) {
      setEditingInspection(record);
      inspectionForm.setFieldsValue({
        after_sales_id: record.after_sales_id,
        sku_id: record.sku_id,
        warehouse_id: record.warehouse_id ?? undefined,
        qty: record.qty,
        result_code: record.result_code,
        inspector_name: record.inspector_name ?? undefined,
        inspected_at: record.inspected_at ? dayjs(record.inspected_at) : undefined
      });
    } else {
      setEditingInspection(null);
      inspectionForm.resetFields();
      inspectionForm.setFieldsValue({
        qty: 1,
        result_code: 'PENDING'
      });
    }
    setModalOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await inspectionForm.validateFields();
      setSaving(true);
      if (editingInspection) {
        const payload: AfterSalesInspectionUpdatePayload = {
          warehouse_id: values.warehouse_id ?? null,
          qty: values.qty,
          result_code: values.result_code,
          inspector_name: values.inspector_name?.trim() || null,
          inspected_at: values.inspected_at?.toISOString()
        };
        await updateAfterSalesInspection(tenantId, editingInspection.inspection_id, payload);
        message.success('质检记录已更新');
      } else {
        const payload: AfterSalesInspectionCreatePayload = {
          after_sales_id: values.after_sales_id.trim(),
          sku_id: values.sku_id.trim(),
          warehouse_id: values.warehouse_id ?? null,
          qty: values.qty,
          result_code: values.result_code,
          inspector_name: values.inspector_name?.trim() || null,
          inspected_at: values.inspected_at?.toISOString()
        };
        await createAfterSalesInspection(tenantId, payload);
        message.success('质检记录已登记');
      }
      setModalOpen(false);
      setEditingInspection(null);
      inspectionForm.resetFields();
      await refetch();
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const columns = useMemo<ColumnsType<AfterSalesInspection>>(
    () => [
      { title: '质检单号', dataIndex: 'inspection_id' },
      { title: '售后单号', dataIndex: 'after_sales_id' },
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: '数量', dataIndex: 'qty' },
      {
        title: '质检结果',
        dataIndex: 'result_code',
        render: (value: AfterSalesInspectionResult) => (
          <Tag color={resultColors[value]}>
            {resultOptions.find((item) => item.value === value)?.label ?? value}
          </Tag>
        )
      },
      { title: '仓库', dataIndex: 'warehouse_id', render: (value) => value || '-' },
      { title: '质检员', dataIndex: 'inspector_name', render: (value) => value || '-' },
      { title: '质检时间', dataIndex: 'inspected_at', render: (value) => formatDateTime(value) },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) =>
          canWrite ? (
            <Button size="small" onClick={() => openModal(record)}>
              登记质检
            </Button>
          ) : null
      }
    ],
    [canWrite]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} data-testid="after-sales-inspections-title">
        退货质检
      </Typography.Title>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item label="售后单号" name="after_sales_id">
                <Input allowClear placeholder="输入售后单号" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item label="SKU" name="sku_id">
                <Input allowClear placeholder="输入SKU" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item label="仓库" name="warehouse_id">
                <Select allowClear options={warehouseOptions} placeholder="选择仓库" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item label="质检结果" name="result_code">
                <Select allowClear options={resultOptions} placeholder="选择结果" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item label="质检时间" name="inspected_range">
                <DatePicker.RangePicker
                  allowClear
                  showTime
                  style={{ width: '100%' }}
                  placeholder={['开始时间', '结束时间']}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
                {canWrite ? (
                  <Button
                    type="dashed"
                    data-testid="after-sales-inspections-create"
                    onClick={() => openModal()}
                  >
                    登记质检
                  </Button>
                ) : null}
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <div data-testid="after-sales-inspections-table">
        <UiModule03MainTableSection<AfterSalesInspection>
          header={header}
          rowKey="inspection_id"
          columns={columns}
          data={data?.items ?? []}
          loading={isFetching}
          error={error}
          onRetry={() => refetch()}
          errorTitle="质检列表加载失败"
          emptyText="暂无质检记录"
          total={data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="after-sales.inspections.columns"
        />
      </div>
      <Modal
        title={editingInspection ? '编辑质检记录' : '登记质检'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSave}
        okText="提交"
        okButtonProps={{ 'data-testid': 'after-sales-inspections-submit' }}
        confirmLoading={saving}
      >
        <Form form={inspectionForm} layout="vertical">
          <Form.Item
            label="售后单号"
            name="after_sales_id"
            rules={[{ required: true, message: '请填写售后单号' }]}
          >
            <Input
              data-testid="after-sales-inspection-after-sales"
              disabled={Boolean(editingInspection)}
              placeholder="售后单号"
            />
          </Form.Item>
          <Form.Item
            label="SKU"
            name="sku_id"
            rules={[{ required: true, message: '请填写SKU' }]}
          >
            <Input
              data-testid="after-sales-inspection-sku"
              disabled={Boolean(editingInspection)}
              placeholder="SKU"
            />
          </Form.Item>
          <Form.Item label="仓库" name="warehouse_id">
            <Select allowClear options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
          <Form.Item
            label="数量"
            name="qty"
            rules={[{ required: true, message: '请填写数量' }]}
          >
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item label="质检结果" name="result_code">
            <Select options={resultOptions} />
          </Form.Item>
          <Form.Item label="质检员" name="inspector_name">
            <Input placeholder="质检员姓名" />
          </Form.Item>
          <Form.Item label="质检时间" name="inspected_at">
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
