import {
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  Popconfirm,
  Row,
  Select,
  Space,
  Switch,
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  PrintDevice,
  PrintDeviceCreatePayload,
  PrintDeviceRegisterPayload
} from '@/api/printDevices';
import {
  createPrintDevice,
  listPrintDevices,
  registerPrintDevice,
  updatePrintDevice
} from '@/api/printDevices';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  fingerprint?: string;
  status?: string;
};

type FormValues = {
  name: string;
  fingerprint: string;
  status?: string;
  is_default?: boolean;
};

type RegisterFormValues = {
  device_fingerprint: string;
  name?: string;
  status?: string;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const resolveTotal = (
  pagination: TablePaginationConfig,
  nextCursor?: string | null
): number => {
  const current = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 20;
  return nextCursor ? (current + 1) * pageSize : current * pageSize;
};

export const PrintDevicesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('print_devices.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [registerForm] = Form.useForm<RegisterFormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [cursorMap, setCursorMap] = useState<Record<number, string | undefined>>({
    1: undefined
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [registerOpen, setRegisterOpen] = useState(false);
  const [editingDevice, setEditingDevice] = useState<PrintDevice | null>(null);
  const [saving, setSaving] = useState(false);
  const [registering, setRegistering] = useState(false);
  const [updatingId, setUpdatingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      fingerprint: filters.fingerprint?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      cursor: cursorMap[pagination.current ?? 1],
      limit: pagination.pageSize ?? 20
    }),
    [cursorMap, filters, pagination.current, pagination.pageSize]
  );

  const devicesQuery = useQuery({
    queryKey: ['print-devices', tenantId, query],
    queryFn: () => listPrintDevices(tenantId, query),
    keepPreviousData: true
  });

  useEffect(() => {
    const nextCursor = devicesQuery.data?.next_cursor ?? undefined;
    if (!nextCursor) {
      return;
    }
    const current = pagination.current ?? 1;
    setCursorMap((prev) => {
      if (prev[current + 1]) {
        return prev;
      }
      return { ...prev, [current + 1]: nextCursor };
    });
  }, [devicesQuery.data?.next_cursor, pagination.current]);

  const updateMutation = useMutation({
    mutationFn: ({ deviceId, payload }: { deviceId: string; payload: FormValues }) =>
      updatePrintDevice(tenantId, deviceId, {
        name: payload.name?.trim() || undefined,
        status: payload.status?.trim() || undefined,
        is_default: payload.is_default ?? undefined
      }),
    onMutate: ({ deviceId }) => setUpdatingId(deviceId),
    onSuccess: () => {
      message.success('打印设备已更新');
      queryClient.invalidateQueries({ queryKey: ['print-devices', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '更新打印设备失败');
    },
    onSettled: () => setUpdatingId(null)
  });

  const columns = useMemo<ColumnsType<PrintDevice>>(
    () => [
      {
        title: '设备名称',
        dataIndex: 'name'
      },
      {
        title: '指纹',
        dataIndex: 'fingerprint'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) => (value === 'DISABLED' ? <Tag color="red">停用</Tag> : '启用')
      },
      {
        title: '默认',
        dataIndex: 'is_default',
        render: (value) => (value ? <Tag color="green">默认</Tag> : '-')
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => {
          const nextStatus = record.status === 'DISABLED' ? 'ACTIVE' : 'DISABLED';
          return (
            <Space>
              <Button
                size="small"
                disabled={!canWrite}
                onClick={() => {
                  setEditingDevice(record);
                  drawerForm.setFieldsValue({
                    name: record.name,
                    fingerprint: record.fingerprint,
                    status: record.status,
                    is_default: record.is_default
                  });
                  setDrawerOpen(true);
                }}
              >
                编辑
              </Button>
              <Popconfirm
                title={record.status === 'DISABLED' ? '确认启用该设备？' : '确认停用该设备？'}
                okText={record.status === 'DISABLED' ? '启用' : '停用'}
                cancelText="取消"
                onConfirm={() =>
                  updateMutation.mutate({
                    deviceId: record.device_id,
                    payload: {
                      name: record.name,
                      fingerprint: record.fingerprint,
                      status: nextStatus,
                      is_default: record.is_default
                    }
                  })
                }
                disabled={!canWrite}
              >
                <Button
                  size="small"
                  loading={updatingId === record.device_id}
                  disabled={!canWrite}
                >
                  {record.status === 'DISABLED' ? '启用' : '停用'}
                </Button>
              </Popconfirm>
            </Space>
          );
        }
      }
    ],
    [canWrite, drawerForm, updateMutation, updatingId]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
    setCursorMap({ 1: undefined });
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
    setCursorMap({ 1: undefined });
  };

  const openCreateDrawer = () => {
    setEditingDevice(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ status: 'ACTIVE' });
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      setSaving(true);
      if (editingDevice) {
        await updateMutation.mutateAsync({
          deviceId: editingDevice.device_id,
          payload: values
        });
      } else {
        const payload: PrintDeviceCreatePayload = {
          name: values.name.trim(),
          fingerprint: values.fingerprint.trim(),
          status: values.status?.trim() || undefined,
          is_default: values.is_default
        };
        await createPrintDevice(tenantId, payload);
        message.success('打印设备已创建');
        queryClient.invalidateQueries({ queryKey: ['print-devices', tenantId] });
      }
      setDrawerOpen(false);
      setEditingDevice(null);
    } catch (err) {
      if (!editingDevice && err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleRegister = async () => {
    try {
      const values = await registerForm.validateFields();
      setRegistering(true);
      const payload: PrintDeviceRegisterPayload = {
        device_fingerprint: values.device_fingerprint.trim(),
        name: values.name?.trim() || undefined,
        status: values.status?.trim() || undefined
      };
      await registerPrintDevice(tenantId, payload);
      message.success('设备已注册');
      setRegisterOpen(false);
      registerForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['print-devices', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setRegistering(false);
    }
  };

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          打印设备
        </Typography.Title>
        <Space>
          <Button onClick={() => setRegisterOpen(true)} disabled={!canWrite}>
            注册设备
          </Button>
          <Button type="primary" onClick={openCreateDrawer} disabled={!canWrite}>
            新建设备
          </Button>
        </Space>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="fingerprint" label="指纹">
                <Input allowClear placeholder="设备指纹" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="全部状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
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
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<PrintDevice>
        header={header}
        rowKey="device_id"
        columns={columns}
        data={devicesQuery.data?.data ?? []}
        loading={devicesQuery.isFetching}
        error={devicesQuery.error}
        onRetry={() => devicesQuery.refetch()}
        errorTitle="打印设备加载失败"
        emptyText="暂无打印设备"
        total={resolveTotal(pagination, devicesQuery.data?.next_cursor)}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="print-devices.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editingDevice ? '编辑打印设备' : '新建打印设备'}
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
            name="name"
            label="设备名称"
            rules={[{ required: true, message: '请输入设备名称' }]}
          >
            <Input placeholder="例如：前台打印机" />
          </Form.Item>
          <Form.Item
            name="fingerprint"
            label="设备指纹"
            rules={[{ required: true, message: '请输入设备指纹' }]}
          >
            <Input placeholder="由本地打印代理生成" disabled={Boolean(editingDevice)} />
          </Form.Item>
          <Form.Item name="status" label="状态" initialValue="ACTIVE">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="is_default" label="设为默认" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Drawer>
      <Drawer
        title="注册打印设备"
        open={registerOpen}
        onClose={() => setRegisterOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setRegisterOpen(false)}>取消</Button>
            <Button type="primary" loading={registering} onClick={handleRegister}>
              注册
            </Button>
          </Space>
        }
      >
        <Form form={registerForm} layout="vertical">
          <Form.Item
            name="device_fingerprint"
            label="设备指纹"
            rules={[{ required: true, message: '请输入设备指纹' }]}
          >
            <Input placeholder="从本地打印代理获取指纹" />
          </Form.Item>
          <Form.Item name="name" label="设备名称">
            <Input placeholder="例如：仓库打印机" />
          </Form.Item>
          <Form.Item name="status" label="状态" initialValue="ACTIVE">
            <Select options={statusOptions} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
