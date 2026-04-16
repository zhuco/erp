import {
  Affix,
  Button,
  Card,
  Descriptions,
  Drawer,
  Form,
  Input,
  Modal,
  Space,
  Switch,
  Tag,
  Typography,
  message
} from 'antd';
import type { InputRef } from 'antd';
import { DownOutlined, UpOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { createProduct } from '@/api/products';
import type { Sku } from '@/api/skus';
import { createSku, listSkus, updateSku } from '@/api/skus';
import { EmptyState } from '@/components/EmptyState';
import { WorkbenchMainTableTemplate } from '@/components/workbench/templates';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

import { resolveSkuScan } from './skuScanRules';

type FeedbackState = {
  status: 'success' | 'error';
  message: string;
};

type QuickCreateState = {
  open: boolean;
  field: 'barcode' | 'sku_code';
  value: string;
};

type SkuDetailFormValues = {
  sku_code?: string;
  barcode?: string;
  name?: string;
  is_active?: boolean;
};

type QuickCreateFormValues = {
  sku_code?: string;
  barcode?: string;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatCost = (costCent?: number | null) =>
  costCent == null ? '-' : `¥${(costCent / 100).toFixed(2)}`;

const playTone = (frequency: number, duration = 140) => {
  if (typeof window === 'undefined') {
    return;
  }
  try {
    const AudioContextConstructor =
      window.AudioContext || (window as typeof window & { webkitAudioContext?: AudioContext })
        .webkitAudioContext;
    if (!AudioContextConstructor) {
      return;
    }
    const context = new AudioContextConstructor();
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    oscillator.type = 'sine';
    oscillator.frequency.value = frequency;
    gain.gain.value = 0.08;
    oscillator.connect(gain);
    gain.connect(context.destination);
    oscillator.start();
    oscillator.stop(context.currentTime + duration / 1000);
    oscillator.onended = () => {
      context.close();
    };
  } catch {
    return;
  }
};

const playSuccessTone = () => playTone(920, 120);
const playErrorTone = () => playTone(240, 200);

type SkuDetailDrawerProps = {
  open: boolean;
  sku: Sku | null;
  onClose: () => void;
  onSave: (values: SkuDetailFormValues) => Promise<void> | void;
  saving: boolean;
  canEdit: boolean;
};

const SkuDetailDrawer = ({
  open,
  sku,
  onClose,
  onSave,
  saving,
  canEdit
}: SkuDetailDrawerProps) => {
  const [form] = Form.useForm<SkuDetailFormValues>();

  useEffect(() => {
    if (!sku) {
      form.resetFields();
      return;
    }
    form.setFieldsValue({
      sku_code: sku.sku_code,
      barcode: sku.barcode,
      name: sku.name,
      is_active: sku.is_active
    });
  }, [form, sku]);

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      await onSave(values);
    } catch (error) {
      if (
        typeof error === 'object' &&
        error &&
        'errorFields' in error
      ) {
        return;
      }
      message.error(error instanceof Error ? error.message : '保存失败');
    }
  };

  return (
    <Drawer
      title="SKU 详情 / 编辑"
      open={open}
      width={520}
      onClose={onClose}
      destroyOnClose
      data-testid="sku-detail-drawer"
      extra={
        <Button
          onClick={handleSave}
          type="primary"
          loading={saving}
          disabled={!sku || !canEdit}
        >
          保存修改
        </Button>
      }
    >
      {sku ? (
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions size="small" column={1}>
              <Descriptions.Item label="SKU ID">{sku.sku_id}</Descriptions.Item>
              <Descriptions.Item label="创建时间">
                {formatDateTime(sku.created_at)}
              </Descriptions.Item>
              <Descriptions.Item label="更新时间">
                {formatDateTime(sku.updated_at)}
              </Descriptions.Item>
              <Descriptions.Item label="成本">
                {formatCost(sku.cost_cent ?? null)}
              </Descriptions.Item>
            </Descriptions>
            <Form form={form} layout="vertical">
              <Form.Item label="SKU 编码" name="sku_code">
                <Input disabled />
              </Form.Item>
              <Form.Item label="条码" name="barcode">
                <Input disabled={!canEdit} />
              </Form.Item>
              <Form.Item label="名称" name="name">
                <Input disabled={!canEdit} />
              </Form.Item>
              <Form.Item label="启用" name="is_active" valuePropName="checked">
                <Switch disabled={!canEdit} />
              </Form.Item>
            </Form>
          </Space>
        ) : (
          <EmptyState description="暂无 SKU 数据" />
        )}
      </Drawer>
  );
};

