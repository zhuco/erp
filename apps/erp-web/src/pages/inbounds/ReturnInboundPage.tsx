import {
  Button,
  Card,
  DatePicker,
  Form,
  Input,
  Modal,
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
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type { AfterSalesStatus, AfterSalesSummary, AfterSalesType } from '@/api/afterSalesOps';
import { getAfterSalesDetail, listAfterSales, receiveAfterSales } from '@/api/afterSalesOps';
import { listInbounds } from '@/api/inbounds';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  status?: AfterSalesStatus;
  after_sales_type?: AfterSalesType;
  order_id?: string;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type ReceiveFormValues = {
  warehouse_id: string;
};

const defaultFilters: FilterValues = { after_sales_type: 'RETURN_REFUND' };

const typeOptions: Array<{ label: string; value: AfterSalesType }> = [
  { label: '退货退款', value: 'RETURN_REFUND' },
  { label: '换货', value: 'EXCHANGE' },
  { label: '补发', value: 'RESEND' }
];

const statusOptions: Array<{ label: string; value: AfterSalesStatus }> = [
  { label: '待处理', value: 'OPEN' },
  { label: '已审核', value: 'APPROVED' },
  { label: '已收货', value: 'RECEIVED' },
  { label: '已退款', value: 'REFUNDED' },
  { label: '已关闭', value: 'CLOSED' },
  { label: '已驳回', value: 'REJECTED' }
];

const statusColors: Record<AfterSalesStatus, string> = {
  OPEN: 'default',
  APPROVED: 'blue',
  RECEIVED: 'green',
  REFUNDED: 'purple',
  CLOSED: 'default',
  REJECTED: 'red'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const ReturnInboundPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const canAfterSalesWrite = useCan('aftersales.write');
  const canInboundWrite = useCan('wms.inbounds.write');
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>(defaultFilters);
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [receiveOpen, setReceiveOpen] = useState(false);
  const [activeAfterSales, setActiveAfterSales] = useState<AfterSalesSummary | null>(null);
  const [receiveForm] = Form.useForm<ReceiveFormValues>();

  const query = useMemo(() => {
    const range = filters.created_range;
    return {
      status: filters.status,
      after_sales_type: filters.after_sales_type,
      order_id: filters.order_id?.trim() || undefined,
      created_from: range?.[0]?.startOf('day').toISOString(),
      created_to: range?.[1]?.endOf('day').toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const listQuery = useQuery({
    queryKey: ['return-inbounds', tenantId, query],
    queryFn: () => listAfterSales(tenantId, query),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'return-inbounds'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters(defaultFilters);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const resolveReturnInboundDocId = async (afterSalesId: string) => {
    const docNo = `RETURN-${afterSalesId}`;
    const response = await listInbounds(tenantId, {
      doc_no: docNo,
      doc_type: 'RETURN',
      limit: 1,
      offset: 0
    });
    return response.items[0]?.doc_id;
  };

  const handleOpenReceive = (record: AfterSalesSummary) => {
    setActiveAfterSales(record);
    receiveForm.resetFields();
    setReceiveOpen(true);
  };

  const handleConfirmReceive = async () => {
    if (!activeAfterSales) {
      return;
    }
    if (!canAfterSalesWrite) {
      message.error('暂无售后收货权限');
      return;
    }
    const values = await receiveForm.validateFields();
    try {
      await receiveAfterSales(tenantId, activeAfterSales.id, {
        warehouse_id: values.warehouse_id,
        putaway_done: false
      });
      message.success('退货收货已登记');
      setReceiveOpen(false);
      const docId = await resolveReturnInboundDocId(activeAfterSales.id);
      if (docId) {
        navigate(`/wms/inbounds/${docId}/receive`);
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '退货收货失败');
    }
  };

  const handlePutaway = async (record: AfterSalesSummary) => {
    if (!canInboundWrite) {
      message.error('暂无上架权限');
      return;
    }
    try {
      const detail = await getAfterSalesDetail(tenantId, record.id);
      if (!detail.return_receipt) {
        message.warning('请先登记退货收货');
        return;
      }
      const docId = await resolveReturnInboundDocId(record.id);
      if (!docId) {
        message.warning('未找到退货入库单');
        return;
      }
      navigate(`/wms/inbounds/${docId}/putaway`);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '获取退货信息失败');
    }
  };

  const columns = useMemo<ColumnsType<AfterSalesSummary>>(
    () => [
      { title: '售后单号', dataIndex: 'id' },
      { title: '订单号', dataIndex: 'order_id' },
      {
        title: '类型',
        dataIndex: 'type',
        render: (value: AfterSalesType) => typeOptions.find((item) => item.value === value)?.label
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: AfterSalesStatus) => (
          <Tag color={statusColors[value] || 'default'}>{value}</Tag>
        )
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canAfterSalesWrite} onClick={() => handleOpenReceive(record)}>
              收货登记
            </Button>
            <Button size="small" disabled={!canInboundWrite} onClick={() => handlePutaway(record)}>
              上架
            </Button>
            <Button size="small" onClick={() => navigate(`/after-sales/${record.id}`)}>
              售后详情
            </Button>
          </Space>
        )
      }
    ],
    [canAfterSalesWrite, canInboundWrite, navigate]
  );

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      退货入库
    </Typography.Title>
  );

  const header = (
    <Space direction="vertical" size={12}>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch} initialValues={defaultFilters}>
          <Space wrap>
            <Form.Item name="after_sales_type" label="售后类型">
              <Select allowClear options={typeOptions} placeholder="选择类型" style={{ width: 200 }} />
            </Form.Item>
            <Form.Item name="status" label="状态">
              <Select allowClear options={statusOptions} placeholder="选择状态" style={{ width: 200 }} />
            </Form.Item>
            <Form.Item name="order_id" label="订单号">
              <Input allowClear placeholder="输入订单号" />
            </Form.Item>
            <Form.Item name="created_range" label="创建时间">
              <DatePicker.RangePicker allowClear />
            </Form.Item>
            <Form.Item label=" ">
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Form.Item>
          </Space>
        </Form>
      </Card>
      {listQuery.error ? (
        <ErrorState
          title="退货列表加载失败"
          error={listQuery.error}
          onRetry={() => listQuery.refetch()}
        />
      ) : null}
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<AfterSalesSummary>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey={(record) => record.id}
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="退货入库加载失败"
        emptyText="暂无退货记录"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="return-inbounds.table.columns"
      />
      <Modal
        title="退货收货登记"
        open={receiveOpen}
        onCancel={() => setReceiveOpen(false)}
        onOk={handleConfirmReceive}
        okText="确认"
        okButtonProps={{ disabled: !canAfterSalesWrite }}
      >
        <Form form={receiveForm} layout="vertical">
          <Form.Item
            name="warehouse_id"
            label="收货仓库"
            rules={[{ required: true, message: '请选择仓库' }]}
          >
            <Select options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
