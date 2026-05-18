import { Alert, Button, Form, Input, Select, Space, Switch, Typography, message } from 'antd';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import { listPrintTemplates } from '@/api/printTemplates';
import { listSettings, upsertSetting, type Setting } from '@/api/system';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState } from '@/components/QueryState';
import { SettingsPageTemplate, SettingsPageSection } from '@/components/page-templates';
import { useAutoMergeSettingsStore } from '@/features/autoMergeSettings';
import { isFeatureEnabled, updateInventoryEnabled, useFeatureFlag } from '@/features/featureFlags';
import { useFastModeSettingsStore } from '@/features/fastModeSettings';
import { useMenuCacheSettingsStore } from '@/features/menuCacheSettings';
import { useOrderAuditSettingsStore } from '@/features/orderAuditSettings';
import type { PrintBridgeInfo, PrintBridgePrinter } from '@/features/printing/PrintBridge';
import { printBridge } from '@/features/printing/PrintBridge';
import {
  buildTemplateKey,
  builtinPrintTemplateOptions,
  getStoredPrintTemplate,
  mapPrintTemplateToOption,
  parseTemplateKey,
  setStoredPrintTemplate
} from '@/features/printing/printTemplateSettings';
import { useSessionStore } from '@/features/sessionStore';
import { isTenantSelected, useTenantId } from '@/features/tenant';

import './systemSettingsPage.css';

type StoredPrinter = {
  id: string;
  name: string;
};

const printerStorageKey = 'erp.printer.default';
const forceReviewPasswordKey = 'orders.force_review_password';

const loadStoredPrinter = (): StoredPrinter | null => {
  if (typeof window === 'undefined') {
    return null;
  }
  const raw = window.localStorage.getItem(printerStorageKey);
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw) as StoredPrinter;
    if (parsed && parsed.id && parsed.name) {
      return parsed;
    }
  } catch {
    return null;
  }
  return null;
};

const saveStoredPrinter = (printer: StoredPrinter | null) => {
  if (typeof window === 'undefined') {
    return;
  }
  if (!printer) {
    window.localStorage.removeItem(printerStorageKey);
    return;
  }
  window.localStorage.setItem(printerStorageKey, JSON.stringify(printer));
};

