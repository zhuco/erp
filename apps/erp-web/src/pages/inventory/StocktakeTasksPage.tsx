import {
  Button,
  Card,
  Col,
  Descriptions,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useRef, useState, type Key } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  assignStocktakeTask,
  cancelStocktakeTask,
  claimStocktakeTask,
  generateStocktakeTasks,
  getStocktakeTask,
  getStocktakeTaskRule,
  listStocktakeTasks,
  printStocktakeTasks,
  submitStocktakeTask,
  updateStocktakeTaskItems,
  updateStocktakeTaskRule,
  type StocktakeTaskAssignMode,
  type StocktakeTaskDetail,
  type StocktakeTaskItem,
  type StocktakeTaskListQuery,
  type StocktakeTaskRule,
  type StocktakeTaskSplitDimension,
  type StocktakeTaskStatus,
  type StocktakeTaskSummary
} from '@/api/stocktakeTasks';
import { listStockCounts } from '@/api/stockCounts';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState, LoadingState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  task_id?: string;
  count_id?: string;
  warehouse_id?: string;
  status?: StocktakeTaskStatus;
  assignee?: string;
};

type GenerateFormValues = {
  count_id: string;
  split_dimension: StocktakeTaskSplitDimension;
  assignment_mode: StocktakeTaskAssignMode;
};

type AssignFormValues = {
  assignee: string;
};

type EntryFormValues = {
  barcode: string;
  location_id?: string;
  counted_qty: number;
};

const statusLabels: Record<StocktakeTaskStatus, string> = {
  PENDING: '待领取',
  IN_PROGRESS: '进行中',
  SUBMITTED: '已提交',
  CANCELLED: '已作废'
};

const statusColors: Record<StocktakeTaskStatus, string> = {
  PENDING: 'blue',
  IN_PROGRESS: 'gold',
  SUBMITTED: 'green',
  CANCELLED: 'default'
};

const splitDimensionOptions = [
  { value: 'AREA', label: '库区' },
  { value: 'AISLE', label: '通道' },
  { value: 'LOCATION_TYPE', label: '货位类型' }
];

const assignmentModeOptions = [
  { value: 'CLAIM', label: '领取模式' },
  { value: 'ASSIGN', label: '指派模式' }
];

