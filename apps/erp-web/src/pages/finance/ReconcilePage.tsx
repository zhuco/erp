import {
  Button,
  DatePicker,
  Form,
  Input,
  Select,
  Tag,
  Typography,
  Upload,
  message
} from 'antd';
import type { UploadFile } from 'antd/es/upload/interface';
import type { ColumnsType } from 'antd/es/table';
import type { Dayjs } from 'dayjs';
import { useDeferredValue, useEffect, useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { createExportJob } from '@/api/exports';
import {
  importStatementCsv,
  listPaymentReconcileItems,
  listPaymentReconcileJobs,
  listReconcileItems,
  listReconcileJobs,
  runPaymentReconcile,
  runReconcile,
  type PaymentReconcileItem,
  type PaymentReconcileJob,
  type ReconcileJob,
  type ReconcileJobItem,
  type StatementImportType
} from '@/api/finance';
import { SettingsPageTemplate } from '@/components/page-templates';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { platformOptions } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { shopOptions } from '@/features/tenantOptions';

const { Text } = Typography;

const dateTimeFormatter = new Intl.DateTimeFormat('zh-CN', {
  dateStyle: 'medium',
  timeStyle: 'short',
  hour12: false
});

const amountFormatter = new Intl.NumberFormat('zh-CN', {
  style: 'currency',
  currency: 'CNY',
  minimumFractionDigits: 2,
  maximumFractionDigits: 2
});

type ImportFormValues = {
  statement_type: StatementImportType;
  platform?: string;
  shop_id?: string;
  period?: [Dayjs, Dayjs];
};

type ReconcileJobType = 'PLATFORM' | 'SHIPPING';

type ReconcileFormValues = {
  job_type: ReconcileJobType;
  statement_id?: string;
  occurred_range?: [Dayjs, Dayjs];
};

type PaymentImportFormValues = {
  channel: string;
  period?: [Dayjs, Dayjs];
};

type PaymentReconcileFormValues = {
  statement_id?: string;
};

const buildJobStatusTag = (status: string) => {
  if (status === 'DONE') {
    return <Tag color="green">DONE</Tag>;
  }
  if (status === 'FAILED') {
    return <Tag color="red">FAILED</Tag>;
  }
  return <Tag color="blue">{status}</Tag>;
};

const buildDiffTag = (diff: number) => {
  if (diff === 0) {
    return <Tag color="green">0</Tag>;
  }
  return <Tag color="orange">{diff}</Tag>;
};

const buildDiffAmountTag = (diffCent: number) => (
  <Tag color={diffCent === 0 ? 'green' : 'orange'}>{amountFormatter.format(diffCent / 100)}</Tag>
);

const formatDateTime = (value?: string | null) =>
  value ? dateTimeFormatter.format(new Date(value)) : '-';

export const ReconcilePage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('finance.write');
  const canExport = useCan('exports.write');
  const queryClient = useQueryClient();
  const [importForm] = Form.useForm<ImportFormValues>();
  const [reconcileForm] = Form.useForm<ReconcileFormValues>();
  const [paymentImportForm] = Form.useForm<PaymentImportFormValues>();
  const [paymentReconcileForm] = Form.useForm<PaymentReconcileFormValues>();
  const [fileList, setFileList] = useState<UploadFile[]>([]);
  const [statementId, setStatementId] = useState<string | null>(null);
  const [paymentFileList, setPaymentFileList] = useState<UploadFile[]>([]);
  const [paymentStatementId, setPaymentStatementId] = useState<string | null>(null);
  const [paymentStatementFilterInput, setPaymentStatementFilterInput] = useState('');
  const [jobPagination, setJobPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [itemPagination, setItemPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [paymentJobPagination, setPaymentJobPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [paymentItemPagination, setPaymentItemPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [selectedPaymentJobId, setSelectedPaymentJobId] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [running, setRunning] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [paymentImporting, setPaymentImporting] = useState(false);
  const [paymentRunning, setPaymentRunning] = useState(false);
  const deferredPaymentStatementFilter = useDeferredValue(paymentStatementFilterInput);

  useEffect(() => {
    if (statementId) {
      reconcileForm.setFieldsValue({ statement_id: statementId });
    }
  }, [statementId, reconcileForm]);

  useEffect(() => {
    if (paymentStatementId) {
      paymentReconcileForm.setFieldsValue({ statement_id: paymentStatementId });
    }
  }, [paymentStatementId, paymentReconcileForm]);

  useEffect(() => {
    setPaymentJobPagination((prev) => ({
      current: 1,
      pageSize: prev.pageSize ?? 10
    }));
  }, [deferredPaymentStatementFilter]);

  const jobQuery = useMemo(() => {
    const offset = ((jobPagination.current ?? 1) - 1) * (jobPagination.pageSize ?? 10);
    return {
      offset,
      limit: jobPagination.pageSize ?? 10
    };
  }, [jobPagination.current, jobPagination.pageSize]);

  const jobList = useQuery({
    queryKey: ['reconcileJobs', tenantId, jobQuery],
    queryFn: () => listReconcileJobs(tenantId, jobQuery),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData
  });

  const itemQuery = useMemo(() => {
    const offset = ((itemPagination.current ?? 1) - 1) * (itemPagination.pageSize ?? 10);
    return {
      job_id: selectedJobId ?? '',
      offset,
      limit: itemPagination.pageSize ?? 10,
      diff_only: true
    };
  }, [itemPagination.current, itemPagination.pageSize, selectedJobId]);

  const itemList = useQuery({
    queryKey: ['reconcileItems', tenantId, itemQuery],
    queryFn: () => listReconcileItems(tenantId, itemQuery),
    enabled: Boolean(tenantId && selectedJobId),
    placeholderData: (previousData) => previousData
  });

  const paymentJobQuery = useMemo(() => {
    const offset =
      ((paymentJobPagination.current ?? 1) - 1) * (paymentJobPagination.pageSize ?? 10);
    const statementId = deferredPaymentStatementFilter.trim();
    return {
      statement_id: statementId || undefined,
      offset,
      limit: paymentJobPagination.pageSize ?? 10
    };
  }, [
    deferredPaymentStatementFilter,
    paymentJobPagination.current,
    paymentJobPagination.pageSize
  ]);

  const paymentJobList = useQuery({
    queryKey: ['paymentReconcileJobs', tenantId, paymentJobQuery],
    queryFn: () => listPaymentReconcileJobs(tenantId, paymentJobQuery),
    enabled: Boolean(tenantId),
    placeholderData: (previousData) => previousData
  });

  const paymentItemQuery = useMemo(() => {
    const offset =
      ((paymentItemPagination.current ?? 1) - 1) * (paymentItemPagination.pageSize ?? 10);
    return {
      job_id: selectedPaymentJobId ?? '',
      offset,
      limit: paymentItemPagination.pageSize ?? 10,
      diff_only: true
    };
  }, [paymentItemPagination.current, paymentItemPagination.pageSize, selectedPaymentJobId]);

  const paymentItemList = useQuery({
    queryKey: ['paymentReconcileItems', tenantId, paymentItemQuery],
    queryFn: () => listPaymentReconcileItems(tenantId, paymentItemQuery),
    enabled: Boolean(tenantId && selectedPaymentJobId),
    placeholderData: (previousData) => previousData
  });

  const handleImport = async (values: ImportFormValues) => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    const file = fileList[0]?.originFileObj;
    if (!file) {
      message.error('请上传CSV账单');
      return;
    }
    const period = values.period;
    const payload = {
      statement_type: values.statement_type,
      platform: values.platform,
      shop_id: values.shop_id,
      period_start: period?.[0]?.format('YYYY-MM-DD'),
      period_end: period?.[1]?.format('YYYY-MM-DD')
    };
    setImporting(true);
    try {
      const text = await file.text();
      const result = await importStatementCsv(tenantId, payload, text);
      setStatementId(result.statement_id ?? null);
      message.success(`导入成功，已写入${result.inserted_lines}行`);
      await queryClient.invalidateQueries({ queryKey: ['reconcileJobs', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导入失败');
    } finally {
      setImporting(false);
    }
  };

  const handleRun = async (values: ReconcileFormValues) => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    const range = values.occurred_range;
    const payload = {
      job_type: values.job_type,
      statement_id: values.statement_id,
      occurred_from: range?.[0]?.toISOString(),
      occurred_to: range?.[1]?.toISOString()
    };
    setRunning(true);
    try {
      const result = await runReconcile(tenantId, payload);
      message.success(`对账完成，差异${result.diff_count}条`);
      setSelectedJobId(result.job_id);
      await queryClient.invalidateQueries({ queryKey: ['reconcileJobs', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['reconcileItems', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '对账失败');
    } finally {
      setRunning(false);
    }
  };

  const handlePaymentImport = async (values: PaymentImportFormValues) => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    const file = paymentFileList[0]?.originFileObj;
    if (!file) {
      message.error('请上传支付明细CSV');
      return;
    }
    const channel = values.channel?.trim();
    if (!channel) {
      message.error('请输入支付渠道');
      return;
    }
    const period = values.period;
    const payload = {
      statement_type: 'PAYMENT' as const,
      channel,
      period_start: period?.[0]?.format('YYYY-MM-DD'),
      period_end: period?.[1]?.format('YYYY-MM-DD')
    };
    setPaymentImporting(true);
    try {
      const text = await file.text();
      const result = await importStatementCsv(tenantId, payload, text);
      setPaymentStatementId(result.statement_id ?? null);
      message.success(`导入成功，已写入${result.inserted_lines}行`);
      await queryClient.invalidateQueries({ queryKey: ['paymentReconcileJobs', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导入失败');
    } finally {
      setPaymentImporting(false);
    }
  };

  const handlePaymentRun = async (values: PaymentReconcileFormValues) => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    const statementIdValue = values.statement_id?.trim();
    if (!statementIdValue) {
      message.error('请输入支付账单ID');
      return;
    }
    setPaymentRunning(true);
    try {
      const result = await runPaymentReconcile(tenantId, { statement_id: statementIdValue });
      message.success(`支付对账完成，差异${result.diff_count}条`);
      setSelectedPaymentJobId(result.job_id);
      await queryClient.invalidateQueries({ queryKey: ['paymentReconcileJobs', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['paymentReconcileItems', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '支付对账失败');
    } finally {
      setPaymentRunning(false);
    }
  };

  const handleExport = async () => {
    if (!selectedJobId) {
      message.error('请选择对账任务后再导出');
      return;
    }
    setExporting(true);
    try {
      const job = await createExportJob({
        type: 'RECONCILE_EXPORT',
        params: { job_id: selectedJobId, diff_only: true }
      });
      message.success(`导出任务已创建：${job.job_id}，请到导出中心下载`);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '导出任务创建失败');
    } finally {
      setExporting(false);
    }
  };

  const jobColumns = useMemo<ColumnsType<ReconcileJob>>(
    () => [
      { title: '任务ID', dataIndex: 'job_id', ellipsis: true },
      { title: '类型', dataIndex: 'job_type' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => buildJobStatusTag(record.status)
      },
      {
        title: '开始时间',
        dataIndex: 'started_at',
        render: (_, record) => formatDateTime(record.started_at)
      },
      {
        title: '结束时间',
        dataIndex: 'finished_at',
        render: (_, record) => formatDateTime(record.finished_at)
      },
      {
        title: '错误',
        dataIndex: 'last_error',
        render: (_, record) => record.last_error ?? '-'
      }
    ],
    []
  );

  const itemColumns = useMemo<ColumnsType<ReconcileJobItem>>(
    () => [
      { title: '类型', dataIndex: 'ref_type' },
      { title: '引用ID', dataIndex: 'ref_id', ellipsis: true },
      { title: '应收', dataIndex: 'expected_cent', render: (_, record) => amountFormatter.format(record.expected_cent / 100) },
      { title: '实收', dataIndex: 'actual_cent', render: (_, record) => amountFormatter.format(record.actual_cent / 100) },
      {
        title: '差异',
        dataIndex: 'diff_cent',
        render: (_, record) => buildDiffAmountTag(record.diff_cent)
      },
      { title: '原因', dataIndex: 'diff_reason', render: (_, record) => record.diff_reason ?? '-' }
    ],
    []
  );

  const reconcileJobHeader = <Typography.Title level={5}>对账任务</Typography.Title>;
  const reconcileItemHeader = <Typography.Title level={5}>差异明细</Typography.Title>;
  const paymentJobHeader = <Typography.Title level={5}>支付对账任务</Typography.Title>;
  const paymentItemHeader = <Typography.Title level={5}>支付对账差异</Typography.Title>;

  const paymentJobColumns = useMemo<ColumnsType<PaymentReconcileJob>>(
    () => [
      { title: '任务ID', dataIndex: 'job_id', ellipsis: true },
      {
        title: '账单ID',
        dataIndex: 'statement_id',
        render: (_, record) => record.statement_id ?? '-',
        ellipsis: true
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => buildJobStatusTag(record.status)
      },
      {
        title: '差异',
        dataIndex: 'diff_count',
        render: (_, record) => buildDiffTag(record.diff_count)
      },
      {
        title: '开始时间',
        dataIndex: 'started_at',
        render: (_, record) => formatDateTime(record.started_at)
      },
      {
        title: '结束时间',
        dataIndex: 'finished_at',
        render: (_, record) => formatDateTime(record.finished_at)
      },
      {
        title: '错误',
        dataIndex: 'last_error',
        render: (_, record) => record.last_error ?? '-'
      }
    ],
    []
  );

  const paymentItemColumns = useMemo<ColumnsType<PaymentReconcileItem>>(
    () => [
      { title: '平台订单号', dataIndex: 'platform_order_id', ellipsis: true },
      {
        title: '订单ID',
        dataIndex: 'order_id',
        render: (_, record) => record.order_id ?? '-',
        ellipsis: true
      },
      { title: '应收', dataIndex: 'expected_cent', render: (_, record) => amountFormatter.format(record.expected_cent / 100) },
      { title: '实收', dataIndex: 'actual_cent', render: (_, record) => amountFormatter.format(record.actual_cent / 100) },
      {
        title: '差异',
        dataIndex: 'diff_cent',
        render: (_, record) => buildDiffAmountTag(record.diff_cent)
      },
      { title: '原因', dataIndex: 'diff_reason', render: (_, record) => record.diff_reason ?? '-' }
    ],
    []
  );

  return (
    <SettingsPageTemplate title="对账管理">
      <div className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          账单导入
        </Typography.Title>
        <Form
          form={importForm}
          layout="inline"
          initialValues={{ statement_type: 'PLATFORM' }}
          onFinish={handleImport}
        >
          <Form.Item name="statement_type" label="账单类型" rules={[{ required: true }]}>
            <Select
              style={{ width: 140 }}
              options={[
                { label: '平台账单', value: 'PLATFORM' },
                { label: '快递账单', value: 'SHIPPING' }
              ]}
            />
          </Form.Item>
          <Form.Item
            noStyle
            shouldUpdate={(prev, next) => prev.statement_type !== next.statement_type}
          >
            {({ getFieldValue }) =>
              getFieldValue('statement_type') === 'PLATFORM' ? (
                <>
                  <Form.Item name="platform" label="平台" rules={[{ required: true }]}>
                    <Select style={{ width: 140 }} options={platformOptions} />
                  </Form.Item>
                  <Form.Item name="shop_id" label="店铺" rules={[{ required: true }]}>
                    <Select style={{ width: 180 }} options={shopOptions} showSearch />
                  </Form.Item>
                  <Form.Item name="period" label="账期" rules={[{ required: true }]}>
                    <DatePicker.RangePicker />
                  </Form.Item>
                </>
              ) : null
            }
          </Form.Item>
          <Form.Item label="CSV文件" required>
            <Upload
              beforeUpload={() => false}
              fileList={fileList}
              onChange={({ fileList: nextList }) => setFileList(nextList.slice(-1))}
              disabled={!canWrite}
            >
              <Button>选择文件</Button>
            </Upload>
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={importing} disabled={!canWrite}>
              导入
            </Button>
          </Form.Item>
          {statementId ? <Text>最新平台账单ID: {statementId}</Text> : null}
        </Form>
      </div>

      <div className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          运行对账
        </Typography.Title>
        <Form
          form={reconcileForm}
          layout="inline"
          initialValues={{ job_type: 'PLATFORM' }}
          onFinish={handleRun}
        >
          <Form.Item name="job_type" label="对账类型" rules={[{ required: true }]}>
            <Select
              style={{ width: 140 }}
              options={[
                { label: '平台对账', value: 'PLATFORM' },
                { label: '快递对账', value: 'SHIPPING' }
              ]}
            />
          </Form.Item>
          <Form.Item
            noStyle
            shouldUpdate={(prev, next) => prev.job_type !== next.job_type}
          >
            {({ getFieldValue }) =>
              getFieldValue('job_type') === 'PLATFORM' ? (
                <Form.Item name="statement_id" label="账单ID" rules={[{ required: true }]}>
                  <Input
                    style={{ width: 280 }}
                    placeholder="例如 11111111-1111-1111-1111-111111111111…"
                    autoComplete="off"
                  />
                </Form.Item>
              ) : (
                <Form.Item name="occurred_range" label="发生时间" rules={[{ required: true }]}>
                  <DatePicker.RangePicker showTime />
                </Form.Item>
              )
            }
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={running} disabled={!canWrite}>
              运行对账
            </Button>
          </Form.Item>
        </Form>
      </div>

      <UiModule03MainTableSection<ReconcileJob>
        toolbarLeft={reconcileJobHeader}
        rowKey="job_id"
        columns={jobColumns}
        data={jobList.data?.items ?? []}
        loading={jobList.isFetching}
        total={jobList.data?.total ?? 0}
        pagination={jobPagination}
        onPaginationChange={setJobPagination}
        error={jobList.error}
        onRetry={() => jobList.refetch()}
        rowSelection={{
          type: 'radio',
          selectedRowKeys: selectedJobId ? [selectedJobId] : [],
          onChange: (keys) => setSelectedJobId(keys[0] as string)
        }}
        rowClickSelects
        dataTestId="finance-reconcile-main-grid"
        columnSettingsKey="finance.reconcile.main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        emptyText="暂无对账任务"
      />

      <UiModule03MainTableSection<ReconcileJobItem>
        toolbarLeft={reconcileItemHeader}
        toolbarRight={
          <Button
            disabled={!selectedJobId || !canExport}
            onClick={handleExport}
            loading={exporting}
          >
            创建导出任务
          </Button>
        }
        rowKey={(record) => `${record.ref_type}-${record.ref_id}`}
        columns={itemColumns}
        data={itemList.data?.items ?? []}
        loading={itemList.isFetching}
        total={itemList.data?.total ?? 0}
        pagination={itemPagination}
        onPaginationChange={setItemPagination}
        error={itemList.error}
        onRetry={() => itemList.refetch()}
        dataTestId="finance-reconcile-items-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        emptyText="暂无差异明细"
      />
      <div className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          支付账单导入
        </Typography.Title>
        <Form form={paymentImportForm} layout="inline" onFinish={handlePaymentImport}>
          <Form.Item name="channel" label="支付渠道" rules={[{ required: true }]}>
            <Input style={{ width: 180 }} placeholder="ALIPAY / WECHAT…" autoComplete="off" />
          </Form.Item>
          <Form.Item name="period" label="账期">
            <DatePicker.RangePicker />
          </Form.Item>
          <Form.Item label="CSV文件" required>
            <Upload
              beforeUpload={() => false}
              fileList={paymentFileList}
              onChange={({ fileList: nextList }) => setPaymentFileList(nextList.slice(-1))}
              disabled={!canWrite}
            >
              <Button>选择文件</Button>
            </Upload>
          </Form.Item>
          <Form.Item>
            <Button
              type="primary"
              htmlType="submit"
              loading={paymentImporting}
              disabled={!canWrite}
            >
              导入
            </Button>
          </Form.Item>
          {paymentStatementId ? (
            <Text>最新支付账单ID: {paymentStatementId}</Text>
          ) : null}
        </Form>
      </div>

      <div className="erp-inline-section">
        <Typography.Title level={4} style={{ marginTop: 0 }}>
          运行支付对账
        </Typography.Title>
        <Form form={paymentReconcileForm} layout="inline" onFinish={handlePaymentRun}>
          <Form.Item name="statement_id" label="账单ID" rules={[{ required: true }]}>
            <Input
              style={{ width: 280 }}
              placeholder="例如 11111111-1111-1111-1111-111111111111…"
              autoComplete="off"
            />
          </Form.Item>
          <Form.Item>
            <Button
              type="primary"
              htmlType="submit"
              loading={paymentRunning}
              disabled={!canWrite}
            >
              运行对账
            </Button>
          </Form.Item>
        </Form>
      </div>

      <UiModule03MainTableSection<PaymentReconcileJob>
        toolbarLeft={paymentJobHeader}
        toolbarRight={
          <Input
            allowClear
            style={{ width: 320 }}
            placeholder="按支付账单ID筛选…"
            value={paymentStatementFilterInput}
            onChange={(event) => {
              setPaymentStatementFilterInput(event.target.value);
              setSelectedPaymentJobId(null);
            }}
            autoComplete="off"
            aria-label="支付账单ID筛选"
          />
        }
        rowKey="job_id"
        columns={paymentJobColumns}
        data={paymentJobList.data?.items ?? []}
        loading={paymentJobList.isFetching}
        total={paymentJobList.data?.total ?? 0}
        pagination={paymentJobPagination}
        onPaginationChange={setPaymentJobPagination}
        error={paymentJobList.error}
        onRetry={() => paymentJobList.refetch()}
        rowSelection={{
          type: 'radio',
          selectedRowKeys: selectedPaymentJobId ? [selectedPaymentJobId] : [],
          onChange: (keys) => setSelectedPaymentJobId(keys[0] as string)
        }}
        rowClickSelects
        dataTestId="finance-payment-reconcile-jobs-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        emptyText="暂无支付对账任务"
      />

      <UiModule03MainTableSection<PaymentReconcileItem>
        toolbarLeft={paymentItemHeader}
        rowKey={(record) => `${record.platform_order_id}-${record.job_id}`}
        columns={paymentItemColumns}
        data={paymentItemList.data?.items ?? []}
        loading={paymentItemList.isFetching}
        total={paymentItemList.data?.total ?? 0}
        pagination={paymentItemPagination}
        onPaginationChange={setPaymentItemPagination}
        error={paymentItemList.error}
        onRetry={() => paymentItemList.refetch()}
        dataTestId="finance-payment-reconcile-items-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        emptyText="暂无支付对账差异"
      />
    </SettingsPageTemplate>
  );
};