export const SystemSettingsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const featureFlags = useSessionStore((state) => state.featureFlags);
  const setFeatureFlags = useSessionStore((state) => state.setFeatureFlags);
  const inventoryEnabled = isFeatureEnabled(featureFlags, 'inventory.enabled');
  const [printers, setPrinters] = useState<PrintBridgePrinter[]>([]);
  const [printerLoading, setPrinterLoading] = useState(false);
  const [printerError, setPrinterError] = useState<string | null>(null);
  const [savedPrinter, setSavedPrinter] = useState<StoredPrinter | null>(() => loadStoredPrinter());
  const [selectedPrinterId, setSelectedPrinterId] = useState<string | undefined>(
    () => loadStoredPrinter()?.id
  );
  const [detecting, setDetecting] = useState(false);
  const [detectInfo, setDetectInfo] = useState<PrintBridgeInfo | null>(null);
  const [detectError, setDetectError] = useState<string | null>(null);
  const [savedTemplate, setSavedTemplate] = useState(() => getStoredPrintTemplate());
  const [selectedTemplateKey, setSelectedTemplateKey] = useState<string | undefined>(() => {
    const stored = getStoredPrintTemplate();
    return stored ? buildTemplateKey(stored.template_id) : undefined;
  });
  const [inventoryUpdating, setInventoryUpdating] = useState(false);
  const autoMergeEnabled = useAutoMergeSettingsStore((state) => state.enabled);
  const setAutoMergeEnabled = useAutoMergeSettingsStore((state) => state.setEnabled);
  const fastModeEnabled = useFastModeSettingsStore((state) => state.enabled);
  const setFastModeEnabled = useFastModeSettingsStore((state) => state.setEnabled);
  const menuCacheEnabled = useMenuCacheSettingsStore((state) => state.enabled);
  const setMenuCacheEnabled = useMenuCacheSettingsStore((state) => state.setEnabled);
  const auditEnabled = useOrderAuditSettingsStore((state) => state.enabled);
  const auditMode = useOrderAuditSettingsStore((state) => state.mode);
  const setAuditEnabled = useOrderAuditSettingsStore((state) => state.setEnabled);
  const setAuditMode = useOrderAuditSettingsStore((state) => state.setMode);
  const mergeFlowFlag = useFeatureFlag('flows.merge_outside_ship_time');
  const [forceReviewPassword, setForceReviewPassword] = useState('');
  const [forceReviewSaving, setForceReviewSaving] = useState(false);
  const [forceReviewSetting, setForceReviewSetting] = useState<Setting | null>(null);

  const templateQuery = useQuery({
    queryKey: ['print-templates', tenantId, 'settings'],
    queryFn: () =>
      listPrintTemplates(tenantId, {
        template_type: 'SHIPPING_LABEL',
        offset: 0,
        limit: 200
      }),
    enabled: tenantReady,
    retry: false
  });

  const forceReviewSettingQuery = useQuery({
    queryKey: ['settings', tenantId, forceReviewPasswordKey],
    queryFn: () =>
      listSettings(tenantId, {
        scope_type: 'TENANT',
        key: forceReviewPasswordKey
      }),
    enabled: tenantReady
  });

  const templateOptions = useMemo(() => {
    const cloudOptions = templateQuery.data?.items.map(mapPrintTemplateToOption) ?? [];
    const merged = [...cloudOptions, ...builtinPrintTemplateOptions];
    const seen = new Set<string>();
    return merged.filter((item) => {
      const key = buildTemplateKey(item.template_id);
      if (seen.has(key)) {
        return false;
      }
      seen.add(key);
      return true;
    });
  }, [templateQuery.data?.items]);

  const templateSelectOptions = useMemo(() => {
    const cloud = templateOptions.filter((item) => item.source === 'cloud');
    const builtin = templateOptions.filter((item) => item.source === 'builtin');
    return [
      {
        label: '云端模板',
        options: cloud.map((item) => ({
          label: item.version ? `${item.name} (v${item.version})` : item.name,
          value: buildTemplateKey(item.template_id)
        }))
      },
      {
        label: '内置模板',
        options: builtin.map((item) => ({
          label: item.name,
          value: buildTemplateKey(item.template_id)
        }))
      }
    ];
  }, [templateOptions]);

  const auditModeOptions = useMemo(
    () => [
      { label: '自动审核', value: 'auto' },
      { label: '手动审核', value: 'manual' }
    ],
    []
  );

  const savedTemplateKey = savedTemplate ? buildTemplateKey(savedTemplate.template_id) : undefined;
  const savedTemplateMissing =
    savedTemplateKey &&
    !templateOptions.some(
      (option) => buildTemplateKey(option.template_id) === savedTemplateKey
    );

  const selectedTemplate = templateOptions.find(
    (option) => buildTemplateKey(option.template_id) === selectedTemplateKey
  );

  const loadPrinters = useCallback(async () => {
    setPrinterLoading(true);
    setPrinterError(null);
    if (!printBridge.getPrinters) {
      setPrinters([]);
      setPrinterError('打印组件未提供打印机列表');
      setPrinterLoading(false);
      return;
    }
    const result = await printBridge.getPrinters();
    if (result.success) {
      setPrinters(result.data ?? []);
    } else {
      setPrinters([]);
      setPrinterError(result.error || '获取打印机列表失败');
    }
    setPrinterLoading(false);
  }, []);

  useEffect(() => {
    void loadPrinters();
  }, [loadPrinters]);

  useEffect(() => {
    if (!forceReviewSettingQuery.data) {
      return;
    }
    setForceReviewSetting(forceReviewSettingQuery.data[0] ?? null);
  }, [forceReviewSettingQuery.data]);

  useEffect(() => {
    if (!printers.length) {
      return;
    }
    if (selectedPrinterId && !printers.some((printer) => printer.id === selectedPrinterId)) {
      setSelectedPrinterId(undefined);
      return;
    }
    if (!selectedPrinterId) {
      const fallback =
        printers.find((printer) => printer.is_default)?.id ?? printers[0]?.id;
      setSelectedPrinterId(fallback);
    }
  }, [printers, selectedPrinterId]);

  useEffect(() => {
    if (!templateOptions.length) {
      return;
    }
    if (selectedTemplateKey) {
      return;
    }
    const fallback = templateOptions[0];
    if (fallback) {
      setSelectedTemplateKey(buildTemplateKey(fallback.template_id));
    }
  }, [selectedTemplateKey, templateOptions]);

  const printerOptions = useMemo(
    () =>
      printers.map((printer) => ({
        label: printer.name,
        value: printer.id
      })),
    [printers]
  );

  const savedPrinterMissing =
    savedPrinter && !printers.some((printer) => printer.id === savedPrinter.id);

  const handleSaveDefault = () => {
    if (!selectedPrinterId) {
      message.error('请选择默认打印机');
      return;
    }
    const target = printers.find((printer) => printer.id === selectedPrinterId);
    if (!target) {
      message.error('请选择可用的打印机');
      return;
    }
    const stored: StoredPrinter = { id: target.id, name: target.name };
    saveStoredPrinter(stored);
    setSavedPrinter(stored);
    message.success('默认打印机已保存');
  };

  const handleInventoryToggle = async (enabled: boolean) => {
    if (!tenantReady) {
      message.error('请先选择商家');
      return;
    }
    setInventoryUpdating(true);
    try {
      const result = await updateInventoryEnabled(tenantId, enabled);
      const nextState = result.inventory_enabled ? 'enabled' : 'disabled';
      setFeatureFlags({ ...featureFlags, 'inventory.enabled': nextState });
      message.success(enabled ? '库存管理已启用' : '库存管理已关闭');
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新库存开关失败');
    } finally {
      setInventoryUpdating(false);
    }
  };

  const handleSaveForceReviewPassword = async () => {
    if (!tenantReady) {
      message.error('请先选择商户');
      return;
    }
    const password = forceReviewPassword.trim();
    if (!password) {
      message.warning('请输入强制审核密码');
      return;
    }
    setForceReviewSaving(true);
    try {
      const setting = await upsertSetting(tenantId, {
        scope_type: 'TENANT',
        scope_id: null,
        key: forceReviewPasswordKey,
        value: { value: password },
        expected_version: forceReviewSetting?.version ?? undefined
      });
      setForceReviewSetting(setting);
      setForceReviewPassword('');
      message.success('强制审核密码已更新');
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新强制审核密码失败');
    } finally {
      setForceReviewSaving(false);
    }
  };

  const handleSaveTemplate = () => {
    const parsed = parseTemplateKey(selectedTemplateKey);
    if (!parsed) {
      message.error('请选择默认模板');
      return;
    }
    setStoredPrintTemplate(parsed);
    setSavedTemplate(parsed);
    message.success('默认模板已保存');
  };

  const handleDetect = async () => {
    setDetecting(true);
    setDetectInfo(null);
    setDetectError(null);
    const result = await printBridge.detect();
    if (result.success) {
      setDetectInfo(result.data ?? { provider: 'unknown' });
      message.success('打印组件可用');
    } else {
      setDetectError(result.error || '打印组件不可用');
      message.error(result.error || '打印组件不可用');
    }
    setDetecting(false);
  };

  return (
    <>
      <SettingsPageTemplate title="系统设置">
        <div className="system-settings-wrapper">
          <div className="system-settings-page">
            <Space direction="vertical" size={12} style={{ width: '100%' }}>
                <div id="system-settings-inventory" className="system-settings-section">
                  <SettingsPageSection
                    title="库存管理"
                    description="控制库存模块的启用状态"
                    className="system-settings-section-card"
                  >
                    <Space align="center" size={12}>
                      <Switch
                        checked={inventoryEnabled}
                        loading={inventoryUpdating}
                        onChange={handleInventoryToggle}
                      />
                      <Typography.Text>
                        {inventoryEnabled ? '已启用库存管理' : '已关闭库存管理'}
                      </Typography.Text>
                    </Space>
                    <Typography.Text type="secondary">
                      关闭后库存相关入口与功能将隐藏，仅保留基础订单功能。
                    </Typography.Text>
                  </SettingsPageSection>
                </div>

                <div id="system-settings-audit" className="system-settings-section">
                  <SettingsPageSection
                    title="订单审核"
                    description="审核模式与强制密码配置"
                    className="system-settings-section-card"
                  >
                    <Form layout="vertical">
                      <Form.Item label="审核开关">
                        <Switch
                          checked={auditEnabled}
                          onChange={(checked) => setAuditEnabled(checked)}
                          checkedChildren="开启"
                          unCheckedChildren="关闭"
                        />
                      </Form.Item>
                      <Form.Item label="审核模式">
                        <Select
                          options={auditModeOptions}
                          value={auditMode}
                          onChange={(value) => setAuditMode(value)}
                          disabled={!auditEnabled}
                          placeholder="请选择审核模式"
                        />
                      </Form.Item>
                      <Form.Item label="强制审核密码">
                        <Space direction="vertical" size={8} style={{ width: '100%' }}>
                          <Input.Password
                            value={forceReviewPassword}
                            onChange={(event) => setForceReviewPassword(event.target.value)}
                            placeholder={
                              forceReviewSetting ? '已设置密码，更新将覆盖' : '请输入新的密码'
                            }
                          />
                          <Button
                            type="primary"
                            onClick={handleSaveForceReviewPassword}
                            loading={forceReviewSaving}
                          >
                            保存密码
                          </Button>
                          <Typography.Text type="secondary">
                            {forceReviewSetting
                              ? `最近更新：${new Date(
                                  forceReviewSetting.updated_at
                                ).toLocaleString()}`
                              : '尚未设置强制审核密码'}
                          </Typography.Text>
                        </Space>
                      </Form.Item>
                      <Typography.Text type="secondary">
                        V1 仅影响订单审核流程，不影响售后与仓储流程。
                      </Typography.Text>
                    </Form>
                  </SettingsPageSection>
                </div>

                <div id="system-settings-fast" className="system-settings-section">
                  <SettingsPageSection
                    title="极速模式"
                    description="减少动画与非必要交互，提升响应速度"
                    className="system-settings-section-card"
                  >
                    <Space align="center" size={12}>
                      <Switch
                        checked={fastModeEnabled}
                        onChange={(checked) => setFastModeEnabled(checked)}
                        checkedChildren="开启"
                        unCheckedChildren="关闭"
                      />
                      <Typography.Text>
                        {fastModeEnabled ? '已启用极速模式' : '已关闭极速模式'}
                      </Typography.Text>
                    </Space>
                    <Typography.Text type="secondary">
                      极速模式属于 V1 试运行能力，启用后可显著减少动效。
                    </Typography.Text>
                  </SettingsPageSection>
                </div>

                <div id="system-settings-performance" className="system-settings-section">
                  <SettingsPageSection
                    title="界面性能"
                    description="控制顶部菜单缓存策略"
                    className="system-settings-section-card"
                  >
                    <Space align="center" size={12}>
                      <Switch
                        checked={menuCacheEnabled}
                        onChange={(checked) => setMenuCacheEnabled(checked)}
                        checkedChildren="开启"
                        unCheckedChildren="关闭"
                      />
                      <Typography.Text>
                        {menuCacheEnabled ? '已开启菜单缓存' : '已关闭菜单缓存'}
                      </Typography.Text>
                    </Space>
                    <Typography.Text type="secondary">
                      启用后会缓存菜单数据，降低登录后加载时间。
                    </Typography.Text>
                  </SettingsPageSection>
                </div>

                <div id="system-settings-merge" className="system-settings-section">
                  <SettingsPageSection
                    title="自动合并"
                    description="发货前自动合并订单"
                    className="system-settings-section-card"
                  >
                    <Space align="center" size={12}>
                      <Switch
                        checked={autoMergeEnabled}
                        onChange={(checked) => setAutoMergeEnabled(checked)}
                        disabled={!mergeFlowFlag.enabled}
                        checkedChildren="开启"
                        unCheckedChildren="关闭"
                      />
                      <Typography.Text>
                        {autoMergeEnabled ? '已启用自动合并' : '已关闭自动合并'}
                      </Typography.Text>
                    </Space>
                    <Typography.Text type="secondary">
                      {!mergeFlowFlag.enabled
                        ? mergeFlowFlag.reason ?? '当前租户暂未开放该能力'
                        : 'V2 流程下可提升发货效率。'}
                    </Typography.Text>
                  </SettingsPageSection>
                </div>

                <div id="system-settings-printer" className="system-settings-section">
                  <SettingsPageSection
                    title="打印机设置"
                    description="检测打印组件并设置默认打印机"
                    className="system-settings-section-card"
                  >
                    <Space wrap>
                      <Button onClick={handleDetect} loading={detecting}>
                        检测组件
                      </Button>
                      <Button onClick={loadPrinters} loading={printerLoading}>
                        刷新打印机
                      </Button>
                    </Space>
                    {detectInfo ? (
                      <Alert
                        type="success"
                        showIcon
                        message="打印组件可用"
                        description={`Provider: ${detectInfo.provider}${
                          detectInfo.version ? ` (${detectInfo.version})` : ''
                        }`}
                      />
                    ) : null}
                    {detectError ? (
                      <ErrorState
                        title="打印组件不可用"
                        description={detectError}
                        hint="请确认本地打印组件已启动并完成授权。"
                        action={
                          <Button size="small" onClick={handleDetect} loading={detecting}>
                            重新检测
                          </Button>
                        }
                      />
                    ) : null}
                    {printerError ? (
                      <ErrorState
                        title="获取打印机失败"
                        description={printerError}
                        action={
                          <Button size="small" onClick={loadPrinters} loading={printerLoading}>
                            重新加载
                          </Button>
                        }
                      />
                    ) : null}
                    {savedPrinterMissing ? (
                      <Alert
                        type="warning"
                        showIcon
                        message="默认打印机不可用"
                        description={`默认打印机 ${savedPrinter?.name} 已不可用，请重新选择。`}
                      />
                    ) : null}
                    {!printerError && printers.length === 0 ? (
                      <EmptyState
                        description="未检测到可用打印机，请确认打印组件已启动。"
                        action={
                          <Button size="small" onClick={loadPrinters} loading={printerLoading}>
                            重新加载
                          </Button>
                        }
                      />
                    ) : null}
                    <Form layout="vertical">
                      <Form.Item label="默认打印机">
                        <Select
                          placeholder="请选择默认打印机"
                          options={printerOptions}
                          value={selectedPrinterId}
                          onChange={(value) => setSelectedPrinterId(value)}
                          loading={printerLoading}
                          disabled={printers.length === 0}
                        />
                      </Form.Item>
                      <Space direction="vertical" size={8}>
                        <Button
                          type="primary"
                          onClick={handleSaveDefault}
                          disabled={!selectedPrinterId || printers.length === 0}
                        >
                          保存默认打印机
                        </Button>
                        <Typography.Text type="secondary">
                          {savedPrinter ? `已保存：${savedPrinter.name}` : '尚未设置默认打印机'}
                        </Typography.Text>
                      </Space>
                    </Form>
                  </SettingsPageSection>
                </div>

                <div id="system-settings-template" className="system-settings-section">
                  <SettingsPageSection
                    title="打印模板"
                    description="设置默认面单模板"
                    className="system-settings-section-card"
                  >
                    {templateQuery.error ? (
                      <ErrorState
                        title="获取模板失败"
                        error={templateQuery.error}
                        onRetry={() => templateQuery.refetch()}
                      />
                    ) : null}
                    {savedTemplateMissing ? (
                      <Alert
                        type="warning"
                        showIcon
                        message="模板已失效"
                        description="已保存的模板不存在，请重新选择。"
                      />
                    ) : null}
                    <Form layout="vertical">
                      <Form.Item label="默认模板">
                        <Select
                          placeholder="请选择默认模板"
                          options={templateSelectOptions}
                          value={selectedTemplateKey}
                          onChange={(value) => setSelectedTemplateKey(value)}
                          loading={templateQuery.isFetching}
                          disabled={!templateOptions.length}
                        />
                      </Form.Item>
                      <Space direction="vertical" size={8}>
                        <Button
                          type="primary"
                          onClick={handleSaveTemplate}
                          disabled={!selectedTemplateKey}
                        >
                          保存模板
                        </Button>
                        <Typography.Text type="secondary">
                          {selectedTemplate
                            ? `当前选择：${selectedTemplate.name}${
                                selectedTemplate.version ? ` (v${selectedTemplate.version})` : ''
                              }`
                            : savedTemplateKey
                              ? `已保存模板：${savedTemplateKey}`
                              : '尚未设置默认模板'}
                        </Typography.Text>
                      </Space>
                    </Form>
                  </SettingsPageSection>
                </div>
            </Space>
          </div>
        </div>
      </SettingsPageTemplate>
    </>
  );
};
