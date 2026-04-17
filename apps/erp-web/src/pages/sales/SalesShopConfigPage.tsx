import {
  Alert,
  Button,
  Card,
  DatePicker,
  Descriptions,
  Drawer,
  Empty,
  Form,
  Input,
  Modal,
  Progress,
  Radio,
  Select,
  Space,
  Switch,
  Tag,
  TimePicker,
  Tooltip,
  Typography,
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
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useLocation, useNavigate } from 'react-router-dom';

import type { SalesShopConfig, SalesShopConfigRequest } from '@/api/salesShops';
import { listSalesShopConfigs, updateSalesShopConfig } from '@/api/salesShops';
import type {
  PlatformOrderNoDownloadResponse,
  SyncJob
} from '@/api/syncJobs';
import {
  downloadPlatformOrdersByOrderNos,
  downloadPlatformOrdersByTimeRange,
  downloadPlatformOrdersManual,
  listSyncJobs,
  runSyncJob
} from '@/api/syncJobs';
import type { Shop } from '@/api/shops';
import {
  listShops,
  requestTokenRefresh,
  startShopAuthorize,
  submitShopAuthCallback
} from '@/api/shops';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { PlatformTag } from '@/components/PlatformTag';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import type { PlatformCode } from '@/features/platform';
import { platformOptions } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import {
  buildCreateChannelRoute,
  buildShopAuthCallbackUrl,
  parseShopAuthCallbackSearch
} from '@/pages/channels/shopAccessModel';
import { routerBasePath, routerMode } from '@/utils/env';

import {
  buildSalesShopPageWarnings,
  buildSalesShopRows,
  resolveSalesShopPageError
} from './salesShopConfigModel';

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
type ManualDownloadMode = 'incremental' | 'order-nos' | 'time-range';

type ColumnFilters = {
  channel?: string;
  platform?: PlatformCode;
  downloadStatus?: string;
  downloadMethod?: DownloadMethod;
  authStatus?: string;
};

