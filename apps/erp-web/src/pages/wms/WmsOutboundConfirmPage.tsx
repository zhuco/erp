import {
  Alert,
  Button,
  Card,
  Descriptions,
  Drawer,
  Empty,
  Input,
  List,
  Modal,
  Space,
  Switch,
  Tag,
  Typography,
  message
} from 'antd';
import type { InputRef } from 'antd';
import { QuestionCircleOutlined, SettingOutlined } from '@ant-design/icons';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import dayjs from 'dayjs';

import type { ApiError } from '@/api/errors';
import { getWmsOutbound, getWmsWave, listWmsOutbounds, shipWmsOutbound } from '@/api/wms';
import type { WmsOutboundDoc, WmsWaveDetail } from '@/api/wms';
import { isTenantSelected, useTenantId } from '@/features/tenant';
import { useSessionStore } from '@/features/sessionStore';
import { useCan } from '@/features/rbac';
import { useFeatureFlag } from '@/features/featureFlags';
import { UiModule02WorkbenchTemplate } from '@/components/ui-modules';

import './wmsOutboundConfirmPage.css';

type ConfirmSettings = {
  clearOnSuccess: boolean;
  keepOnError: boolean;
  autoFocus: boolean;
};

type RecentRecord = {
  id: string;
  input: string;
  title: string;
  status: 'success' | 'error' | 'warning';
  message: string;
  count?: number;
  timestamp: string;
};

const settingsStorageKey = 'wms.outbound.confirm.settings';
const defaultSettings: ConfirmSettings = {
  clearOnSuccess: true,
  keepOnError: false,
  autoFocus: true
};

const uuidPattern =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

const isUuid = (value: string) => uuidPattern.test(value);