const statusOptions = [
  { value: 'PENDING', label: '待领取' },
  { value: 'IN_PROGRESS', label: '进行中' },
  { value: 'SUBMITTED', label: '已提交' },
  { value: 'CANCELLED', label: '已作废' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const StocktakeTasksPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [detailOpen, setDetailOpen] = useState(false);
  const [ruleOpen, setRuleOpen] = useState(false);
  const [generateOpen, setGenerateOpen] = useState(false);
  const [activeTaskId, setActiveTaskId] = useState<string | null>(null);
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [assignState, setAssignState] = useState<{
    open: boolean;
    task?: StocktakeTaskSummary;
  }>({ open: false });
  const [countedMap, setCountedMap] = useState<Record<string, number>>({});
  const [activeItemIndex, setActiveItemIndex] = useState(0);
  const [filterForm] = Form.useForm<FilterValues>();
  const [ruleForm] = Form.useForm<StocktakeTaskRule>();
  const [generateForm] = Form.useForm<GenerateFormValues>();
  const [assignForm] = Form.useForm<AssignFormValues>();
  const [entryForm] = Form.useForm<EntryFormValues>();
  const lastMatchedItemIdRef = useRef<string | null>(null);

  const syncCountedMap = (items: StocktakeTaskItem[]) => {
    const nextMap: Record<string, number> = {};
    items.forEach((item) => {
      nextMap[item.task_item_id] = item.counted_qty;
    });
    setCountedMap(nextMap);
  };

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'stocktake-tasks'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady,
    retry: false
  });

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const ruleQuery = useQuery({
    queryKey: ['stocktake-task-rule', tenantId],
    queryFn: () => getStocktakeTaskRule(tenantId),
    enabled: tenantReady,
    retry: false
  });

  const stockCountQuery = useQuery({
    queryKey: ['stock-counts', tenantId, 'stocktake-tasks'],
    queryFn: () => listStockCounts(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && generateOpen,
    retry: false
  });

  const stockCountOptions = useMemo(
    () =>
      stockCountQuery.data?.items.map((count) => ({
        label: `${count.count_id} (${warehouseMap.get(count.warehouse_id) ?? count.warehouse_id})`,
        value: count.count_id
      })) ?? [],
    [stockCountQuery.data?.items, warehouseMap]
  );

  const listQuery = useMemo<StocktakeTaskListQuery>(
    () => ({
      task_id: filters.task_id?.trim() || undefined,
      count_id: filters.count_id?.trim() || undefined,
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status,
      assignee: filters.assignee?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQueryState = useQuery({
    queryKey: ['stocktake-tasks', tenantId, listQuery],
    queryFn: () => listStocktakeTasks(tenantId, listQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const detailQuery = useQuery<StocktakeTaskDetail>({
    queryKey: ['stocktake-task-detail', tenantId, activeTaskId],
    queryFn: () => getStocktakeTask(tenantId, activeTaskId ?? ''),
    enabled: tenantReady && Boolean(activeTaskId),
    onSuccess: (detail) => {
      syncCountedMap(detail.items);
      setActiveItemIndex(0);
    }
  });

  const updateDetailState = (detail: StocktakeTaskDetail) => {
    queryClient.setQueryData(['stocktake-task-detail', tenantId, detail.task.task_id], detail);
    syncCountedMap(detail.items);
  };

  const ruleMutation = useMutation({
    mutationFn: async () => {
      const values = await ruleForm.validateFields();
      return updateStocktakeTaskRule(tenantId, values);
    },
    onSuccess: async () => {
      message.success('任务规则已更新');
      setRuleOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['stocktake-task-rule', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '更新任务规则失败');
    }
  });

  const generateMutation = useMutation({
    mutationFn: async () => {
      const values = await generateForm.validateFields();
      return generateStocktakeTasks(tenantId, values);
    },
    onSuccess: async () => {
      message.success('盘点任务已生成');
      setGenerateOpen(false);
      generateForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['stocktake-tasks', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '生成任务失败');
    }
  });

  const assignMutation = useMutation({
    mutationFn: async (values: AssignFormValues) => {
      if (!assignState.task) {
        throw new Error('请选择任务');
      }
      return assignStocktakeTask(tenantId, assignState.task.task_id, values);
    },
    onSuccess: async () => {
      message.success('任务已指派');
      setAssignState({ open: false });
      assignForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['stocktake-tasks', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '指派任务失败');
    }
  });

  const claimMutation = useMutation({
    mutationFn: (taskId: string) => claimStocktakeTask(tenantId, taskId),
    onSuccess: async () => {
      message.success('任务已领取');
      await queryClient.invalidateQueries({ queryKey: ['stocktake-tasks', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '领取任务失败');
    }
  });

  const cancelMutation = useMutation({
    mutationFn: (taskId: string) => cancelStocktakeTask(tenantId, taskId),
    onSuccess: async () => {
      message.success('任务已作废');
      await queryClient.invalidateQueries({ queryKey: ['stocktake-tasks', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['stocktake-task-detail', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '作废任务失败');
    }
  });

  const saveItemsMutation = useMutation({
    mutationFn: async (items: StocktakeTaskItem[]) => {
      if (!detailQuery.data) {
        throw new Error('任务详情未加载');
      }
      const updates = items.map((item) => ({
        task_item_id: item.task_item_id,
        counted_qty: countedMap[item.task_item_id] ?? item.counted_qty
      }));
      return updateStocktakeTaskItems(tenantId, detailQuery.data.task.task_id, updates);
    },
    onSuccess: (detail) => {
      message.success('盘点数据已保存');
      updateDetailState(detail);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存盘点数据失败');
    }
  });

  const entryMutation = useMutation({
    mutationFn: async (values: EntryFormValues) => {
      const detail = detailQuery.data;
      if (!detail) {
        throw new Error('任务详情未加载');
      }
      const barcode = values.barcode.trim();
      if (!barcode) {
        throw new Error('请输入货品条码');
      }
      if (values.counted_qty == null) {
        throw new Error('请输入盘点数量');
      }
      const locationId = values.location_id?.trim() || undefined;
      if (detail.location_enabled && !locationId) {
        throw new Error('请输入货位');
      }
      const matchedItem = detail.items.find((item) => {
        const codeMatched = item.barcode === barcode || item.sku_id === barcode;
        if (!codeMatched) {
          return false;
        }
        if (detail.location_enabled) {
          return item.location_id === locationId;
        }
        return true;
      });
      if (!matchedItem) {
        throw new Error('未找到匹配的任务明细');
      }
      lastMatchedItemIdRef.current = matchedItem.task_item_id;
      return updateStocktakeTaskItems(tenantId, detail.task.task_id, [
        { task_item_id: matchedItem.task_item_id, counted_qty: values.counted_qty }
      ]);
    },
    onSuccess: (detail) => {
      message.success('盘点录入成功');
      updateDetailState(detail);
      entryForm.resetFields();
      const matchedIndex = detail.items.findIndex(
        (item) => item.task_item_id === lastMatchedItemIdRef.current
      );
      lastMatchedItemIdRef.current = null;
      const nextIndex =
        matchedIndex >= 0 ? Math.min(matchedIndex + 1, detail.items.length - 1) : activeItemIndex;
      setActiveItemIndex(nextIndex);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '盘点录入失败');
    }
  });

  const submitMutation = useMutation({
    mutationFn: async () => {
      const detail = detailQuery.data;
      if (!detail) {
        throw new Error('任务详情未加载');
      }
      const items = detail.items.map((item) => ({
        task_item_id: item.task_item_id,
        counted_qty: countedMap[item.task_item_id] ?? item.counted_qty
      }));
      return submitStocktakeTask(tenantId, detail.task.task_id, { items });
    },
    onSuccess: async (detail) => {
      message.success('任务已提交');
      updateDetailState(detail);
      await queryClient.invalidateQueries({ queryKey: ['stocktake-tasks', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '提交任务失败');
    }
  });

  const printMutation = useMutation({
    mutationFn: (taskIds: string[]) => printStocktakeTasks(tenantId, taskIds),
    onSuccess: () => {
      message.success('任务单已发送到打印队列');
      setSelectedRowKeys([]);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '打印任务单失败');
    }
  });

  const assignmentMode = ruleQuery.data?.assignment_mode ?? 'CLAIM';
  const currentTask = detailQuery.data?.task;
  const detailItems = detailQuery.data?.items ?? [];
  const detailLoading = detailQuery.isFetching || detailQuery.isLoading;
  const isReadOnly = currentTask?.status === 'SUBMITTED' || currentTask?.status === 'CANCELLED';
  const currentItem = detailItems[activeItemIndex];
  const nextItem = detailItems[activeItemIndex + 1];

  const columns = useMemo<ColumnsType<StocktakeTaskSummary>>(
    () => [
      {
        title: '任务号',
        dataIndex: 'task_id'
      },
      {
        title: '盘点单号',
        dataIndex: 'count_id'
      },
      {
        title: '范围描述',
        dataIndex: 'range_desc'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value) => warehouseMap.get(value) ?? value
      },
      {
        title: '负责人',
        dataIndex: 'assignee',
        render: (value) => value ?? '-'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: StocktakeTaskStatus) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        )
      },
      {
        title: '完成时间',
        dataIndex: 'completed_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => {
          const canOperate = canWrite && record.status !== 'CANCELLED';
          const canClaim =
            assignmentMode === 'CLAIM' &&
            canOperate &&
            record.status === 'PENDING' &&
            !record.assignee;
          const canAssign =
            assignmentMode === 'ASSIGN' && canOperate && record.status !== 'SUBMITTED';
          const canCancel = canOperate && record.status !== 'SUBMITTED';

          return (
            <Space>
              <Button
                size="small"
                onClick={() => {
                  setActiveTaskId(record.task_id);
                  setDetailOpen(true);
                }}
              >
                详情
              </Button>
              {canAssign ? (
                <Button
                  size="small"
                  onClick={() => {
                    assignForm.setFieldsValue({ assignee: record.assignee ?? '' });
                    setAssignState({ open: true, task: record });
                  }}
                >
                  指派
                </Button>
              ) : null}
              {canClaim ? (
                <Button size="small" onClick={() => claimMutation.mutate(record.task_id)}>
                  领取
                </Button>
              ) : null}
              {canCancel ? (
                <Button
                  size="small"
                  danger
                  onClick={() =>
                    Modal.confirm({
                      title: '确认作废任务？',
                      content: '作废后任务将无法继续录入。',
                      onOk: () => cancelMutation.mutate(record.task_id)
                    })
                  }
                >
                  作废
                </Button>
              ) : null}
            </Space>
          );
        }
      }
    ],
    [
      assignmentMode,
      assignForm,
      canWrite,
      cancelMutation,
      claimMutation,
      warehouseMap
    ]
  );

  const detailColumns = useMemo<ColumnsType<StocktakeTaskItem>>(
    () => {
      const cols: ColumnsType<StocktakeTaskItem> = [
        {
          title: 'SKU',
          dataIndex: 'sku_id'
        },
        {
          title: '条码',
          dataIndex: 'barcode',
          render: (value) => value ?? '-'
        }
      ];
      if (detailQuery.data?.location_enabled) {
        cols.push({
          title: '货位',
          dataIndex: 'location_id',
          render: (value) => value ?? '-'
        });
      }
      cols.push(
        {
          title: '系统数量',
          dataIndex: 'system_qty'
        },
        {
          title: '盘点数量',
          dataIndex: 'counted_qty',
          render: (_, record) =>
            isReadOnly ? (
              countedMap[record.task_item_id] ?? record.counted_qty
            ) : (
              <InputNumber
                min={0}
                value={countedMap[record.task_item_id] ?? record.counted_qty}
                onChange={(value) => {
                  setCountedMap((prev) => ({
                    ...prev,
                    [record.task_item_id]: value ?? 0
                  }));
                }}
              />
            )
        },
        {
          title: '差异',
          key: 'diff_qty',
          render: (_, record) => {
            const counted = countedMap[record.task_item_id] ?? record.counted_qty;
            return counted - record.system_qty;
          }
        }
      );
      return cols;
    },
    [countedMap, detailQuery.data?.location_enabled, isReadOnly]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    filterForm.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleLocateTask = (taskId?: string) => {
    const trimmed = taskId?.trim();
    if (!trimmed) {
      return;
    }
    filterForm.setFieldsValue({ task_id: trimmed });
    setFilters((prev) => ({ ...prev, task_id: trimmed }));
    setPagination((prev) => ({ ...prev, current: 1 }));
    setActiveTaskId(trimmed);
    setDetailOpen(true);
  };

  const handlePrint = () => {
    if (selectedRowKeys.length === 0) {
      message.warning('请选择任务后再打印');
      return;
    }
    printMutation.mutate(selectedRowKeys as string[]);
  };

  useEffect(() => {
    if (ruleOpen && ruleQuery.data) {
      ruleForm.setFieldsValue(ruleQuery.data);
    }
  }, [ruleOpen, ruleQuery.data, ruleForm]);

  useEffect(() => {
    if (generateOpen) {
      generateForm.setFieldsValue({
        split_dimension: ruleQuery.data?.split_dimension ?? 'AREA',
        assignment_mode: ruleQuery.data?.assignment_mode ?? 'CLAIM'
      });
    }
  }, [generateOpen, generateForm, ruleQuery.data]);

  useEffect(() => {
    if (!detailOpen) {
      setActiveTaskId(null);
      entryForm.resetFields();
    }
  }, [detailOpen, entryForm]);

  useEffect(() => {
    if (detailQuery.data?.location_enabled && currentItem?.location_id) {
      entryForm.setFieldsValue({ location_id: currentItem.location_id });
    }
  }, [currentItem?.location_id, detailQuery.data?.location_enabled, entryForm]);

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Row justify="space-between" align="middle">
        <Typography.Title level={3} style={{ margin: 0 }}>
          盘点任务
        </Typography.Title>
        <Space>
          {canWrite ? (
            <Button type="primary" onClick={() => setGenerateOpen(true)}>
              生成任务
            </Button>
          ) : null}
          {canWrite ? <Button onClick={() => setRuleOpen(true)}>任务规则设置</Button> : null}
          {canWrite ? (
            <Button onClick={handlePrint} loading={printMutation.isPending}>
              打印
            </Button>
          ) : null}
        </Space>
      </Row>
      <Card>
        <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={6}>
              <Form.Item name="task_id" label="任务号/扫码">
                <Input.Search
                  allowClear
                  placeholder="输入任务号扫描"
                  enterButton="定位"
                  onSearch={(value) => handleLocateTask(value)}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="count_id" label="盘点单号">
                <Input allowClear placeholder="输入盘点单号" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse_id" label="仓库">
                <Select allowClear options={warehouseOptions} placeholder="选择仓库" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="任务状态">
                <Select allowClear options={statusOptions} placeholder="选择状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="assignee" label="负责人">
                <Input allowClear placeholder="输入负责人" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
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
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
      <UiModule03MainTableSection<StocktakeTaskSummary>
        rowKey="task_id"
        columns={columns}
        data={listQueryState.data?.items ?? []}
        loading={listQueryState.isFetching}
        error={listQueryState.error}
        onRetry={() => listQueryState.refetch()}
        errorTitle="盘点任务加载失败"
        emptyText="暂无盘点任务"
        total={listQueryState.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="inventory.stocktake.tasks.table.columns"
        rowSelection={
          canWrite
            ? {
                selectedRowKeys,
                onChange: (keys) => setSelectedRowKeys(keys)
              }
            : undefined
        }
        enablePanels={false}
      />
      <Modal
        title="任务规则设置"
        open={ruleOpen}
        onCancel={() => setRuleOpen(false)}
        onOk={() => ruleMutation.mutate()}
        confirmLoading={ruleMutation.isPending}
        destroyOnHidden
      >
        <Form
          form={ruleForm}
          layout="vertical"
          initialValues={{ split_dimension: 'AREA', assignment_mode: 'CLAIM' }}
        >
          <Form.Item
            name="split_dimension"
            label="拆分维度"
            rules={[{ required: true, message: '请选择拆分维度' }]}
          >
            <Select options={splitDimensionOptions} />
          </Form.Item>
          <Form.Item
            name="assignment_mode"
            label="任务模式"
            rules={[{ required: true, message: '请选择任务模式' }]}
          >
            <Select options={assignmentModeOptions} />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="生成盘点任务"
        open={generateOpen}
        onCancel={() => setGenerateOpen(false)}
        onOk={() => generateMutation.mutate()}
        confirmLoading={generateMutation.isPending}
        destroyOnHidden
      >
        <Form form={generateForm} layout="vertical">
          <Form.Item
            name="count_id"
            label="盘点单号"
            rules={[{ required: true, message: '请输入盘点单号' }]}
          >
            {stockCountOptions.length > 0 ? (
              <Select
                options={stockCountOptions}
                placeholder="选择盘点单"
                showSearch
                optionFilterProp="label"
              />
            ) : (
              <Input placeholder="输入盘点单号" />
            )}
          </Form.Item>
          <Form.Item
            name="split_dimension"
            label="拆分维度"
            rules={[{ required: true, message: '请选择拆分维度' }]}
          >
            <Select options={splitDimensionOptions} />
          </Form.Item>
          <Form.Item
            name="assignment_mode"
            label="任务模式"
            rules={[{ required: true, message: '请选择任务模式' }]}
          >
            <Select options={assignmentModeOptions} />
          </Form.Item>
        </Form>
        {stockCountQuery.error ? (
          <ErrorState
            title="盘点单加载失败"
            error={stockCountQuery.error}
            onRetry={() => stockCountQuery.refetch()}
          />
        ) : null}
      </Modal>
      <Modal
        title="指派任务"
        open={assignState.open}
        onCancel={() => setAssignState({ open: false })}
        onOk={() => assignForm.submit()}
        confirmLoading={assignMutation.isPending}
        destroyOnHidden
      >
        <Form form={assignForm} layout="vertical" onFinish={(values) => assignMutation.mutate(values)}>
          <Form.Item
            name="assignee"
            label="负责人"
            rules={[{ required: true, message: '请输入负责人' }]}
          >
            <Input placeholder="输入负责人姓名" />
          </Form.Item>
        </Form>
      </Modal>
      <Drawer
        title="盘点任务详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={820}
        destroyOnHidden
      >
        {detailLoading && !detailQuery.data ? (
          <LoadingState loading={detailLoading} onRetry={() => detailQuery.refetch()} />
        ) : null}
        {detailQuery.error ? (
          <ErrorState
            title="任务详情加载失败"
            error={detailQuery.error}
            onRetry={() => detailQuery.refetch()}
          />
        ) : null}
        {detailQuery.data ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Descriptions bordered size="small" column={2}>
              <Descriptions.Item label="任务号">
                {detailQuery.data.task.task_id}
              </Descriptions.Item>
              <Descriptions.Item label="盘点单号">
                {detailQuery.data.task.count_id}
              </Descriptions.Item>
              <Descriptions.Item label="仓库">
                {warehouseMap.get(detailQuery.data.task.warehouse_id) ??
                  detailQuery.data.task.warehouse_id}
              </Descriptions.Item>
              <Descriptions.Item label="状态">
                <Tag color={statusColors[detailQuery.data.task.status]}>
                  {statusLabels[detailQuery.data.task.status]}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label="负责人">
                {detailQuery.data.task.assignee ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="完成时间">
                {formatDateTime(detailQuery.data.task.completed_at)}
              </Descriptions.Item>
            </Descriptions>
            <Card size="small" title="任务范围">
              <Typography.Text>{detailQuery.data.task.range_desc}</Typography.Text>
            </Card>
            {currentItem ? (
              <Card size="small" title="作业指引">
                <Space direction="vertical" size={4}>
                  <Typography.Text>
                    当前SKU：{currentItem.sku_id}
                    {detailQuery.data.location_enabled && currentItem.location_id
                      ? ` / 货位 ${currentItem.location_id}`
                      : ''}
                  </Typography.Text>
                  {nextItem ? (
                    <Typography.Text type="secondary">
                      下一货位：{nextItem.location_id ?? nextItem.sku_id}
                    </Typography.Text>
                  ) : (
                    <Typography.Text type="secondary">当前为最后一条明细</Typography.Text>
                  )}
                </Space>
              </Card>
            ) : null}
            <Card size="small" title="扫码录入">
              <Form
                form={entryForm}
                layout="vertical"
                onFinish={(values) => entryMutation.mutate(values)}
                initialValues={{ counted_qty: 0 }}
                disabled={isReadOnly}
              >
                <Row gutter={16}>
                  <Col xs={24} md={8}>
                    <Form.Item
                      name="barcode"
                      label="货品条码"
                      rules={[{ required: true, message: '请输入货品条码' }]}
                    >
                      <Input placeholder="扫描或输入条码/SKU" />
                    </Form.Item>
                  </Col>
                  {detailQuery.data.location_enabled ? (
                    <Col xs={24} md={6}>
                      <Form.Item
                        name="location_id"
                        label="货位"
                        rules={[{ required: true, message: '请输入货位' }]}
                      >
                        <Input placeholder="扫描货位" />
                      </Form.Item>
                    </Col>
                  ) : null}
                  <Col xs={24} md={6}>
                    <Form.Item
                      name="counted_qty"
                      label="盘点数量"
                      rules={[{ required: true, message: '请输入盘点数量' }]}
                    >
                      <InputNumber min={0} style={{ width: '100%' }} />
                    </Form.Item>
                  </Col>
                  <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
                    <Button
                      type="primary"
                      htmlType="submit"
                      loading={entryMutation.isPending}
                      disabled={isReadOnly}
                    >
                      录入
                    </Button>
                  </Col>
                </Row>
              </Form>
              {isReadOnly ? (
                <Typography.Text type="secondary">
                  任务已提交或作废，当前仅可查看。
                </Typography.Text>
              ) : null}
            </Card>
            <Space>
              <Button
                onClick={() => saveItemsMutation.mutate(detailItems)}
                loading={saveItemsMutation.isPending}
                disabled={isReadOnly}
              >
                保存录入
              </Button>
              <Button
                type="primary"
                onClick={() => submitMutation.mutate()}
                loading={submitMutation.isPending}
                disabled={isReadOnly}
              >
                提交任务
              </Button>
            </Space>
            <Table
              rowKey="task_item_id"
              columns={detailColumns}
              dataSource={detailItems}
              pagination={false}
              size="small"
            />
          </Space>
        ) : null}
      </Drawer>
    </Space>
  );
};