export const SkuPage = () => {
  const tenantId = useTenantId();
  const canWriteSkus = useCan('skus.write');
  const inputRef = useRef<InputRef>(null);
  const lastScanRef = useRef<{ value: string; at: number } | null>(null);
  const [scanValue, setScanValue] = useState('');
  const [collapsed, setCollapsed] = useState(false);
  const [isScanning, setIsScanning] = useState(false);
  const [feedback, setFeedback] = useState<FeedbackState | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [selectedSku, setSelectedSku] = useState<Sku | null>(null);
  const [quickCreate, setQuickCreate] = useState<QuickCreateState | null>(null);
  const [createForm] = Form.useForm<QuickCreateFormValues>();
  const [saving, setSaving] = useState(false);
  const [creating, setCreating] = useState(false);

  const focusInput = useCallback(() => {
    if (collapsed) {
      return;
    }
    requestAnimationFrame(() => {
      inputRef.current?.focus({ cursor: 'end' });
    });
  }, [collapsed]);

  useEffect(() => {
    focusInput();
  }, [focusInput]);

  useEffect(() => {
    if (!feedback) {
      return;
    }
    const timer = window.setTimeout(() => {
      setFeedback(null);
    }, 2000);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  useEffect(() => {
    if (!quickCreate?.open) {
      return;
    }
    createForm.setFieldsValue({
      barcode: quickCreate.field === 'barcode' ? quickCreate.value : undefined,
      sku_code: quickCreate.field === 'sku_code' ? quickCreate.value : undefined
    });
  }, [createForm, quickCreate]);

  const handleSaveSku = async (values: SkuDetailFormValues) => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!selectedSku) {
      message.warning('请先选择 SKU');
      return;
    }
    if (!canWriteSkus) {
      message.warning('暂无 SKU 写入权限');
      return;
    }
    setSaving(true);
    try {
      const payload = {
        barcode: values.barcode?.trim() || null,
        name: values.name?.trim() || null,
        is_active: values.is_active ?? selectedSku.is_active
      };
      const updated = await updateSku(tenantId, selectedSku.sku_id, payload);
      setSelectedSku(updated);
      message.success('SKU 已更新');
    } catch (error) {
      message.error(error instanceof Error ? error.message : 'SKU 更新失败');
    } finally {
      setSaving(false);
    }
  };

  const handleQuickCreate = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!canWriteSkus) {
      message.warning('暂无 SKU 写入权限');
      return;
    }
    let createdSku: Sku | null = null;
    setCreating(true);
    try {
      const values = (await createForm.validateFields()) as QuickCreateFormValues;
      const skuCode = values.sku_code?.trim();
      if (!skuCode) {
        message.warning('请输入 SKU 编码');
        return;
      }
      const barcode = values.barcode?.trim() || null;
      const product = await createProduct(tenantId, { name: skuCode });
      createdSku = await createSku(tenantId, {
        product_id: product.product_id,
        sku_code: skuCode,
        barcode,
        name: skuCode
      });
      message.success('SKU 创建成功');
      setQuickCreate(null);
      createForm.resetFields();
      setSelectedSku(createdSku);
      setDrawerOpen(true);
    } catch (error) {
      if (
        typeof error === 'object' &&
        error &&
        'errorFields' in error
      ) {
        return;
      }
      message.error(error instanceof Error ? error.message : 'SKU 创建失败');
    } finally {
      setCreating(false);
      if (!createdSku) {
        focusInput();
      }
    }
  };

  const handleScan = useCallback(async () => {
    const value = scanValue.trim();
    if (!value) {
      setFeedback({ status: 'error', message: '请输入条码或 SKU 编码' });
      playErrorTone();
      focusInput();
      return;
    }

    if (isScanning) {
      return;
    }

    const now = Date.now();
    const lastScan = lastScanRef.current;
    if (lastScan && lastScan.value === value && now - lastScan.at < 600) {
      return;
    }
    lastScanRef.current = { value, at: now };

    setIsScanning(true);
    try {
      if (!tenantId) {
        throw new Error('请先选择商家');
      }
      const barcodeResult = await listSkus(tenantId, {
        barcode: value,
        limit: 1
      });
      let match = resolveSkuScan(value, barcodeResult.data ?? []);
      if (match.type !== 'barcode') {
        const skuCodeResult = await listSkus(tenantId, {
          sku_code: value,
          limit: 1
        });
        match = resolveSkuScan(value, [...(barcodeResult.data ?? []), ...(skuCodeResult.data ?? [])]);
      }

      if (match.type === 'barcode' || match.type === 'sku_code') {
        setSelectedSku(match.sku);
        setDrawerOpen(true);
        setFeedback({ status: 'success', message: '命中 SKU，已打开详情' });
        message.success('SKU 已识别');
        playSuccessTone();
      } else {
        setQuickCreate({ open: true, field: match.inferredField, value: match.value });
        setFeedback({ status: 'error', message: '未识别 SKU，已进入快速创建' });
        message.warning('未识别 SKU，进入快速创建');
        playErrorTone();
      }
    } catch (error) {
      setFeedback({
        status: 'error',
        message: error instanceof Error ? error.message : 'SKU 查询失败'
      });
      message.error('SKU 查询失败');
      playErrorTone();
    } finally {
      setScanValue('');
      setIsScanning(false);
      focusInput();
    }
  }, [focusInput, isScanning, scanValue, tenantId]);

  const feedbackTag = useMemo(() => {
    if (!feedback) {
      return (
        <Typography.Text type="secondary">
          支持连续扫码，回车提交后自动清空并重新聚焦。
        </Typography.Text>
      );
    }
    return (
      <Tag color={feedback.status === 'success' ? 'green' : 'red'}>{feedback.message}</Tag>
    );
  }, [feedback]);

  return (
    <>
      <WorkbenchMainTableTemplate
        header={<Typography.Title level={3}>商品 SKU</Typography.Title>}
        tableContent={
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Affix offsetTop={0}>
              <Card
                size="small"
                title={
                  <Space>
                    <span>扫码识别框</span>
                    <Tag color="blue">支持条码枪</Tag>
                  </Space>
                }
                extra={
                  <Button type="text" onClick={() => setCollapsed((prev) => !prev)}>
                    {collapsed ? '展开' : '收起'}
                    {collapsed ? <DownOutlined /> : <UpOutlined />}
                  </Button>
                }
                style={{ boxShadow: '0 6px 18px rgba(0,0,0,0.08)' }}
              >
                {collapsed ? null : (
                  <Space direction="vertical" size={8} style={{ width: '100%' }}>
                    <Space.Compact style={{ width: '100%' }}>
                      <Input
                        ref={inputRef}
                        value={scanValue}
                        allowClear
                        autoFocus
                        placeholder="扫描条码或输入 SKU 编码，回车识别"
                        onChange={(event) => setScanValue(event.target.value)}
                        onPressEnter={handleScan}
                        data-testid="sku-scan-input"
                        status={feedback?.status === 'error' ? 'error' : undefined}
                      />
                      <Button
                        type="primary"
                        onClick={handleScan}
                        loading={isScanning}
                        data-testid="sku-scan-submit"
                      >
                        识别
                      </Button>
                    </Space.Compact>
                    {feedbackTag}
                  </Space>
                )}
              </Card>
            </Affix>
            <Card>
              <EmptyState description="SKU 列表建设中" />
            </Card>
          </Space>
        }
      />
      <SkuDetailDrawer
        open={drawerOpen}
        sku={selectedSku}
        onClose={() => {
          setDrawerOpen(false);
          setSelectedSku(null);
          focusInput();
        }}
        onSave={handleSaveSku}
        saving={saving}
        canEdit={canWriteSkus}
      />
      <Modal
        title="快速创建 SKU"
        open={Boolean(quickCreate?.open)}
        onCancel={() => {
          setQuickCreate(null);
          createForm.resetFields();
          focusInput();
        }}
        onOk={handleQuickCreate}
        okText="创建"
        okButtonProps={{ loading: creating, disabled: !canWriteSkus }}
        destroyOnClose
        data-testid="sku-quick-create-modal"
      >
        <Form form={createForm} layout="vertical">
          <Form.Item
            label="SKU 编码"
            name="sku_code"
            rules={[{ required: true, message: '请输入 SKU 编码' }]}
          >
            <Input placeholder="请输入 SKU 编码" />
          </Form.Item>
          <Form.Item label="条码" name="barcode">
            <Input placeholder="请输入条码" />
          </Form.Item>
          <Typography.Text type="secondary">
            未命中时会自动预填条码或 SKU 编码。
          </Typography.Text>
        </Form>
      </Modal>
    </>
  );
};

