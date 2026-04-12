import {
  Alert,
  Button,
  Drawer,
  Form,
  Input,
  Progress,
  Select,
  Space,
  Switch,
  Tag,
  TimePicker,
  Tooltip,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  DownloadOutlined,
  FileSearchOutlined,
  KeyOutlined,
  PlusOutlined,
  QuestionCircleOutlined,
  SettingOutlined
} from '@ant-design/icons';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type { SalesShopConfig, SalesShopConfigRequest } from '@/api/salesShops';
import { listSalesShopConfigs, updateSalesShopConfig } from '@/api/salesShops';
import type { SyncJob } from '@/api/syncJobs';
import { listSyncJobs, runSyncJob } from '@/api/syncJobs';
import type { Shop } from '@/api/shops';
import { listShops, startShopAuthorize } from '@/api/shops';
import { PlatformTag } from '@/components/PlatformTag';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import type { PlatformCode } from '@/features/platform';
import { platformOptions } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type ConfigFormValues = {
  auto_download_orders: boolean;
  auto_to_review: boolean;
  download_interval_minutes: number;
  auto_merge_non_shipping_time: boolean;
  non_shipping_time_range?: [Dayjs, Dayjs];
  enable_stock_sync: boolean;
  enable_after_sales_download: boolean;
  enable_logistics_upload: boolean;
};

type DownloadMethod = 'realtime' | 'scheduled' | 'manual';

type ColumnFilters = {
  channel?: string;
  platform?: PlatformCode;
  downloadStatus?: string;
  downloadMethod?: DownloadMethod;
  authStatus?: string;
};

const intervalOptions = [5, 10, 15, 30, 60].map((value) => ({
  value,
  label: `每${value}分钟`
}));

const downloadMethodOptions = [
  { label: '实时推送', value: 'realtime' },
  { label: '定时拉取', value: 'scheduled' },
  { label: '手动下载', value: 'manual' }
];

const downloadStatusOptions = [
  { label: '任务成功', value: 'SUCCESS' },
  { label: '任务失败', value: 'FAILED' },
  { label: '下载中', value: 'RUNNING' },
  { label: '待执行', value: 'PENDING' }
];

