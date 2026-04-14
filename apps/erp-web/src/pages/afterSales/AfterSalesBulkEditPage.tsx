import { Button, Form, Progress, Space, Typography, Upload, message } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { UploadFile } from 'antd/es/upload/interface';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type { AfterSalesImportJob, AfterSalesImportJobListQuery } from '@/api/afterSalesOps';
import { createAfterSalesImportJob, listAfterSalesImportJobs } from '@/api/afterSalesOps';
import { ServerTable } from '@/components/ServerTable';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { downloadAfterSalesTemplate } from './afterSalesImportTemplate';

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const AfterSalesBulkEditPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('aftersales.write');
  const [fileList, setFileList] = useState<UploadFile[]>([]);
  const [importing, setImporting] = useState(false);
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });

  const query = useMemo<AfterSalesImportJobListQuery>(
    () => ({
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 10),
      limit: pagination.pageSize ?? 10
    }),
    [pagination.current, pagination.pageSize]
  );

  const { data, isFetching, error, refetch } = useQuery({
    queryKey: ['after-sales-import-jobs', tenantId, query],
    queryFn: () => listAfterSalesImportJobs(tenantId, query),
    keepPreviousData: true
  });

  const importColumns = useMemo<ColumnsType<AfterSalesImportJob>>(
    () => [
      {
        title: '任务编号',
        dataIndex: 'job_id'
      },
      {
        title: '文件名',
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
        key: 'result',
        render: (_, record) => `${record.success_rows}/${record.failed_rows}`
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value) => formatDateTime(value)
      }
    ],
    []
  );

  const handleImport = async () => {
    const file = fileList[0]?.originFileObj;
    if (!file) {
      message.warning('请先选择导入文件');
      return;
    }
    try {
      setImporting(true);
      await createAfterSalesImportJob(tenantId, file as File);
      message.success(`已提交导入任务：${file.name}`);
      setFileList([]);
      await refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '导入失败');
    } finally {
      setImporting(false);
    }
  };

  return (
    <UiModule07SettingsLayout
      title={<span data-testid="after-sales-bulk-title">批量导入售后明细</span>}
    >
      <UiModule07SettingsSection title="导入说明">
        <Form layout="vertical">
          <Form.Item label="导入说明">
            <Typography.Text type="secondary">
              支持 CSV/Excel 文件，字段包含：order_id、after_sales_type、sku_id、qty、refund_cent、amount_cent、reason_code。
            </Typography.Text>
          </Form.Item>
          <Form.Item label="上传文件">
            <div data-testid="after-sales-bulk-upload">
              <Upload
                beforeUpload={() => false}
                fileList={fileList}
                onChange={({ fileList: nextList }) => setFileList(nextList.slice(-1))}
              >
                <Button>选择文件</Button>
              </Upload>
            </div>
          </Form.Item>
          <Form.Item>
            <Space>
              <Button
                type="primary"
                onClick={handleImport}
                loading={importing}
                disabled={!canWrite}
              >
                开始导入
              </Button>
              <Button onClick={downloadAfterSalesTemplate}>下载模板</Button>
            </Space>
          </Form.Item>
        </Form>
      </UiModule07SettingsSection>
      <UiModule07SettingsSection title="导入任务">
        <ServerTable<AfterSalesImportJob>
          rowKey="job_id"
          columns={importColumns}
          data={data?.items ?? []}
          loading={isFetching}
          error={error}
          onRetry={() => refetch()}
          errorTitle="导入任务加载失败"
          emptyText="暂无导入任务"
          total={data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="after-sales.import.jobs.columns"
        />
      </UiModule07SettingsSection>
    </UiModule07SettingsLayout>
  );
};
