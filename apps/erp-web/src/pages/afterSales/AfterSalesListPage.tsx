import {
  Button,
  Card,
  Col,
  DatePicker,
  Form,
  Input,
  InputNumber,
  Modal,
  Progress,
  Row,
  Select,
  Space,
  Tag,
  Tabs,
  Typography,
  Upload,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { UploadFile } from 'antd/es/upload/interface';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';

import type {
  AfterSalesListQuery,
  AfterSalesSummary,
  AfterSalesCreatePayload,
  AfterSalesType,
  AfterSalesStatus,
  AfterSalesImportJob,
  AfterSalesImportJobListQuery
} from '@/api/afterSalesOps';
import {
  listAfterSales,
  createAfterSales,
  approveAfterSales,
  rejectAfterSales,
  createAfterSalesImportJob,
  listAfterSalesImportJobs,
  createAfterSalesExchangeShipment,
  createAfterSalesResendShipment
} from '@/api/afterSalesOps';
import { ServerTable } from '@/components/ServerTable';
import { UiModule03MainTableSection, UiModule04BottomTable } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  status?: AfterSalesStatus;
  after_sales_type?: AfterSalesType;
  order_id?: string;
  created_range?: [Dayjs, Dayjs];
};

type CreateValues = {
  order_id: string;
  type: AfterSalesType;
  reason_code?: string;
  amount_cent?: number;
  items: Array<{ sku_id: string; qty: number; refund_cent: number }>;
};

const typeOptions = [
  { label: '仅退款', value: 'REFUND_ONLY' },
  { label: '退货退款', value: 'RETURN_REFUND' },
  { label: '换货', value: 'EXCHANGE' },
  { label: '补发', value: 'RESEND' }
];

const statusOptions = [
  { label: '待审核', value: 'OPEN' },
  { label: '已同意', value: 'APPROVED' },
  { label: '已收货', value: 'RECEIVED' },
  { label: '已退款', value: 'REFUNDED' },
  { label: '已关闭', value: 'CLOSED' },
  { label: '已驳回', value: 'REJECTED' }
];

const statusColorMap: Record<AfterSalesStatus, string> = {
  OPEN: 'orange',
  APPROVED: 'blue',
  RECEIVED: 'purple',
  REFUNDED: 'green',
  CLOSED: 'default',
  REJECTED: 'red'
};

const typeColorMap: Record<AfterSalesType, string> = {
  REFUND_ONLY: 'gold',
  RETURN_REFUND: 'cyan',
  EXCHANGE: 'blue',
  RESEND: 'purple'
};

const statusLabelMap = new Map(statusOptions.map((option) => [option.value, option.label]));
const typeLabelMap = new Map(typeOptions.map((option) => [option.value, option.label]));

