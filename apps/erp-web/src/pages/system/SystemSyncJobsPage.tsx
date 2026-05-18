import {
  Button,
  Col,
  Form,
  Input,
  Modal,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';

import { useCan } from '@/features/rbac';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { IndustrialCard, IndustrialTable } from '@/components/industrial';

type JobStatus = 'RUNNING' | 'PAUSED' | 'FAILED' | 'SUCCESS';
type JobMode = 'FULL' | 'INCREMENTAL';

type SyncJob = {
  job_id: string;
  name: string;
  target_system: string;
  mode: JobMode;
  schedule: string;
  status: JobStatus;
  last_run?: string;
  next_run?: string;
  failure_count: number;
  last_error?: string;
  ticket_id?: string;
};

type JobLog = {
  log_id: string;
  job_id: string;
  status: JobStatus;
  message: string;
  created_at: string;
};

type JobFormValues = {
  schedule: string;
  mode: JobMode;
};

const jobStorageKey = 'system.sync-jobs.list';
const logStorageKey = 'system.sync-jobs.logs';

const statusColors: Record<JobStatus, string> = {
  RUNNING: 'blue',
  PAUSED: 'default',
  FAILED: 'red',
  SUCCESS: 'green'
};

const modeOptions = [
  { label: '全量同步', value: 'FULL' },
  { label: '增量同步', value: 'INCREMENTAL' }
];

const defaultJobs: SyncJob[] = [
  {
    job_id: 'JOB-ORDER',
    name: '订单主数据同步',
    target_system: '电商平台',
    mode: 'INCREMENTAL',
    schedule: '*/15 * * * *',
    status: 'RUNNING',
    last_run: dayjs().subtract(12, 'minute').toISOString(),
    next_run: dayjs().add(3, 'minute').toISOString(),
    failure_count: 0
  },
  {
    job_id: 'JOB-INV',
    name: '库存余额同步',
    target_system: 'WMS',
    mode: 'FULL',
    schedule: '0 */2 * * *',
    status: 'FAILED',
    last_run: dayjs().subtract(3, 'hour').toISOString(),
    next_run: dayjs().add(1, 'hour').toISOString(),
    failure_count: 2,
    last_error: '接口超时',
    ticket_id: 'TICKET-0081'
  }
];

const defaultLogs: JobLog[] = [
  {
    log_id: 'LOG-1001',
    job_id: 'JOB-ORDER',
    status: 'SUCCESS',
    message: '同步完成，新增 120 条订单',
    created_at: dayjs().subtract(1, 'hour').toISOString()
  },
  {
    log_id: 'LOG-1002',
    job_id: 'JOB-INV',
    status: 'FAILED',
    message: '同步失败：接口超时',
    created_at: dayjs().subtract(3, 'hour').toISOString()
  }
];

const loadFromStorage = <T,>(key: string, fallback: T): T => {
  const raw = localStorage.getItem(key);
  if (!raw) {
    return fallback;
  }
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
};

const formatDateTime = (value?: string) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

export const SystemSyncJobsPage = () => {
  const canWrite = useCan('SYS.SETTINGS.WRITE');
  const [jobs, setJobs] = useState<SyncJob[]>(() =>
    loadFromStorage(jobStorageKey, defaultJobs)
  );
  const [logs, setLogs] = useState<JobLog[]>(() =>
    loadFromStorage(logStorageKey, defaultLogs)
  );
  const [filterValues, setFilterValues] = useState<{ keyword?: string; status?: JobStatus }>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [modalOpen, setModalOpen] = useState(false);
  const [logOpen, setLogOpen] = useState(false);
  const [editing, setEditing] = useState<SyncJob | null>(null);
  const [activeLogs, setActiveLogs] = useState<JobLog[]>([]);
  const [form] = Form.useForm<JobFormValues>();

  useEffect(() => {
    localStorage.setItem(jobStorageKey, JSON.stringify(jobs));
  }, [jobs]);

  useEffect(() => {
    localStorage.setItem(logStorageKey, JSON.stringify(logs));
  }, [logs]);

  const openConfig = (record: SyncJob) => {
    setEditing(record);
    form.setFieldsValue({ schedule: record.schedule, mode: record.mode });
    setModalOpen(true);
  };

  const handleSave = async () => {
    const values = await form.validateFields();
    if (!editing) {
      return;
    }
    setJobs((prev) =>
      prev.map((item) =>
        item.job_id === editing.job_id ? { ...item, ...values } : item
      )
    );
    message.success('同步任务配置已更新');
    setModalOpen(false);
    setEditing(null);
  };

  const handleRun = (record: SyncJob) => {
    if (record.status === 'PAUSED') {
      message.warning('任务已暂停，请先启用');
      return;
    }
    const now = dayjs();
    const nextRun = now.add(30, 'minute').toISOString();
    setJobs((prev) =>
      prev.map((item) =>
        item.job_id === record.job_id
          ? {
              ...item,
              status: 'SUCCESS',
              last_run: now.toISOString(),
              next_run: nextRun,
              last_error: undefined
            }
          : item
      )
    );
    setLogs((prev) => [
      {
        log_id: `LOG-${Date.now()}`,
        job_id: record.job_id,
        status: 'SUCCESS',
        message: '手动触发同步成功',
        created_at: now.toISOString()
      },
      ...prev
    ]);
    message.success('已触发同步任务');
  };

  const handleToggle = (record: SyncJob) => {
    if (!canWrite) {
      message.warning('暂无权限');
      return;
    }
    setJobs((prev) =>
      prev.map((item) =>
        item.job_id === record.job_id
          ? { ...item, status: item.status === 'PAUSED' ? 'RUNNING' : 'PAUSED' }
          : item
      )
    );
  };

  const filteredJobs = jobs.filter((job) => {
    if (filterValues.status && job.status !== filterValues.status) {
      return false;
    }
    if (filterValues.keyword && !job.name.includes(filterValues.keyword)) {
      return false;
    }
    return true;
  });

  const pageData = useMemo(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    return filteredJobs.slice((current - 1) * pageSize, current * pageSize);
  }, [filteredJobs, pagination.current, pagination.pageSize]);

  const columns = useMemo<ColumnsType<SyncJob>>(
    () => [
      { title: '任务名称', dataIndex: 'name' },
      { title: '目标系统', dataIndex: 'target_system' },
      {
        title: '同步模式',
        dataIndex: 'mode',
        render: (value: JobMode) =>
          modeOptions.find((option) => option.value === value)?.label ?? value
      },
      { title: '调度频率', dataIndex: 'schedule' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: JobStatus) => <Tag color={statusColors[value]}>{value}</Tag>
      },
      {
        title: '最近运行',
        dataIndex: 'last_run',
        render: (value?: string) => formatDateTime(value)
      },
      {
        title: '下次运行',
        dataIndex: 'next_run',
        render: (value?: string) => formatDateTime(value)
      },
      { title: '失败次数', dataIndex: 'failure_count' },
      {
        title: '最新工单',
        dataIndex: 'ticket_id',
        render: (value?: string) => value ?? '-'
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space size={8}>
            <Button size="small" onClick={() => handleRun(record)}>
              立即运行
            </Button>
            <Button size="small" disabled={!canWrite} onClick={() => openConfig(record)}>
              配置
            </Button>
            <Button size="small" disabled={!canWrite} onClick={() => handleToggle(record)}>
              {record.status === 'PAUSED' ? '启用' : '暂停'}
            </Button>
            <Button
              size="small"
              onClick={() => {
                setActiveLogs(logs.filter((log) => log.job_id === record.job_id));
                setLogOpen(true);
              }}
            >
              日志
            </Button>
          </Space>
        )
      }
    ],
    [logs, canWrite]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        同步任务
      </Typography.Title>
      <IndustrialCard>
        <Form layout="vertical">
          <Row gutter={16}>
            <Col xs={24} md={10}>
              <Form.Item label="关键字">
                <Input
                  placeholder="输入任务名称"
                  value={filterValues.keyword}
                  onChange={(event) => {
                    setFilterValues((prev) => ({ ...prev, keyword: event.target.value }));
                    setPagination((prev) => ({ ...prev, current: 1 }));
                  }}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item label="状态">
                <Select
                  allowClear
                  placeholder="全部状态"
                  options={[
                    { label: '运行中', value: 'RUNNING' },
                    { label: '暂停', value: 'PAUSED' },
                    { label: '失败', value: 'FAILED' },
                    { label: '成功', value: 'SUCCESS' }
                  ]}
                  value={filterValues.status}
                  onChange={(value) => {
                    setFilterValues((prev) => ({ ...prev, status: value }));
                    setPagination((prev) => ({ ...prev, current: 1 }));
                  }}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button
                  onClick={() => {
                    setFilterValues({});
                    setPagination((prev) => ({ ...prev, current: 1 }));
                  }}
                >
                  重置
                </Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </IndustrialCard>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<SyncJob>
        header={header}
        columns={columns}
        data={pageData}
        loading={false}
        error={undefined}
        onRetry={() => undefined}
        emptyText="暂无同步任务"
        total={filteredJobs.length}
        pagination={pagination}
        onPaginationChange={setPagination}
        rowKey="job_id"
        enablePanels={false}
      />
      <Modal
        title="同步任务配置"
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSave}
        okText="保存"
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="schedule"
            label="调度频率"
            rules={[{ required: true, message: '请输入调度频率' }]}
          >
            <Input placeholder="Cron 表达式，例如 */15 * * * *" />
          </Form.Item>
          <Form.Item
            name="mode"
            label="同步模式"
            rules={[{ required: true, message: '请选择同步模式' }]}
          >
            <Select options={modeOptions} />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="同步日志"
        open={logOpen}
        onCancel={() => setLogOpen(false)}
        footer={[
          <Button key="close" onClick={() => setLogOpen(false)}>
            关闭
          </Button>
        ]}
      >
        <IndustrialTable<JobLog>
          rowKey="log_id"
          dataSource={activeLogs}
          pagination={false}
          columns={[
            {
              title: '时间',
              dataIndex: 'created_at',
              render: (value: string) => formatDateTime(value)
            },
            {
              title: '状态',
              dataIndex: 'status',
              render: (value: JobStatus) => <Tag color={statusColors[value]}>{value}</Tag>
            },
            { title: '说明', dataIndex: 'message' }
          ]}
        />
      </Modal>
    </>
  );
};
