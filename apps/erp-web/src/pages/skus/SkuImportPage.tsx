import { Alert, Button, Card, Descriptions, Space, Table, Typography, Upload, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { UploadFile } from 'antd/es/upload/interface';
import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import type {
  SkuImportIssue,
  SkuImportPreviewRow,
  SkuImportRequest,
  SkuImportPrecheckResponse
} from '@/api/skus';
import { createSkuImport, precheckSkuImport } from '@/api/skus';
import type { components } from '@/api/generated/schema';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { UiModule02WorkbenchTemplate } from '@/components/ui-modules';

type ExportJob = components['schemas']['ExportJob'];

const readFileAsBase64 = (file: File): Promise<string> =>
  new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      if (typeof result !== 'string') {
        reject(new Error('Invalid file'));
        return;
      }
      const base64 = result.includes(',') ? result.split(',')[1] : result;
      resolve(base64);
    };
    reader.onerror = () => reject(new Error('Failed to read file'));
    reader.readAsDataURL(file);
  });

const resolveFileType = (fileName?: string) => {
  if (!fileName) {
    return 'CSV';
  }
  const lower = fileName.toLowerCase();
  if (lower.endsWith('.xlsx')) {
    return 'XLSX';
  }
  if (lower.endsWith('.xls')) {
    return 'XLS';
  }
  return 'CSV';
};

export const SkuImportPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const navigate = useNavigate();
  const [fileList, setFileList] = useState<UploadFile[]>([]);
  const [precheck, setPrecheck] = useState<SkuImportPrecheckResponse | null>(null);
  const [importPayload, setImportPayload] = useState<SkuImportRequest | null>(null);
  const [prechecking, setPrechecking] = useState(false);
  const [importing, setImporting] = useState(false);
  const [job, setJob] = useState<ExportJob | null>(null);

  const buildPayload = async () => {
    if (!fileList.length || !fileList[0]?.originFileObj) {
      throw new Error('请选择导入文件');
    }
    const file = fileList[0].originFileObj as File;
    const content_base64 = await readFileAsBase64(file);
    return {
      file_name: file.name,
      file_type: resolveFileType(file.name),
      content_base64
    };
  };

  const handlePrecheck = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!canWrite) {
      message.error('暂无权限');
      return;
    }
    setPrechecking(true);
    setJob(null);
    try {
      const payload = await buildPayload();
      const result = await precheckSkuImport(tenantId, payload);
      setPrecheck(result);
      setImportPayload(payload);
      message.success('预检完成');
    } catch (err) {
      message.error(err instanceof Error ? err.message : '预检失败');
    } finally {
      setPrechecking(false);
    }
  };

  const handleImport = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!importPayload) {
      message.error('请先完成预检');
      return;
    }
    setImporting(true);
    try {
      const result = await createSkuImport(tenantId, importPayload);
      setJob(result);
      message.success('导入任务已创建');
    } catch (err) {
      message.error(err instanceof Error ? err.message : '创建导入任务失败');
    } finally {
      setImporting(false);
    }
  };

  const errorColumns = useMemo<ColumnsType<SkuImportIssue>>(
    () => [
      { title: '行号', dataIndex: 'row', width: 80 },
      { title: '字段', dataIndex: 'field', render: (value) => value ?? '-' },
      { title: '问题', dataIndex: 'message' }
    ],
    []
  );

  const previewColumns = useMemo<ColumnsType<SkuImportPreviewRow>>(
    () => [
      { title: '行号', dataIndex: 'row', width: 80 },
      { title: '商品', dataIndex: 'product_name', render: (value) => value ?? '-' },
      { title: 'SKU', dataIndex: 'sku_code' },
      { title: '条码', dataIndex: 'barcode', render: (value) => value ?? '-' },
      { title: '名称', dataIndex: 'name', render: (value) => value ?? '-' },
      { title: '类型', dataIndex: 'sku_kind', render: (value) => value ?? '-' },
      {
        title: '标签',
        dataIndex: 'tags',
        render: (value: string[]) => (value?.length ? value.join(', ') : '-')
      },
      {
        title: '图片',
        dataIndex: 'image_urls',
        render: (value: string[]) => (value?.length ? `${value.length} 张` : '-')
      },
      {
        title: '成本',
        dataIndex: 'cost_cent',
        render: (value?: number | null) => (value == null ? '-' : `¥${(value / 100).toFixed(2)}`)
      },
      {
        title: '供应商',
        dataIndex: 'supplier_ids',
        render: (value: string[]) => (value?.length ? value.join(', ') : '-')
      }
    ],
    []
  );

  const hasErrors = Boolean(precheck?.errors.length);

  return (
    <UiModule02WorkbenchTemplate
      header={
        <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
          <Typography.Title level={3} style={{ margin: 0 }}>
            SKU 导入
          </Typography.Title>
          <Button onClick={() => navigate('/exports')}>查看任务</Button>
        </Space>
      }
      tableContent={
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Card>
            <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Alert
            type="info"
            showIcon
            message="支持 CSV / Excel (XLS/XLSX) 文件"
            description="请先上传文件并执行预检，预检通过后创建导入任务。"
          />
          <Upload
            beforeUpload={() => false}
            fileList={fileList}
            onChange={({ fileList: nextList }) => {
              setFileList(nextList.slice(-1));
              setPrecheck(null);
              setImportPayload(null);
              setJob(null);
            }}
            disabled={!canWrite}
          >
            <Button>选择文件</Button>
          </Upload>
          <Space>
            <Button type="primary" onClick={handlePrecheck} loading={prechecking} disabled={!canWrite}>
              导入预检
            </Button>
            <Button
              onClick={handleImport}
              type="primary"
              loading={importing}
              disabled={!canWrite || !precheck || precheck.valid === 0}
            >
              创建导入任务
            </Button>
          </Space>
        </Space>
      </Card>
      {precheck ? (
        <Card title="预检结果">
          <Descriptions column={3} size="small">
            <Descriptions.Item label="总行数">{precheck.total}</Descriptions.Item>
            <Descriptions.Item label="可导入">{precheck.valid}</Descriptions.Item>
            <Descriptions.Item label="错误数">{precheck.errors.length}</Descriptions.Item>
          </Descriptions>
          {hasErrors ? (
            <Table<SkuImportIssue>
              rowKey={(record) => `${record.row}-${record.field ?? 'NA'}`}
              columns={errorColumns}
              dataSource={precheck.errors}
              pagination={{ pageSize: 10 }}
              style={{ marginTop: 16 }}
            />
          ) : null}
          <Table<SkuImportPreviewRow>
            rowKey={(record) => `${record.row}-${record.sku_code}`}
            columns={previewColumns}
            dataSource={precheck.preview}
            pagination={{ pageSize: 10 }}
            style={{ marginTop: 16 }}
          />
        </Card>
      ) : null}
      {job ? (
        <Card title="导入任务">
          <Space direction="vertical">
            <Typography.Text>任务 ID: {job.job_id}</Typography.Text>
            <Typography.Text>状态: {job.status}</Typography.Text>
            <Typography.Text>进度: {job.progress}%</Typography.Text>
          </Space>
        </Card>
      ) : null}
        </Space>
      }
    />
  );
};