const authStatusOptions = [
  { label: '授权正常', value: 'AUTHORIZED' },
  { label: '授权失败', value: 'EXPIRED' },
  { label: '未授权', value: 'DISABLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const resolveDownloadMethod = (config: SalesShopConfig) => {
  if (!config.auto_download_orders) {
    return { key: 'manual' as const, label: '手动下载' };
  }
  if (config.download_interval_minutes <= 5) {
    return { key: 'realtime' as const, label: '实时推送' };
  }
  return {
    key: 'scheduled' as const,
    label: `定时拉取（每${config.download_interval_minutes}分钟）`
  };
};

const resolveJobProgress = (job?: SyncJob | null) => {
  if (!job) {
    return { percent: 0, status: 'exception' as const, label: '未开始' };
  }
  switch (job.status) {
    case 'SUCCESS':
      return { percent: 100, status: 'success' as const, label: '100%' };
    case 'FAILED':
      return { percent: 0, status: 'exception' as const, label: '失败' };
    case 'RUNNING':
      return { percent: 60, status: 'active' as const, label: '下载中' };
    default:
      return { percent: 0, status: 'normal' as const, label: '等待' };
  }
};

const resolveJobStatus = (job?: SyncJob | null) => {
  if (!job) {
    return { label: '未开始', color: 'default' };
  }
  switch (job.status) {
    case 'SUCCESS':
      return { label: '任务成功', color: 'green' };
    case 'FAILED':
      return { label: '任务失败', color: 'red' };
    case 'RUNNING':
      return { label: '下载中', color: 'blue' };
    default:
      return { label: '待执行', color: 'default' };
  }
};

const resolveAuthStatus = (config: SalesShopConfig, shop?: Shop | null) => {
  const status = shop?.status ?? config.status;
  const token = shop?.token_status ?? null;
  if (token) {
    if (token.is_expired) {
      return { key: 'EXPIRED', label: '授权失败', color: 'red' };
    }
    if (token.is_expiring_soon) {
      return { key: 'AUTHORIZED', label: '即将到期', color: 'orange' };
    }
    if (token.expires_in_minutes != null) {
      const days = Math.max(Math.ceil(token.expires_in_minutes / 1440), 1);
      return { key: 'AUTHORIZED', label: `${days}天后到期`, color: 'green' };
    }
    return { key: 'AUTHORIZED', label: '授权正常', color: 'green' };
  }
  if (status === 'EXPIRED') {
    return { key: 'EXPIRED', label: '授权失败', color: 'red' };
  }
  if (status === 'DISABLED') {
    return { key: 'DISABLED', label: '未授权', color: 'default' };
  }
  return { key: 'AUTHORIZED', label: '授权正常', color: 'green' };
};

const resolveNextDownloadTime = (job: SyncJob | undefined, config: SalesShopConfig) => {
  if (!job?.last_time || !config.auto_download_orders) {
    return '-';
  }
  const next = dayjs(job.last_time).add(config.download_interval_minutes, 'minute');
  return next.isValid() ? next.format('YYYY-MM-DD HH:mm') : '-';
};

export const SalesShopConfigPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('SYS.SETTINGS.WRITE');
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [configForm] = Form.useForm<ConfigFormValues>();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [searchValue, setSearchValue] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [columnFilters, setColumnFilters] = useState<ColumnFilters>({});
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [editingConfig, setEditingConfig] = useState<SalesShopConfig | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [syncingId, setSyncingId] = useState<string | null>(null);
  const [batchDownloading, setBatchDownloading] = useState(false);

  const orderSyncFlag = useFeatureFlag('flows.order_auto_download');
  const mergeFlag = useFeatureFlag('flows.merge_outside_ship_time');
  const inventorySyncFlag = useFeatureFlag('flows.inventory_sync');
  const afterSalesFlag = useFeatureFlag('flows.aftersales_download');
  const logisticsFlag = useFeatureFlag('flows.logistics_upload');

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sales-shop-configs', tenantId],
    queryFn: () => listSalesShopConfigs(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId),
    keepPreviousData: true
  });

  const shopsQuery = useQuery({
    queryKey: ['shops', tenantId, 'sales-download'],
    queryFn: () => listShops(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    keepPreviousData: true
  });

  const syncJobsQuery = useQuery({
    queryKey: ['sync-jobs', tenantId, 'order-sync'],
    queryFn: () => listSyncJobs(tenantId, { job_type: 'order_sync', offset: 0, limit: 200 }),
    enabled: Boolean(tenantId),
    keepPreviousData: true
  });

  const disabledFlows = useMemo(() => {
    const names: string[] = [];
    if (!orderSyncFlag.enabled) {
      names.push('订单下载');
    }
    if (!mergeFlag.enabled) {
      names.push('非发货时间合并');
    }
    if (!inventorySyncFlag.enabled) {
      names.push('库存同步');
    }
    if (!afterSalesFlag.enabled) {
      names.push('售后下载');
    }
    if (!logisticsFlag.enabled) {
      names.push('物流上传');
    }
    return names;
  }, [
    afterSalesFlag.enabled,
    inventorySyncFlag.enabled,
    logisticsFlag.enabled,
    mergeFlag.enabled,
    orderSyncFlag.enabled
  ]);

  const shopMap = useMemo(() => {
    const map = new Map<string, Shop>();
    (shopsQuery.data?.items ?? []).forEach((shop) => {
      map.set(shop.shop_id, shop);
    });
    return map;
  }, [shopsQuery.data?.items]);

  const syncJobMap = useMemo(() => {
    const map = new Map<string, SyncJob>();
    (syncJobsQuery.data?.items ?? []).forEach((job) => {
      if (job.job_type === 'order_sync') {
        map.set(job.shop_id, job);
      }
    });
    return map;
  }, [syncJobsQuery.data?.items]);

  const configItems = data?.items ?? [];
  const configMap = useMemo(
    () => new Map(configItems.map((config) => [config.shop_id, config])),
    [configItems]
  );

  const filteredItems = useMemo(() => {
    const keyword = searchKeyword.trim().toLowerCase();
    return configItems.filter((config) => {
      if (keyword) {
        const channel = config.name?.toLowerCase() ?? '';
        const platform = config.platform?.toLowerCase() ?? '';
        if (!channel.includes(keyword) && !platform.includes(keyword)) {
          return false;
        }
      }
      if (columnFilters.channel) {
        const channel = config.name?.toLowerCase() ?? '';
        if (!channel.includes(columnFilters.channel.trim().toLowerCase())) {
          return false;
        }
      }
      if (columnFilters.platform && config.platform !== columnFilters.platform) {
        return false;
      }
      if (columnFilters.downloadStatus) {
        const job = syncJobMap.get(config.shop_id);
        if (!job || job.status !== columnFilters.downloadStatus) {
          return false;
        }
      }
      if (columnFilters.downloadMethod) {
        const method = resolveDownloadMethod(config);
        if (method.key !== columnFilters.downloadMethod) {
          return false;
        }
      }
      if (columnFilters.authStatus) {
        const auth = resolveAuthStatus(config, shopMap.get(config.shop_id));
        if (auth.key !== columnFilters.authStatus) {
          return false;
        }
      }
      return true;
    });
  }, [columnFilters, configItems, searchKeyword, shopMap, syncJobMap]);

  const pagedItems = useMemo(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    const start = (current - 1) * pageSize;
    return filteredItems.slice(start, start + pageSize);
  }, [filteredItems, pagination.current, pagination.pageSize]);

  const resolveDownloadDecision = (config: SalesShopConfig) => {
    if (!orderSyncFlag.enabled) {
      return { allowed: false, reason: orderSyncFlag.reason || '订单下载已关闭' };
    }
    if (config.status !== 'AUTHORIZED') {
      return { allowed: false, reason: '网店未授权' };
    }
    return { allowed: true };
  };

  const openConfig = (record: SalesShopConfig) => {
    setEditingConfig(record);
    setDrawerOpen(true);
    const range =
      record.non_shipping_time_start && record.non_shipping_time_end
        ? [
            dayjs(record.non_shipping_time_start, 'HH:mm'),
            dayjs(record.non_shipping_time_end, 'HH:mm')
          ]
        : undefined;
    configForm.setFieldsValue({
      auto_download_orders: record.auto_download_orders,
      auto_to_review: record.auto_to_review,
      download_interval_minutes: record.download_interval_minutes,
      auto_merge_non_shipping_time: record.auto_merge_non_shipping_time,
      non_shipping_time_range: range,
      enable_stock_sync: record.enable_stock_sync,
      enable_after_sales_download: record.enable_after_sales_download,
      enable_logistics_upload: record.enable_logistics_upload
    });
  };

  const handleSave = async () => {
    if (!editingConfig) {
      return;
    }
    try {
      const values = await configForm.validateFields();
      setSaving(true);
      const range = values.non_shipping_time_range;
      if (values.auto_merge_non_shipping_time && (!range || range.length < 2)) {
        message.warning('请选择非发货时间段');
        setSaving(false);
        return;
      }
      const payload: SalesShopConfigRequest = {
        auto_download_orders: values.auto_download_orders,
        auto_to_review: values.auto_to_review,
        download_interval_minutes: values.download_interval_minutes,
        auto_merge_non_shipping_time: values.auto_merge_non_shipping_time,
        non_shipping_time_start:
          values.auto_merge_non_shipping_time && range?.[0]
            ? range[0].format('HH:mm')
            : null,
        non_shipping_time_end:
          values.auto_merge_non_shipping_time && range?.[1]
            ? range[1].format('HH:mm')
            : null,
        enable_stock_sync: values.enable_stock_sync,
        enable_after_sales_download: values.enable_after_sales_download,
        enable_logistics_upload: values.enable_logistics_upload
      };
      await updateSalesShopConfig(tenantId, editingConfig.shop_id, payload);
      message.success('网店配置已更新');
      setDrawerOpen(false);
      setEditingConfig(null);
      await queryClient.invalidateQueries({ queryKey: ['sales-shop-configs', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleRunSync = async (record: SalesShopConfig) => {
    const decision = resolveDownloadDecision(record);
    if (!decision.allowed) {
      message.warning(decision.reason || '暂不可下载');
      return;
    }
    try {
      setSyncingId(record.shop_id);
      await runSyncJob(tenantId, record.shop_id, { job_type: 'order_sync' });
      message.success('下载任务已提交');
      await queryClient.invalidateQueries({ queryKey: ['sync-jobs', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '下载任务提交失败');
    } finally {
      setSyncingId(null);
    }
  };

  const handleBatchDownload = async () => {
    if (!selectedRowKeys.length) {
      message.warning('请先选择网店');
      return;
    }
    const selectedConfigs = selectedRowKeys
      .map((key) => configMap.get(key))
      .filter((config): config is SalesShopConfig => Boolean(config));
    const allowedConfigs = selectedConfigs.filter(
      (config) => resolveDownloadDecision(config).allowed
    );
    if (!allowedConfigs.length) {
      message.warning('所选网店暂不可下载');
      return;
    }
    try {
      setBatchDownloading(true);
      await Promise.all(
        allowedConfigs.map((config) =>
          runSyncJob(tenantId, config.shop_id, { job_type: 'order_sync' })
        )
      );
      message.success('下载任务已提交');
      await queryClient.invalidateQueries({ queryKey: ['sync-jobs', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '下载任务提交失败');
    } finally {
      setBatchDownloading(false);
    }
  };

  const handleAuthorize = async (record: SalesShopConfig) => {
    try {
      const response = await startShopAuthorize(tenantId, record.shop_id, {
        redirect_uri: window.location.origin
      });
      window.open(response.authorize_url, '_blank', 'noopener,noreferrer');
    } catch (err) {
      message.error(err instanceof Error ? err.message : '发起授权失败');
    }
  };

  const handleSearch = (value: string) => {
    setSearchValue(value);
    setSearchKeyword(value);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const updateColumnFilters = (patch: Partial<ColumnFilters>) => {
    setColumnFilters((prev) => ({ ...prev, ...patch }));
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const columns = useMemo<ColumnsType<SalesShopConfig>>(
    () => {
      const current = pagination.current ?? 1;
      const pageSize = pagination.pageSize ?? 20;
      return [
        {
          title: '序号',
          key: 'index',
          width: 72,
          align: 'center',
          render: (_: SalesShopConfig, __: SalesShopConfig, index: number) =>
            (current - 1) * pageSize + index + 1
        },
        {
          title: '销售渠道',
          dataIndex: 'name',
          key: 'name',
          render: (_, record) => record.name || record.shop_id
        },
        {
          title: '销售平台',
          dataIndex: 'platform',
          key: 'platform',
          render: (_, record) => <PlatformTag platform={record.platform} />
        },
        {
          title: '下载进度',
          key: 'download_progress',
          render: (_, record) => {
            const job = syncJobMap.get(record.shop_id);
            const progress = resolveJobProgress(job);
            return (
              <Progress
                percent={progress.percent}
                size="small"
                status={progress.status}
                format={() => progress.label}
              />
            );
          }
        },
        {
          title: '状态',
          key: 'download_status',
          render: (_, record) => {
            const job = syncJobMap.get(record.shop_id);
            const status = resolveJobStatus(job);
            return <Tag color={status.color}>{status.label}</Tag>;
          }
        },
        {
          title: '下载方式',
          key: 'download_method',
          render: (_, record) => resolveDownloadMethod(record).label
        },
        {
          title: '上次下载时间',
          key: 'last_download_time',
          render: (_, record) => formatDateTime(syncJobMap.get(record.shop_id)?.last_time)
        },
        {
          title: '下次下载时间',
          key: 'next_download_time',
          render: (_, record) =>
            resolveNextDownloadTime(syncJobMap.get(record.shop_id), record)
        },
        {
          title: '授权状态',
          key: 'auth_status',
          render: (_, record) => {
            const auth = resolveAuthStatus(record, shopMap.get(record.shop_id));
            return <Tag color={auth.color}>{auth.label}</Tag>;
          }
        },
        {
          title: '操作',
          key: 'actions',
          render: (_, record) => {
            const decision = resolveDownloadDecision(record);
            const downloadButton = (
              <Button
                size="small"
                type="link"
                icon={<DownloadOutlined />}
                disabled={!decision.allowed || !canWrite}
                loading={syncingId === record.shop_id}
                onClick={() => handleRunSync(record)}
              >
                下载
              </Button>
            );
            return (
              <Space size={4}>
                <Button
                  size="small"
                  type="link"
                  icon={<KeyOutlined />}
                  disabled={!canWrite}
                  onClick={() => handleAuthorize(record)}
                >
                  授权
                </Button>
                {decision.allowed ? (
                  downloadButton
                ) : (
                  <Tooltip title={decision.reason}>
                    <span>{downloadButton}</span>
                  </Tooltip>
                )}
                <Button
                  size="small"
                  type="link"
                  icon={<SettingOutlined />}
                  disabled={!canWrite}
                  onClick={() => openConfig(record)}
                >
                  网店配置
                </Button>
              </Space>
            );
          }
        }
      ];
    },
    [
      canWrite,
      pagination.current,
      pagination.pageSize,
      resolveDownloadDecision,
      shopMap,
      syncJobMap,
      syncingId
    ]
  );

  const queryCells = useMemo(
    () => ({
      name: (
        <Input
          size="small"
          allowClear
          placeholder="销售渠道"
          value={columnFilters.channel}
          onChange={(event) => updateColumnFilters({ channel: event.target.value })}
        />
      ),
      platform: (
        <Select
          size="small"
          allowClear
          placeholder="销售平台"
          options={platformOptions}
          value={columnFilters.platform}
          onChange={(value) => updateColumnFilters({ platform: value ?? undefined })}
        />
      ),
      download_status: (
        <Select
          size="small"
          allowClear
          placeholder="状态"
          options={downloadStatusOptions}
          value={columnFilters.downloadStatus}
          onChange={(value) => updateColumnFilters({ downloadStatus: value ?? undefined })}
        />
      ),
      download_method: (
        <Select
          size="small"
          allowClear
          placeholder="下载方式"
          options={downloadMethodOptions}
          value={columnFilters.downloadMethod}
          onChange={(value) =>
            updateColumnFilters({ downloadMethod: value as DownloadMethod | undefined })
          }
        />
      ),
      auth_status: (
        <Select
          size="small"
          allowClear
          placeholder="授权状态"
          options={authStatusOptions}
          value={columnFilters.authStatus}
          onChange={(value) => updateColumnFilters({ authStatus: value ?? undefined })}
        />
      )
    }),
    [columnFilters, updateColumnFilters]
  );

  const toolbarLeft = (
    <Space size={8} wrap={false} align="center">
      <Button
        type="primary"
        icon={<PlusOutlined />}
        disabled={!canWrite}
        onClick={() => message.info('新增网店功能待接入')}
      >
        新增网店
      </Button>
      <Button
        icon={<DownloadOutlined />}
        disabled={!canWrite}
        loading={batchDownloading}
        onClick={handleBatchDownload}
      >
        下载订单
      </Button>
      <Button
        icon={<SettingOutlined />}
        disabled={!canWrite}
        onClick={() => {
          if (!selectedRowKeys.length) {
            message.warning('请先选择网店');
            return;
          }
          if (selectedRowKeys.length > 1) {
            message.warning('一次只能修改一个网店');
            return;
          }
          const selected = configMap.get(selectedRowKeys[0]);
          if (!selected) {
            message.warning('未找到选中的网店');
            return;
          }
          openConfig(selected);
        }}
      >
        修改网店配置
      </Button>
      <Button
        icon={<FileSearchOutlined />}
        onClick={() => navigate('/sales/sync-job-runs')}
      >
        网店日志
      </Button>
    </Space>
  );

  const toolbarCenter = (
    <Input.Search
      allowClear
      value={searchValue}
      placeholder="请输入销售渠道或销售平台名称查询"
      onSearch={handleSearch}
      onChange={(event) => setSearchValue(event.target.value)}
      style={{ width: 280 }}
    />
  );

  const toolbarRight = (
    <Space size={8}>
      <Button
        icon={<QuestionCircleOutlined />}
        onClick={() => message.info('帮助功能待接入')}
      >
        帮助
      </Button>
    </Space>
  );

  const header = disabledFlows.length ? (
    <Alert
      type="warning"
      showIcon
      message={`以下流程已关闭：${disabledFlows.join('、')}`}
    />
  ) : null;

  const mergedLoading =
    isLoading ||
    isFetching ||
    shopsQuery.isLoading ||
    shopsQuery.isFetching ||
    syncJobsQuery.isLoading ||
    syncJobsQuery.isFetching;
  const mergedError = error || shopsQuery.error || syncJobsQuery.error;

  const handleRetry = () => {
    refetch();
    shopsQuery.refetch();
    syncJobsQuery.refetch();
  };

  return (
    <>
      <UiModule03MainTableSection<SalesShopConfig>
        toolbarLeft={toolbarLeft}
        toolbarCenter={toolbarCenter}
        toolbarRight={toolbarRight}
        header={header}
        rowKey="shop_id"
        columns={columns}
        data={pagedItems}
        loading={mergedLoading}
        error={mergedError}
        onRetry={handleRetry}
        errorTitle="网店下载列表加载失败"
        emptyText="暂无网店记录"
        total={filteredItems.length}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="sales.shop.download.columns"
        rowSelection={{
          selectedRowKeys,
          onChange: (keys) => setSelectedRowKeys(keys as string[])
        }}
        queryCells={queryCells}
      />

      <Drawer
        title={editingConfig ? `配置 - ${editingConfig.name}` : '网店配置'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={420}
        destroyOnClose
        footer={
          <Space style={{ display: 'flex', justifyContent: 'flex-end' }}>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={configForm} layout="vertical">
          <Form.Item
            name="auto_download_orders"
            label="自动下载订单"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>
          <Form.Item name="auto_to_review" label="自动转入审核" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item
            name="download_interval_minutes"
            label="定时下载"
            rules={[{ required: true, message: '请选择下载间隔' }]}
          >
            <Select options={intervalOptions} />
          </Form.Item>
          <Form.Item
            name="auto_merge_non_shipping_time"
            label="非发货时间自动合并"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>
          <Form.Item shouldUpdate>
            {() => (
              <Form.Item name="non_shipping_time_range" label="非发货时间段">
                <TimePicker.RangePicker
                  format="HH:mm"
                  minuteStep={5}
                  style={{ width: '100%' }}
                  disabled={!configForm.getFieldValue('auto_merge_non_shipping_time')}
                />
              </Form.Item>
            )}
          </Form.Item>
          <Form.Item name="enable_stock_sync" label="库存同步" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item
            name="enable_after_sales_download"
            label="售后下载"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>
          <Form.Item
            name="enable_logistics_upload"
            label="物流上传"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