const formatDateTime = (value?: string) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const AfterSalesListPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('aftersales.write');
  const navigate = useNavigate();
  const [form] = Form.useForm<FilterValues>();
  const [createForm] = Form.useForm<CreateValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [jobPagination, setJobPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [creating, setCreating] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [importing, setImporting] = useState(false);
  const [importFiles, setImportFiles] = useState<UploadFile[]>([]);

  const query = useMemo<AfterSalesListQuery>(() => {
    const range = filters.created_range;
    return {
      status: filters.status,
      after_sales_type: filters.after_sales_type,
      order_id: filters.order_id?.trim() || undefined,
      created_from: range?.[0]?.toISOString(),
      created_to: range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const jobQuery = useMemo<AfterSalesImportJobListQuery>(
    () => ({
      offset: ((jobPagination.current ?? 1) - 1) * (jobPagination.pageSize ?? 10),
      limit: jobPagination.pageSize ?? 10
    }),
    [jobPagination.current, jobPagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['after-sales', tenantId, query],
    queryFn: () => listAfterSales(tenantId, query),
    keepPreviousData: true
  });

  const importJobsQuery = useQuery({
    queryKey: ['after-sales-import-jobs', tenantId, jobQuery],
    queryFn: () => listAfterSalesImportJobs(tenantId, jobQuery),
    keepPreviousData: true
  });

  const handleApprove = async (record: AfterSalesSummary) => {
    try {
      await approveAfterSales(tenantId, record.id);
      message.success('售后已审核');
      await refetch();
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  const handleReject = async (record: AfterSalesSummary) => {
    try {
      await rejectAfterSales(tenantId, record.id);
      message.success('售后已驳回');
      await refetch();
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  const handleCreateShipment = async (record: AfterSalesSummary) => {
    try {
      if (record.type === 'EXCHANGE') {
        await createAfterSalesExchangeShipment(tenantId, record.id, {});
        message.success('换货包裹已生成');
      } else if (record.type === 'RESEND') {
        await createAfterSalesResendShipment(tenantId, record.id, {});
        message.success('补发包裹已生成');
      }
      navigate('/oms/shipments');
    } catch (err) {
      message.error((err as Error).message);
    }
  };

  const columns = useMemo<ColumnsType<AfterSalesSummary>>(
    () => [
      {
        title: '售后单号',
        dataIndex: 'id',
        render: (_, record) => (
          <Link to={`/after-sales/${record.id}`} data-testid="after-sales-link">
            {record.id}
          </Link>
        )
      },
      {
        title: '订单号',
        dataIndex: 'order_id',
        render: (value: string) => <Link to={`/orders/${value}`}>{value}</Link>
      },
      {
        title: '类型',
        dataIndex: 'type',
        render: (value: AfterSalesType) => (
          <Tag color={typeColorMap[value] ?? 'default'}>
            {typeLabelMap.get(value) ?? value}
          </Tag>
        )
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: AfterSalesStatus) => (
          <Tag color={statusColorMap[value] ?? 'default'}>
            {statusLabelMap.get(value) ?? value}
          </Tag>
        )
      },
      {
        title: '退款金额',
        dataIndex: 'amount_cent',
        render: (value: number) => `¥${(value / 100).toFixed(2)}`
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => {
          if (!canWrite) {
            return null;
          }
          const canShipment =
            ['APPROVED', 'RECEIVED'].includes(record.status) &&
            ['EXCHANGE', 'RESEND'].includes(record.type);
          return (
            <Space>
              {record.status === 'OPEN' && (
                <>
                  <Button size="small" type="link" onClick={() => handleApprove(record)}>
                    同意
                  </Button>
                  <Button size="small" type="link" danger onClick={() => handleReject(record)}>
                    驳回
                  </Button>
                </>
              )}
              {record.status === 'APPROVED' && (
                <Button
                  size="small"
                  type="link"
                  data-testid="after-sales-receive-action"
                  onClick={() => navigate(`/after-sales/${record.id}/receive`)}
                >
                  收货
                </Button>
              )}
              {canShipment && (
                <Button size="small" type="link" onClick={() => handleCreateShipment(record)}>
                  {record.type === 'EXCHANGE' ? '生成换货包裹' : '生成补发包裹'}
                </Button>
              )}
            </Space>
          );
        }
      }
    ],
    [canWrite, navigate]
  );

  const jobColumns = useMemo<ColumnsType<AfterSalesImportJob>>(
    () => [
      {
        title: '文件',
        dataIndex: 'file_name',
        render: (value, record) => value || record.job_id
      },
      {
        title: '状态',
        dataIndex: 'status'
      },
      {
        title: '进度',
        dataIndex: 'progress',
        render: (value: number) => <Progress percent={value} size="small" />
      },
      {
        title: '成功/失败',
        key: 'rows',
        render: (_, record) => `${record.success_rows}/${record.failed_rows}`
      },
      {
        title: '错误',
        dataIndex: 'last_error',
        render: (value: string) => value || '-'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value) => formatDateTime(value)
      }
    ],
    []
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

  const handleCreate = async () => {
    try {
      const values = await createForm.validateFields();
      setCreating(true);
      const payload: AfterSalesCreatePayload = {
        order_id: values.order_id,
        type: values.type,
        reason_code: values.reason_code,
        amount_cent: values.amount_cent,
        items: values.items
      };
      await createAfterSales(tenantId, payload);
      message.success('售后创建成功');
      setCreateOpen(false);
      createForm.resetFields();
      await refetch();
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    } finally {
      setCreating(false);
    }
  };

  const handleImport = async () => {
    const file = importFiles[0]?.originFileObj;
    if (!file) {
      message.error('请先上传导入文件');
      return;
    }
    setImporting(true);
    try {
      await createAfterSalesImportJob(tenantId, file as File);
      message.success('导入任务已创建');
      setImportOpen(false);
      setImportFiles([]);
      await importJobsQuery.refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '导入失败');
    } finally {
      setImporting(false);
    }
  };

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} data-testid="after-sales-title">
        售后处理
      </Typography.Title>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="选择状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="after_sales_type" label="类型">
                <Select allowClear options={typeOptions} placeholder="选择类型" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="order_id" label="订单号">
                <Input allowClear placeholder="输入订单号" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="created_range" label="创建时间">
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
                {canWrite && (
                  <>
                    <Button type="dashed" onClick={() => setCreateOpen(true)}>
                      新建售后
                    </Button>
                    <Button type="dashed" onClick={() => setImportOpen(true)}>
                      导入售后
                    </Button>
                  </>
                )}
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  const bottomTabs = (
    <Tabs
      size="small"
      type="card"
      items={[
        {
          key: 'import-jobs',
          label: '导入任务',
          children: (
            <UiModule04BottomTable
              syncKey={`${jobPagination.current ?? 1}-${importJobsQuery.data?.total ?? 0}`}
            >
              <ServerTable<AfterSalesImportJob>
                rowKey="job_id"
                columns={jobColumns}
                data={importJobsQuery.data?.items ?? []}
                loading={importJobsQuery.isFetching}
                error={importJobsQuery.error}
                onRetry={() => importJobsQuery.refetch()}
                errorTitle="导入任务加载失败"
                emptyText="暂无导入任务"
                total={importJobsQuery.data?.total ?? 0}
                pagination={jobPagination}
                onPaginationChange={setJobPagination}
                columnSettingsKey="after-sales.import.jobs.columns"
                enablePanels={false}
              />
            </UiModule04BottomTable>
          )
        }
      ]}
    />
  );

  return (
    <>
      <UiModule03MainTableSection<AfterSalesSummary>
        header={header}
        rowKey="id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="售后列表加载失败"
        emptyText="暂无售后记录"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="after-sales.table.columns"
        bottomPanel={bottomTabs}
      />
      <Modal
        title="新建售后"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={handleCreate}
        confirmLoading={creating}
        okText="提交"
      >
        <Form form={createForm} layout="vertical" initialValues={{ items: [{ qty: 1 }] }}>
          <Form.Item
            name="order_id"
            label="订单号"
            rules={[{ required: true, message: '请输入订单号' }]}
          >
            <Input placeholder="订单号" />
          </Form.Item>
          <Form.Item
            name="type"
            label="售后类型"
            rules={[{ required: true, message: '请选择售后类型' }]}
          >
            <Select options={typeOptions} />
          </Form.Item>
          <Form.Item name="reason_code" label="原因">
            <Input placeholder="原因代码/备注" />
          </Form.Item>
          <Form.Item name="amount_cent" label="退款金额(分)">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.List name="items">
            {(fields, { add, remove }) => (
              <>
                {fields.map((field) => (
                  <Space
                    key={field.key}
                    align="baseline"
                    style={{ display: 'flex', marginBottom: 8 }}
                  >
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
                      <InputNumber min={1} placeholder="数量" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'refund_cent']}
                      rules={[{ required: true, message: '退款金额必填' }]}
                    >
                      <InputNumber min={0} placeholder="退款金额(分)" />
                    </Form.Item>
                    <Button type="link" danger onClick={() => remove(field.name)}>
                      删除
                    </Button>
                  </Space>
                ))}
                <Button type="dashed" onClick={() => add({ qty: 1 })} block>
                  添加明细
                </Button>
              </>
            )}
          </Form.List>
        </Form>
      </Modal>
      <Modal
        title="导入售后"
        open={importOpen}
        onCancel={() => setImportOpen(false)}
        onOk={handleImport}
        confirmLoading={importing}
        okText="开始导入"
      >
        <Space direction="vertical" style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            支持 CSV/Excel，字段示例：order_id、type、sku_id、qty、refund_cent、amount_cent。
          </Typography.Text>
          <Upload
            beforeUpload={() => false}
            fileList={importFiles}
            onChange={({ fileList: nextList }) => setImportFiles(nextList.slice(-1))}
          >
            <Button>选择文件</Button>
          </Upload>
        </Space>
      </Modal>
    </>
  );
};
