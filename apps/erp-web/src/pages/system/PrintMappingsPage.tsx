import {
  Alert,
  Button,
  Drawer,
  Form,
  Input,
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

import { createPrintJob } from '@/api/printJobs';
import type { PrintDevice } from '@/api/printDevices';
import { listPrintDevices } from '@/api/printDevices';
import type { PrintMapping, PrintMappingUpsertPayload } from '@/api/printMappings';
import {
  deletePrintMapping,
  listPrintMappings,
  upsertPrintMapping
} from '@/api/printMappings';
import type { PrintTemplate } from '@/api/printTemplates';
import { listPrintTemplates } from '@/api/printTemplates';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FormValues = {
  doc_type: string;
  template_id: string;
  device_fingerprint?: string;
  printer_name?: string;
};

const docTypeOptions = [
  { label: '面单', value: 'SHIPPING_LABEL' },
  { label: '拣货单', value: 'PICK_LIST' },
  { label: '装箱单', value: 'PACK_LIST' },
  { label: '发票', value: 'INVOICE' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const resolveDocTypeLabel = (value: string) =>
  docTypeOptions.find((item) => item.value === value)?.label ?? value;

const generateUuid = () => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `test-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

export const PrintMappingsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('print_mappings.write');
  const canTestPrint = useCan('print_jobs.write');
  const printAgentFlag = useFeatureFlag('print.enable_local_agent');
  const queryClient = useQueryClient();
  const [drawerForm] = Form.useForm<FormValues>();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingMapping, setEditingMapping] = useState<PrintMapping | null>(null);
  const [saving, setSaving] = useState(false);
  const [testingId, setTestingId] = useState<string | null>(null);
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const mappingsQuery = useQuery({
    queryKey: ['print-mappings', tenantId],
    queryFn: () => listPrintMappings(tenantId)
  });

  const templatesQuery = useQuery({
    queryKey: ['print-templates', tenantId, 'mapping'],
    queryFn: () => listPrintTemplates(tenantId, { offset: 0, limit: 200 })
  });

  const devicesQuery = useQuery({
    queryKey: ['print-devices', tenantId, 'mapping'],
    queryFn: () => listPrintDevices(tenantId, { limit: 200 })
  });

  const templateOptions = useMemo(
    () =>
      (templatesQuery.data?.items ?? []).map((template: PrintTemplate) => ({
        label: `${template.template_type} v${template.version} (${template.shop_id})`,
        value: template.template_id
      })),
    [templatesQuery.data?.items]
  );

  const deviceOptions = useMemo(
    () =>
      (devicesQuery.data?.data ?? []).map((device: PrintDevice) => ({
        label: `${device.name} (${device.fingerprint})`,
        value: device.fingerprint
      })),
    [devicesQuery.data?.data]
  );

  const upsertMutation = useMutation({
    mutationFn: (payload: PrintMappingUpsertPayload) => upsertPrintMapping(tenantId, payload),
    onSuccess: () => {
      message.success('打印映射已更新');
      queryClient.invalidateQueries({ queryKey: ['print-mappings', tenantId] });
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (docType: string) => deletePrintMapping(tenantId, docType),
    onSuccess: () => {
      message.success('打印映射已删除');
      queryClient.invalidateQueries({ queryKey: ['print-mappings', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除打印映射失败');
    }
  });

  const columns = useMemo<ColumnsType<PrintMapping>>(
    () => [
      {
        title: '单据类型',
        dataIndex: 'doc_type',
        render: (value) => <Tag>{resolveDocTypeLabel(value)}</Tag>
      },
      {
        title: '模板',
        dataIndex: 'template_id',
        render: (value) => {
          const match = templatesQuery.data?.items?.find(
            (item) => item.template_id === value
          );
          return match ? `${match.template_type} v${match.version}` : value;
        }
      },
      {
        title: '设备指纹',
        dataIndex: 'device_fingerprint',
        render: (value) => value ?? '-'
      },
      {
        title: '打印机名',
        dataIndex: 'printer_name',
        render: (value) => value ?? '-'
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
                setEditingMapping(record);
                drawerForm.setFieldsValue({
                  doc_type: record.doc_type,
                  template_id: record.template_id,
                  device_fingerprint: record.device_fingerprint ?? undefined,
                  printer_name: record.printer_name ?? undefined
                });
                setDrawerOpen(true);
              }}
            >
              编辑
            </Button>
            <Button
              size="small"
              loading={testingId === record.mapping_id}
              disabled={!canTestPrint}
              onClick={() => handleTestPrint(record)}
            >
              测试打印
            </Button>
            <Popconfirm
              title="确认删除该映射？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.doc_type)}
              disabled={!canWrite}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, canTestPrint, deleteMutation, drawerForm, templatesQuery.data?.items, testingId]
  );

  const openCreateDrawer = () => {
    setEditingMapping(null);
    drawerForm.resetFields();
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: PrintMappingUpsertPayload = {
        doc_type: values.doc_type,
        template_id: values.template_id,
        device_fingerprint: values.device_fingerprint || null,
        printer_name: values.printer_name?.trim() || null
      };
      setSaving(true);
      await upsertMutation.mutateAsync(payload);
      setDrawerOpen(false);
      setEditingMapping(null);
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      } else {
        message.error('更新打印映射失败');
      }
    } finally {
      setSaving(false);
    }
  };

  const handleTestPrint = async (mapping: PrintMapping) => {
    try {
      setTestingId(mapping.mapping_id);
      const job = await createPrintJob(tenantId, {
        job_type: mapping.doc_type,
        ref_type: 'TEST',
        ref_id: generateUuid(),
        template_id: mapping.template_id,
        device_fingerprint: printAgentFlag.enabled
          ? mapping.device_fingerprint ?? undefined
          : undefined,
        payload: { type: 'test', doc_type: mapping.doc_type }
      });
      if (job.output_url) {
        window.open(job.output_url, '_blank', 'noopener,noreferrer');
        message.success('测试任务已输出，已打开预览');
      } else {
        message.success('测试打印任务已创建，请在打印任务中查看输出');
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '测试打印失败');
    } finally {
      setTestingId(null);
    }
  };

  const pageData = useMemo(() => {
    const items = mappingsQuery.data ?? [];
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    return items.slice((current - 1) * pageSize, current * pageSize);
  }, [mappingsQuery.data, pagination.current, pagination.pageSize]);

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      {!printAgentFlag.enabled ? (
        <Alert
          type="warning"
          showIcon
          message="本地打印代理未启用"
          description={printAgentFlag.reason || '设备映射将仅生成下载任务'}
        />
      ) : null}
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          打印映射
        </Typography.Title>
        <Button type="primary" onClick={openCreateDrawer} disabled={!canWrite}>
          新建映射
        </Button>
      </Space>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<PrintMapping>
        header={header}
        rowKey="mapping_id"
        columns={columns}
        data={pageData}
        loading={mappingsQuery.isFetching}
        error={mappingsQuery.error}
        onRetry={() => mappingsQuery.refetch()}
        errorTitle="打印映射加载失败"
        emptyText="暂无打印映射"
        total={mappingsQuery.data?.length ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        enablePanels={false}
      />
      <Drawer
        title={editingMapping ? '编辑打印映射' : '新建打印映射'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnHidden
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
            name="doc_type"
            label="单据类型"
            rules={[{ required: true, message: '请选择单据类型' }]}
          >
            <Select
              options={docTypeOptions}
              placeholder="选择单据类型"
              disabled={Boolean(editingMapping)}
            />
          </Form.Item>
          <Form.Item
            name="template_id"
            label="默认模板"
            rules={[{ required: true, message: '请选择模板' }]}
          >
            <Select options={templateOptions} placeholder="选择模板" />
          </Form.Item>
          <Form.Item name="device_fingerprint" label="默认设备">
            <Select
              allowClear
              options={deviceOptions}
              placeholder={printAgentFlag.enabled ? '选择设备' : '未启用本地代理'}
              disabled={!printAgentFlag.enabled}
            />
          </Form.Item>
          <Form.Item name="printer_name" label="打印机名称">
            <Input placeholder="例如：Zebra 110Xi4" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