type ManualDownloadFormValues = {
  mode: ManualDownloadMode;
  order_nos_text?: string;
  time_range?: [Dayjs, Dayjs];
  cursor?: string;
  watermark?: string;
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

const manualDownloadModeOptions: Array<{ label: string; value: ManualDownloadMode }> = [
  { label: '增量下载', value: 'incremental' },
  { label: '按订单号下载', value: 'order-nos' },
  { label: '按日期下载', value: 'time-range' }
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

const parseOrderNosInput = (value?: string) =>
  (value ?? '')
    .split(/[\n,，\s]+/)
    .map((item) => item.trim())
    .filter(Boolean);

const isPlatformCode = (value?: string): value is PlatformCode =>
  Boolean(value && platformOptions.some((option) => option.value === value));

export const SalesShopConfigPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('SYS.SETTINGS.WRITE');
  const location = useLocation();
  const navigate = useNavigate();
  const {
    setLeftPanel,
    setRightPanel,
    setBottomPanel,
    setLeftPanelVisible,
    setRightPanelVisible,
    setBottomPanelVisible,
    setRightPanelWidth,
    clearPanels
  } = useWorkbenchPanels();
  const queryClient = useQueryClient();
  const [configForm] = Form.useForm<ConfigFormValues>();
  const [manualDownloadForm] = Form.useForm<ManualDownloadFormValues>();
  const manualDownloadMode = Form.useWatch('mode', manualDownloadForm);
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [searchValue, setSearchValue] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [columnFilters, setColumnFilters] = useState<ColumnFilters>({});
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [activeShopId, setActiveShopId] = useState<string | null>(null);
  const [editingConfig, setEditingConfig] = useState<SalesShopConfig | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [syncingId, setSyncingId] = useState<string | null>(null);
  const [batchDownloading, setBatchDownloading] = useState(false);
  const [manualDownloadOpen, setManualDownloadOpen] = useState(false);
  const [manualDownloadTarget, setManualDownloadTarget] =
    useState<SalesShopConfig | null>(null);
  const [manualDownloadSubmitting, setManualDownloadSubmitting] = useState(false);
  const [manualDownloadResult, setManualDownloadResult] =
    useState<PlatformOrderNoDownloadResponse | null>(null);
  const [refreshingTokenShopId, setRefreshingTokenShopId] = useState<string | null>(null);
  const callbackRequestKeyRef = useRef<string | null>(null);

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
  const shopItems = shopsQuery.data?.items ?? [];
  const baseItems = useMemo(
    () => buildSalesShopRows(configItems, shopItems),
    [configItems, shopItems]
  );
  const configMap = useMemo(
    () => new Map(baseItems.map((config) => [config.shop_id, config])),
    [baseItems]
  );
  const authCallbackState = useMemo(
    () => parseShopAuthCallbackSearch(location.search),
    [location.search]
  );
  const selectedConfigId = useMemo(() => {
    if (activeShopId && configMap.has(activeShopId)) {
      return activeShopId;
    }
    if (selectedRowKeys.length === 1 && configMap.has(selectedRowKeys[0])) {
      return selectedRowKeys[0];
    }
    return null;
  }, [activeShopId, configMap, selectedRowKeys]);
  const selectedConfig = useMemo(
    () => (selectedConfigId ? configMap.get(selectedConfigId) ?? null : null),
    [configMap, selectedConfigId]
  );
  const selectedShop = useMemo(
    () => (selectedConfig ? shopMap.get(selectedConfig.shop_id) ?? null : null),
    [selectedConfig, shopMap]
  );
  const selectedSyncJob = useMemo(
    () => (selectedConfig ? syncJobMap.get(selectedConfig.shop_id) : undefined),
    [selectedConfig, syncJobMap]
  );
  const selectedAuthStatus = useMemo(
    () => (selectedConfig ? resolveAuthStatus(selectedConfig, selectedShop) : null),
    [selectedConfig, selectedShop]
  );
  const selectedCallbackUrl = useMemo(
    () =>
      selectedConfig
        ? buildShopAuthCallbackUrl({
            origin: window.location.origin,
            routerMode,
            routerBasePath,
            shopId: selectedConfig.shop_id
          })
        : '-',
    [selectedConfig]
  );

  const filteredItems = useMemo(() => {
    const keyword = searchKeyword.trim().toLowerCase();
    return baseItems.filter((config) => {
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
  }, [baseItems, columnFilters, searchKeyword, shopMap, syncJobMap]);

  const pagedItems = useMemo(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    const start = (current - 1) * pageSize;
    return filteredItems.slice(start, start + pageSize);
  }, [filteredItems, pagination.current, pagination.pageSize]);

  const resolveShopCallbackUrl = useCallback(
    (shopId: string) =>
      buildShopAuthCallbackUrl({
        origin: window.location.origin,
        routerMode,
        routerBasePath,
        shopId
      }),
    []
  );

  useEffect(() => {
    if (!authCallbackState || !tenantId) {
      callbackRequestKeyRef.current = null;
      return;
    }

    const requestKey = [
      authCallbackState.shopId ?? '',
      authCallbackState.code ?? '',
      authCallbackState.error ?? '',
      authCallbackState.state ?? ''
    ].join('|');
    if (callbackRequestKeyRef.current === requestKey) {
      return;
    }
    callbackRequestKeyRef.current = requestKey;

    if (!authCallbackState.shopId) {
      message.error('授权回调缺少店铺ID，无法完成授权绑定。');
      navigate('/channels/shops', { replace: true });
      return;
    }

    void (async () => {
      const shopId = authCallbackState.shopId as string;
      try {
        await submitShopAuthCallback(tenantId, shopId, {
          code: authCallbackState.code,
          state: authCallbackState.state,
          error: authCallbackState.error,
          error_description: authCallbackState.errorDescription,
          redirect_uri: resolveShopCallbackUrl(shopId)
        });
        await queryClient.invalidateQueries({ queryKey: ['shops', tenantId] });
        await queryClient.invalidateQueries({ queryKey: ['sales-shop-configs', tenantId] });
        if (authCallbackState.hasError) {
          message.error(authCallbackState.errorDescription || authCallbackState.error || '授权失败');
        } else {
          message.success('授权回调已接收，店铺状态已更新。');
        }
      } catch (callbackError) {
        message.error(callbackError instanceof Error ? callbackError.message : '授权回调处理失败');
      } finally {
        navigate('/channels/shops', { replace: true });
      }
    })();
  }, [authCallbackState, navigate, queryClient, resolveShopCallbackUrl, tenantId]);

  const resolveDownloadDecision = useCallback(
    (config: SalesShopConfig): DownloadDecision => {
      if (!orderSyncFlag.enabled) {
        return { allowed: false, reason: orderSyncFlag.reason || '订单下载已关闭。' };
      }
      const auth = resolveAuthStatus(config, shopMap.get(config.shop_id));
      if (auth.key !== 'AUTHORIZED') {
        return { allowed: false, reason: '网店尚未完成有效授权。' };
      }
      return { allowed: true };
    },
    [orderSyncFlag.enabled, orderSyncFlag.reason, shopMap]
  );

  const openConfig = useCallback(
    (record: SalesShopConfig) => {
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
    },
    [configForm]
  );

  const handleSave = useCallback(async () => {
    if (!editingConfig) {
      return;
    }
    try {
      const values = await configForm.validateFields();
      setSaving(true);
      const range = values.non_shipping_time_range;
      if (values.auto_merge_non_shipping_time && (!range || range.length < 2)) {
        message.warning('请选择非发货时间段。');
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
      message.success('网店配置已更新。');
      setDrawerOpen(false);
      setEditingConfig(null);
      await queryClient.invalidateQueries({ queryKey: ['sales-shop-configs', tenantId] });
    } catch (saveError) {
      if (saveError instanceof Error && saveError.message) {
        message.error(saveError.message);
      }
    } finally {
      setSaving(false);
    }
  }, [configForm, editingConfig, queryClient, tenantId]);

  const handleRunSync = useCallback(
    async (record: SalesShopConfig) => {
      const decision = resolveDownloadDecision(record);
      if (!decision.allowed) {
        message.warning(decision.reason || '暂不可下载。');
        return;
      }
      try {
        setSyncingId(record.shop_id);
        await runSyncJob(tenantId, record.shop_id, { job_type: 'order_sync' });
        message.success('下载任务已提交。');
        await queryClient.invalidateQueries({ queryKey: ['sync-jobs', tenantId] });
      } catch (syncError) {
        message.error(syncError instanceof Error ? syncError.message : '下载任务提交失败');
      } finally {
        setSyncingId(null);
      }
    },
    [queryClient, resolveDownloadDecision, tenantId]
  );

  const openManualDownload = useCallback(
    (record: SalesShopConfig) => {
      setManualDownloadTarget(record);
      setManualDownloadResult(null);
      manualDownloadForm.setFieldsValue({
        mode: 'incremental',
        order_nos_text: '',
        time_range: undefined,
        cursor: undefined,
        watermark: undefined
      });
      setManualDownloadOpen(true);
    },
    [manualDownloadForm]
  );

  const handleManualDownload = useCallback(async () => {
    if (!manualDownloadTarget) {
      return;
    }
    try {
      const values = await manualDownloadForm.validateFields();
      setManualDownloadSubmitting(true);
      setManualDownloadResult(null);
      if (values.mode === 'order-nos') {
        const orderNos = parseOrderNosInput(values.order_nos_text);
        if (!orderNos.length) {
          message.warning('请至少输入一个订单号。');
          setManualDownloadSubmitting(false);
          return;
        }
        const result = await downloadPlatformOrdersByOrderNos(tenantId, manualDownloadTarget.shop_id, {
          order_nos: orderNos
        });
        setManualDownloadResult(result);
        message.success(`已处理 ${result.total} 条订单号下载请求。`);
      } else if (values.mode === 'time-range') {
        const range = values.time_range;
        if (!range || range.length < 2) {
          message.warning('请选择下载日期范围。');
          setManualDownloadSubmitting(false);
          return;
        }
        await downloadPlatformOrdersByTimeRange(tenantId, manualDownloadTarget.shop_id, {
          start_time: range[0].toISOString(),
          end_time: range[1].toISOString(),
          cursor: values.cursor?.trim() || undefined
        });
        message.success('按日期下载任务已提交。');
      } else {
        await downloadPlatformOrdersManual(tenantId, manualDownloadTarget.shop_id, {
          cursor: values.cursor?.trim() || undefined,
          watermark: values.watermark?.trim() || undefined
        });
        message.success('增量下载任务已提交。');
      }
      await queryClient.invalidateQueries({ queryKey: ['sync-jobs', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['sales-shop-configs', tenantId] });
    } catch (manualError) {
      if (manualError instanceof Error) {
        message.error(manualError.message);
      }
    } finally {
      setManualDownloadSubmitting(false);
    }
  }, [manualDownloadForm, manualDownloadTarget, queryClient, tenantId]);

  const handleRefreshToken = useCallback(
    async (record: SalesShopConfig) => {
      try {
        setRefreshingTokenShopId(record.shop_id);
        await requestTokenRefresh(tenantId, record.shop_id);
        message.success('Token 刷新任务已提交。');
        await queryClient.invalidateQueries({ queryKey: ['shops', tenantId] });
      } catch (refreshError) {
        message.error(refreshError instanceof Error ? refreshError.message : '刷新 Token 失败');
      } finally {
        setRefreshingTokenShopId(null);
      }
    },
    [queryClient, tenantId]
  );

  const handleBatchDownload = useCallback(async () => {
    if (!selectedRowKeys.length) {
      message.warning('请先选择网店。');
      return;
    }
    const selectedConfigs = selectedRowKeys
      .map((key) => configMap.get(key))
      .filter((config): config is SalesShopConfig => Boolean(config));
    const allowedConfigs = selectedConfigs.filter(
      (config) => resolveDownloadDecision(config).allowed
    );
    if (!allowedConfigs.length) {
      message.warning('所选网店暂不可下载。');
      return;
    }
    try {
      setBatchDownloading(true);
      await Promise.all(
        allowedConfigs.map((config) =>
          runSyncJob(tenantId, config.shop_id, { job_type: 'order_sync' })
        )
      );
      message.success('批量下载任务已提交。');
      await queryClient.invalidateQueries({ queryKey: ['sync-jobs', tenantId] });
    } catch (batchError) {
      message.error(batchError instanceof Error ? batchError.message : '批量下载任务提交失败');
    } finally {
      setBatchDownloading(false);
    }
  }, [configMap, queryClient, resolveDownloadDecision, selectedRowKeys, tenantId]);

  const handleAuthorize = useCallback(
    async (record: SalesShopConfig) => {
      try {
        const response = await startShopAuthorize(tenantId, record.shop_id, {
          redirect_uri: resolveShopCallbackUrl(record.shop_id)
        });
        window.open(response.authorize_url, '_blank', 'noopener,noreferrer');
      } catch (authorizeError) {
        message.error(authorizeError instanceof Error ? authorizeError.message : '发起授权失败');
      }
    },
    [resolveShopCallbackUrl, tenantId]
  );

  const handleCreateShop = useCallback(() => {
    const preferredPlatform = selectedConfig?.platform;
    const routePlatform = isPlatformCode(preferredPlatform)
      ? preferredPlatform
      : columnFilters.platform;
    navigate(buildCreateChannelRoute(routePlatform));
  }, [columnFilters.platform, navigate, selectedConfig?.platform]);

  const handleSearch = useCallback((value: string) => {
    setSearchValue(value);
    setSearchKeyword(value.trim());
    setPagination((prev) => ({ ...prev, current: 1 }));
  }, []);

  const updateColumnFilters = useCallback((patch: Partial<ColumnFilters>) => {
    setColumnFilters((prev) => ({ ...prev, ...patch }));
    setPagination((prev) => ({ ...prev, current: 1 }));
  }, []);

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
                onClick={() => void handleRunSync(record)}
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
                  onClick={() => void handleAuthorize(record)}
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
      handleAuthorize,
      handleRunSync,
      openConfig,
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
        onClick={handleCreateShop}
      >
        新增网店
      </Button>
      <Button
        icon={<DownloadOutlined />}
        disabled={!canWrite}
        loading={batchDownloading}
        onClick={() => void handleBatchDownload()}
      >
        批量下载订单
      </Button>
      <Button
        icon={<SettingOutlined />}
        disabled={!canWrite}
        onClick={() => {
          if (!selectedConfig) {
            message.warning('请先在表格中选择一个网店。');
            return;
          }
          openConfig(selectedConfig);
        }}
      >
        修改网店配置
      </Button>
      <Button
        icon={<FileSearchOutlined />}
        onClick={() => navigate('/channels/sync-jobs')}
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
      style={{ width: 320 }}
    />
  );

  const toolbarRight = (
    <Space size={8}>
      <Button
        icon={<QuestionCircleOutlined />}
        onClick={() =>
          message.info('右侧面板提供授权与下载入口，日常建议使用增量下载或平台推送。')
        }
      >
        帮助
      </Button>
    </Space>
  );

  const dataWarnings = useMemo(
    () =>
      buildSalesShopPageWarnings({
        shopsError: shopsQuery.error,
        syncJobsError: syncJobsQuery.error
      }),
    [shopsQuery.error, syncJobsQuery.error]
  );

  const headerMessages = [
    ...(disabledFlows.length ? [`以下流程已关闭：${disabledFlows.join('、')}`] : []),
    ...dataWarnings
  ];

  const header = headerMessages.length ? (
    <Alert
      type="warning"
      showIcon
      message={headerMessages[0]}
      description={headerMessages.length > 1 ? headerMessages.slice(1).join(' ') : undefined}
    />
  ) : null;

  const shouldWaitForFallbackData =
    !configItems.length && !error && (shopsQuery.isLoading || shopsQuery.isFetching);

  const mergedLoading = isLoading || isFetching || shouldWaitForFallbackData;
  const mergedError = resolveSalesShopPageError({
    hasRows: baseItems.length > 0,
    configError: error,
    shopsError: shopsQuery.error,
    syncJobsError: syncJobsQuery.error
  });

  const handleRetry = () => {
    void refetch();
    void shopsQuery.refetch();
    void syncJobsQuery.refetch();
  };

  const rightPanel = useMemo(() => {
    if (!selectedConfig) {
      return (
        <div data-testid="shop-access-right-panel-empty">
          <Card size="small" title="店铺授权与订购中心">
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description="请在左侧列表选择一个网店后继续。"
            />
          </Card>
          <Card size="small" title="工业化规范" style={{ marginTop: 12 }}>
            <Typography.Paragraph type="secondary" style={{ marginBottom: 8 }}>
              系统已按平台适配接口差异、订单数据差异和售后差异，下载链路统一进入标准任务中心。
            </Typography.Paragraph>
            <Typography.Paragraph type="secondary" style={{ marginBottom: 8 }}>
              日常下载建议使用增量下载，平台支持时可通过推送方式驱动。
            </Typography.Paragraph>
            <Typography.Paragraph type="secondary" style={{ marginBottom: 0 }}>
              未发货退款订单在 WMS 出库前会自动触发取消与释放。
            </Typography.Paragraph>
          </Card>
        </div>
      );
    }

    const decision = resolveDownloadDecision(selectedConfig);
    const tokenStatus = selectedShop?.token_status;

    return (
      <div data-testid="shop-access-right-panel">
        <Card size="small" title="店铺授权">
          <Descriptions size="small" column={1}>
            <Descriptions.Item label="店铺">
              {selectedConfig.name || selectedConfig.shop_id}
            </Descriptions.Item>
            <Descriptions.Item label="平台">
              <PlatformTag platform={selectedConfig.platform} />
            </Descriptions.Item>
            <Descriptions.Item label="授权状态">
              {selectedAuthStatus ? (
                <Tag color={selectedAuthStatus.color}>{selectedAuthStatus.label}</Tag>
              ) : (
                '-'
              )}
            </Descriptions.Item>
            <Descriptions.Item label="Token 到期">
              {tokenStatus?.expires_at ? formatDateTime(tokenStatus.expires_at) : '-'}
            </Descriptions.Item>
            <Descriptions.Item label="授权范围">
              {tokenStatus?.scopes || '-'}
            </Descriptions.Item>
            <Descriptions.Item label="回调地址">
              <Typography.Text copyable={{ text: selectedCallbackUrl }} style={{ wordBreak: 'break-all' }}>
                {selectedCallbackUrl}
              </Typography.Text>
            </Descriptions.Item>
          </Descriptions>
          <Space style={{ marginTop: 12 }} wrap>
            <Button
              type="primary"
              icon={<KeyOutlined />}
              disabled={!canWrite}
              onClick={() => void handleAuthorize(selectedConfig)}
            >
              发起授权
            </Button>
            <Button
              loading={refreshingTokenShopId === selectedConfig.shop_id}
              disabled={!canWrite}
              onClick={() => void handleRefreshToken(selectedConfig)}
            >
              刷新 Token
            </Button>
          </Space>
        </Card>

        <Card size="small" title="订单下载" style={{ marginTop: 12 }}>
          <Descriptions size="small" column={1}>
            <Descriptions.Item label="上次下载时间">
              {formatDateTime(selectedSyncJob?.last_time)}
            </Descriptions.Item>
            <Descriptions.Item label="下载状态">
              {(() => {
                const status = resolveJobStatus(selectedSyncJob);
                return <Tag color={status.color}>{status.label}</Tag>;
              })()}
            </Descriptions.Item>
            <Descriptions.Item label="策略说明">
              <Typography.Text type="secondary">
                日常采用增量下载；平台支持时可切换为推送；手动支持按订单号和按日期下载。
              </Typography.Text>
            </Descriptions.Item>
          </Descriptions>

          {!decision.allowed ? (
            <Alert
              type="warning"
              showIcon
              style={{ marginTop: 12 }}
              message={decision.reason || '当前店铺不可下载。'}
            />
          ) : null}

          <Space style={{ marginTop: 12 }} wrap>
            <Button
              type="primary"
              icon={<DownloadOutlined />}
              disabled={!canWrite || !decision.allowed}
              loading={syncingId === selectedConfig.shop_id}
              onClick={() => void handleRunSync(selectedConfig)}
            >
              立即增量下载
            </Button>
            <Button disabled={!canWrite} onClick={() => openManualDownload(selectedConfig)}>
              手动下载
            </Button>
          </Space>
        </Card>

        <Card size="small" title="工业化规范" style={{ marginTop: 12 }}>
          <Space direction="vertical" size={6}>
            <Typography.Text type="secondary">
              1. 统一抽象多平台接口差异（鉴权、分页、限流、字段映射、售后状态机）。
            </Typography.Text>
            <Typography.Text type="secondary">
              2. 下载任务统一写入任务中心，支持增量水位、按号重放与按日期补采。
            </Typography.Text>
            <Typography.Text type="secondary">
              3. 未发货退款订单会在 WMS 出库前自动取消订单，避免误发。
            </Typography.Text>
          </Space>
        </Card>
      </div>
    );
  }, [
    canWrite,
    handleAuthorize,
    handleRefreshToken,
    handleRunSync,
    openManualDownload,
    refreshingTokenShopId,
    resolveDownloadDecision,
    selectedAuthStatus,
    selectedCallbackUrl,
    selectedConfig,
    selectedShop,
    selectedSyncJob,
    syncingId
  ]);

  useEffect(() => {
    setLeftPanel(undefined);
    setRightPanel(rightPanel);
    setBottomPanel(undefined);
    setLeftPanelVisible(false);
    setRightPanelVisible(true);
    setBottomPanelVisible(false);
    setRightPanelWidth(420);
    return () => clearPanels();
  }, [
    clearPanels,
    rightPanel,
    setBottomPanel,
    setBottomPanelVisible,
    setLeftPanel,
    setLeftPanelVisible,
    setRightPanel,
    setRightPanelVisible,
    setRightPanelWidth
  ]);

  return (
    <>
      <UiModule03MainTableSection<SalesShopConfig>
        dataTestId="sales-shop-config-grid"
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
          onChange: (keys) => {
            const normalized = keys.map((key) => String(key));
            setSelectedRowKeys(normalized);
            setActiveShopId((previous) => {
              if (!normalized.length) {
                return null;
              }
              if (previous && normalized.includes(previous)) {
                return previous;
              }
              return normalized[0];
            });
          }
        }}
        onRow={(record) => ({
          onClick: () => setActiveShopId(record.shop_id)
        })}
        queryCells={queryCells}
      />

      <Drawer
        title={editingConfig ? `配置 - ${editingConfig.name}` : '网店配置'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={420}
        destroyOnHidden
        footer={
          <Space style={{ display: 'flex', justifyContent: 'flex-end' }}>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={() => void handleSave()}>
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

      <Modal
        title={
          manualDownloadTarget
            ? `手动下载 - ${manualDownloadTarget.name || manualDownloadTarget.shop_id}`
            : '手动下载'
        }
        open={manualDownloadOpen}
        onCancel={() => setManualDownloadOpen(false)}
        destroyOnHidden
        width={760}
        okText="提交下载"
        cancelText="取消"
        confirmLoading={manualDownloadSubmitting}
        onOk={() => void handleManualDownload()}
      >
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          <Alert
            type="info"
            showIcon
            message="日常建议使用增量下载或平台推送；手动下载用于补采与对账。"
          />

          <Form
            form={manualDownloadForm}
            layout="vertical"
            initialValues={{ mode: 'incremental' as ManualDownloadMode }}
          >
            <Form.Item name="mode" label="下载模式" rules={[{ required: true }]}>
              <Radio.Group
                options={manualDownloadModeOptions}
                optionType="button"
                buttonStyle="solid"
              />
            </Form.Item>

            {manualDownloadMode === 'order-nos' ? (
              <Form.Item
                name="order_nos_text"
                label="订单号"
                extra="支持换行、英文逗号、中文逗号、空格分隔。"
              >
                <Input.TextArea rows={5} placeholder="示例：PDD20260413001,PDD20260413002" />
              </Form.Item>
            ) : null}

            {manualDownloadMode === 'time-range' ? (
              <Form.Item
                name="time_range"
                label="下载日期范围"
                rules={[{ required: true, message: '请选择日期范围' }]}
              >
                <DatePicker.RangePicker
                  style={{ width: '100%' }}
                  showTime={{ format: 'HH:mm' }}
                  format="YYYY-MM-DD HH:mm"
                />
              </Form.Item>
            ) : null}

            <Form.Item
              name="cursor"
              label="游标（可选）"
              extra="用于平台侧的断点重试或分页续传。"
            >
              <Input placeholder="输入平台游标（可选）" />
            </Form.Item>

            {manualDownloadMode === 'incremental' ? (
              <Form.Item
                name="watermark"
                label="增量水位（可选）"
                extra="例如平台更新时间戳或业务水位键。"
              >
                <Input placeholder="输入增量水位（可选）" />
              </Form.Item>
            ) : null}
          </Form>

          {manualDownloadResult ? (
            <Card
              size="small"
              title={`订单号下载结果（成功 ${manualDownloadResult.success_count} / 失败 ${manualDownloadResult.failure_count}）`}
            >
              <Space direction="vertical" size={8} style={{ width: '100%' }}>
                {manualDownloadResult.items.length ? (
                  manualDownloadResult.items.map((item) => (
                    <div
                      key={`${item.platform_order_id}-${item.order_id ?? 'none'}`}
                      style={{
                        display: 'flex',
                        justifyContent: 'space-between',
                        gap: 8,
                        borderBottom: '1px solid var(--ant-color-border-secondary)',
                        paddingBottom: 6
                      }}
                    >
                      <Typography.Text>{item.platform_order_id}</Typography.Text>
                      <Space size={8}>
                        <Tag color={item.success ? 'green' : 'red'}>
                          {item.success ? '成功' : '失败'}
                        </Tag>
                        <Typography.Text type="secondary">
                          {item.message || item.order_id || '-'}
                        </Typography.Text>
                      </Space>
                    </div>
                  ))
                ) : (
                  <Typography.Text type="secondary">无返回明细。</Typography.Text>
                )}
              </Space>
            </Card>
          ) : null}
        </Space>
      </Modal>
    </>
  );
};