export const WmsOutboundConfirmPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('wms.write');
  const waveFlag = useFeatureFlag('wms.enable_wave');
  const userName = useSessionStore((state) => state.userName);
  const inputRef = useRef<InputRef>(null);
  const [keyword, setKeyword] = useState('');
  const [processing, setProcessing] = useState(false);
  const [shippedCount, setShippedCount] = useState(0);
  const [records, setRecords] = useState<RecentRecord[]>([]);
  const [settings, setSettings] = useState<ConfirmSettings>(defaultSettings);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
  const [waveDetail, setWaveDetail] = useState<WmsWaveDetail | null>(null);
  const [waveConfirmOpen, setWaveConfirmOpen] = useState(false);
  const [waveSubmitting, setWaveSubmitting] = useState(false);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(settingsStorageKey);
      if (!raw) {
        return;
      }
      const parsed = JSON.parse(raw) as Partial<ConfirmSettings>;
      setSettings((prev) => ({ ...prev, ...parsed }));
    } catch {
      setSettings(defaultSettings);
    }
  }, []);

  useEffect(() => {
    localStorage.setItem(settingsStorageKey, JSON.stringify(settings));
  }, [settings]);

  const focusInput = useCallback(() => {
    if (!settings.autoFocus) {
      return;
    }
    requestAnimationFrame(() => inputRef.current?.focus());
  }, [settings.autoFocus]);

  useEffect(() => {
    focusInput();
  }, [focusInput]);

  const addRecord = useCallback((record: RecentRecord) => {
    setRecords((prev) => [record, ...prev].slice(0, 10));
  }, []);

  const resolveOutbound = useCallback(
    async (value: string) => {
      if (isUuid(value)) {
        try {
          const detail = await getWmsOutbound(tenantId, value);
          return detail.doc;
        } catch (error) {
          const apiError = error as ApiError;
          if (apiError?.status && apiError.status !== 404) {
            throw error;
          }
        }
      }

      const exactDoc = await listWmsOutbounds(tenantId, {
        doc_no: value,
        limit: 5,
        offset: 0
      });
      if (exactDoc.items.length > 0) {
        if (exactDoc.items.length > 1) {
          message.warning('匹配到多个出库单，将处理最新的一单');
        }
        return exactDoc.items[0];
      }

      const fuzzyDoc = await listWmsOutbounds(tenantId, {
        doc_no_keyword: value,
        limit: 5,
        offset: 0
      });
      if (fuzzyDoc.items.length > 0) {
        if (fuzzyDoc.items.length > 1) {
          message.warning('匹配到多个出库单，将处理最新的一单');
        }
        return fuzzyDoc.items[0];
      }

      const tracking = await listWmsOutbounds(tenantId, {
        tracking_no: value,
        limit: 5,
        offset: 0
      });
      if (tracking.items.length > 0) {
        if (tracking.items.length > 1) {
          message.warning('匹配到多个出库单，将处理最新的一单');
        }
        return tracking.items[0];
      }

      if (isUuid(value)) {
        const shipment = await listWmsOutbounds(tenantId, {
          shipment_id: value,
          limit: 5,
          offset: 0
        });
        if (shipment.items.length > 0) {
          if (shipment.items.length > 1) {
            message.warning('匹配到多个出库单，将处理最新的一单');
          }
          return shipment.items[0];
        }
      }

      return null;
    },
    [tenantId]
  );

  const shipOutbound = useCallback(
    async (doc: WmsOutboundDoc, inputValue: string) => {
      try {
        await shipWmsOutbound(tenantId, doc.outbound_doc_id, {
          shipped_at: new Date().toISOString()
        });
        setShippedCount((prev) => prev + 1);
        message.success(`出库确认成功：${doc.doc_no}`);
        addRecord({
          id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
          input: inputValue,
          title: doc.doc_no,
          status: 'success',
          message: '出库确认成功',
          count: 1,
          timestamp: dayjs().format('HH:mm:ss')
        });
        if (settings.clearOnSuccess) {
          setKeyword('');
        }
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : '出库确认失败';
        message.error(errorMessage);
        addRecord({
          id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
          input: inputValue,
          title: doc.doc_no,
          status: 'error',
          message: errorMessage,
          timestamp: dayjs().format('HH:mm:ss')
        });
        if (!settings.keepOnError) {
          setKeyword('');
        }
      } finally {
        focusInput();
      }
    },
    [
      addRecord,
      focusInput,
      settings.clearOnSuccess,
      settings.keepOnError,
      tenantId
    ]
  );

  const handleSubmit = async () => {
    const value = keyword.trim();
    if (!value) {
      message.warning('请输入发货单号/物流单号/关联单号');
      return;
    }
    if (!tenantReady) {
      message.error('请先选择商户');
      return;
    }
    if (!canWrite) {
      message.error('当前账号无出库权限');
      return;
    }

    setProcessing(true);
    try {
      if (waveFlag.enabled && isUuid(value)) {
        try {
          const wave = await getWmsWave(tenantId, value);
          if (wave) {
            setWaveDetail(wave);
            setWaveConfirmOpen(true);
            return;
          }
        } catch (error) {
          const apiError = error as ApiError;
          if (!apiError?.status || apiError.status !== 404) {
            throw error;
          }
        }
      }

      const outbound = await resolveOutbound(value);
      if (!outbound) {
        message.error('未找到匹配的出库单');
        addRecord({
          id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
          input: value,
          title: value,
          status: 'error',
          message: '未找到匹配的出库单',
          timestamp: dayjs().format('HH:mm:ss')
        });
        if (!settings.keepOnError) {
          setKeyword('');
        }
        return;
      }
      await shipOutbound(outbound, value);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : '处理失败';
      message.error(errorMessage);
      addRecord({
        id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        input: value,
        title: value,
        status: 'error',
        message: errorMessage,
        timestamp: dayjs().format('HH:mm:ss')
      });
    } finally {
      setProcessing(false);
      focusInput();
    }
  };

  const waveSummary = useMemo(() => {
    if (!waveDetail) {
      return null;
    }
    const eligibleDocs = waveDetail.docs.filter(
      (doc) => doc.status !== 'SHIPPED' && doc.status !== 'CANCELLED'
    );
    return {
      total: waveDetail.docs.length,
      eligible: eligibleDocs.length,
      eligibleDocs
    };
  }, [waveDetail]);

  const handleWaveConfirm = async () => {
    if (!waveDetail || !waveSummary) {
      return;
    }
    if (!waveSummary.eligibleDocs.length) {
      message.info('波次内没有可出库单据');
      setWaveConfirmOpen(false);
      setWaveDetail(null);
      return;
    }
    setWaveSubmitting(true);
    try {
      const results = await Promise.allSettled(
        waveSummary.eligibleDocs.map((doc) =>
          shipWmsOutbound(tenantId, doc.outbound_doc_id, {
            shipped_at: new Date().toISOString()
          })
        )
      );
      const failures: Array<{ doc_no: string; reason: string }> = [];
      const successCount = results.reduce((count, result, index) => {
        if (result.status === 'fulfilled') {
          return count + 1;
        }
        const reason =
          result.reason instanceof Error ? result.reason.message : '出库确认失败';
        failures.push({
          doc_no: waveSummary.eligibleDocs[index]?.doc_no ?? '-',
          reason
        });
        return count;
      }, 0);

      if (successCount > 0) {
        setShippedCount((prev) => prev + successCount);
        message.success(`波次出库完成：成功 ${successCount} 单`);
      }
      if (failures.length > 0) {
        Modal.error({
          title: '波次出库存在失败',
          content: (
            <List
              size="small"
              dataSource={failures}
              renderItem={(item) => (
                <List.Item>
                  <Space direction="vertical" size={0}>
                    <Typography.Text>{item.doc_no}</Typography.Text>
                    <Typography.Text type="secondary">{item.reason}</Typography.Text>
                  </Space>
                </List.Item>
              )}
            />
          )
        });
      }

      addRecord({
        id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
        input: waveDetail.wave.wave_id,
        title: `波次 ${waveDetail.wave.wave_id}`,
        status: failures.length ? 'warning' : 'success',
        message: failures.length
          ? `成功 ${successCount} 单，失败 ${failures.length} 单`
          : `成功 ${successCount} 单`,
        count: successCount,
        timestamp: dayjs().format('HH:mm:ss')
      });
    } finally {
      setWaveSubmitting(false);
      setWaveConfirmOpen(false);
      setWaveDetail(null);
      setKeyword(settings.clearOnSuccess ? '' : keyword);
      focusInput();
    }
  };

  const recordStatusTag = (record: RecentRecord) => {
    if (record.status === 'success') {
      return <Tag color="green">成功</Tag>;
    }
    if (record.status === 'warning') {
      return <Tag color="orange">部分失败</Tag>;
    }
    return <Tag color="red">失败</Tag>;
  };

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      出库确认
    </Typography.Title>
  );

  const toolbarRight = (
    <Space>
      <Button icon={<SettingOutlined />} size="small" onClick={() => setSettingsOpen(true)}>
        设置
      </Button>
      <Button icon={<QuestionCircleOutlined />} size="small" onClick={() => setHelpOpen(true)}>
        帮助
      </Button>
    </Space>
  );

  return (
    <UiModule02WorkbenchTemplate
      toolbarLeft={toolbarLeft}
      toolbarRight={toolbarRight}
      tableContent={
        <div className="wms-outbound-confirm" data-testid="outbound-confirm-page">
          {!canWrite ? (
            <Alert
              type="warning"
          showIcon
          message="当前账号没有出库权限"
          description="请联系管理员授权后再进行出库确认操作。"
        />
      ) : null}

      <Card className="wms-outbound-confirm__input-card">
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <div className="wms-outbound-confirm__input">
            <Input
              ref={inputRef}
              size="large"
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              onPressEnter={handleSubmit}
              placeholder="发货单号/物流单号/关联单号"
              disabled={processing || !tenantReady || !canWrite}
              allowClear
              data-testid="outbound-confirm-input"
            />
          </div>
          <Descriptions column={4} size="small">
            <Descriptions.Item label="配货人">{userName || '-'}</Descriptions.Item>
            <Descriptions.Item label="验货人">{userName || '-'}</Descriptions.Item>
            <Descriptions.Item label="打包人">{userName || '-'}</Descriptions.Item>
            <Descriptions.Item label="已出库数量">{shippedCount}</Descriptions.Item>
          </Descriptions>
        </Space>
      </Card>

      <Card title="最近处理记录" data-testid="outbound-confirm-recent">
        {records.length ? (
          <List
            dataSource={records}
            renderItem={(record) => (
              <List.Item>
                <Space direction="vertical" size={4}>
                  <Space>
                    {recordStatusTag(record)}
                    <Typography.Text>{record.title}</Typography.Text>
                    {record.count ? (
                      <Tag color="blue">+{record.count}</Tag>
                    ) : null}
                  </Space>
                  <Typography.Text type="secondary">{record.message}</Typography.Text>
                </Space>
                <Typography.Text type="secondary">{record.timestamp}</Typography.Text>
              </List.Item>
            )}
          />
        ) : (
          <Empty description="暂无记录" />
        )}
      </Card>

      <Drawer
        title="出库设置"
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        width={360}
      >
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Space align="center" className="wms-outbound-confirm__setting-row">
            <Typography.Text>成功后清空输入</Typography.Text>
            <Switch
              checked={settings.clearOnSuccess}
              onChange={(value) =>
                setSettings((prev) => ({ ...prev, clearOnSuccess: value }))
              }
            />
          </Space>
          <Space align="center" className="wms-outbound-confirm__setting-row">
            <Typography.Text>失败后保留输入</Typography.Text>
            <Switch
              checked={settings.keepOnError}
              onChange={(value) => setSettings((prev) => ({ ...prev, keepOnError: value }))}
            />
          </Space>
          <Space align="center" className="wms-outbound-confirm__setting-row">
            <Typography.Text>处理后自动聚焦</Typography.Text>
            <Switch
              checked={settings.autoFocus}
              onChange={(value) => setSettings((prev) => ({ ...prev, autoFocus: value }))}
            />
          </Space>
        </Space>
      </Drawer>

      <Modal
        title="出库确认帮助"
        open={helpOpen}
        onCancel={() => setHelpOpen(false)}
        footer={[
          <Button key="close" type="primary" onClick={() => setHelpOpen(false)}>
            我知道了
          </Button>
        ]}
      >
        <Space direction="vertical" size={12}>
          <Typography.Text>
            支持扫描发货单号、物流单号或关联单号，回车即可确认出库。
          </Typography.Text>
          <Typography.Text>
            如扫描波次号，将展示波次内可出库单据并执行批量出库确认。
          </Typography.Text>
          <Typography.Text type="secondary">
            若提示“未找到单据/状态不允许”，请核对单号或联系管理员处理。
          </Typography.Text>
        </Space>
      </Modal>

      <Modal
        title="波次批量出库确认"
        open={waveConfirmOpen}
        onCancel={() => {
          setWaveConfirmOpen(false);
          setWaveDetail(null);
        }}
        onOk={handleWaveConfirm}
        okText="确认出库"
        confirmLoading={waveSubmitting}
        data-testid="outbound-confirm-wave-modal"
        okButtonProps={{ 'data-testid': 'outbound-confirm-wave-submit' }}
      >
        {waveSummary ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Typography.Text>波次号：{waveDetail?.wave.wave_id}</Typography.Text>
            <Typography.Text>本波次共 {waveSummary.total} 单</Typography.Text>
            <Typography.Text>可出库 {waveSummary.eligible} 单</Typography.Text>
            <Typography.Text type="secondary">
              点击确认后将批量执行出库确认。
            </Typography.Text>
          </Space>
        ) : (
          <Typography.Text type="secondary">未找到波次信息</Typography.Text>
        )}
      </Modal>
        </div>
      }
    />
  );
};
