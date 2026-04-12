import dayjs from 'dayjs';

import type {
  StocktakeTaskAssignPayload,
  StocktakeTaskDetail,
  StocktakeTaskGeneratePayload,
  StocktakeTaskItem,
  StocktakeTaskItemUpdate,
  StocktakeTaskListQuery,
  StocktakeTaskListResponse,
  StocktakeTaskRule,
  StocktakeTaskSummary
} from '../stocktakeTasks';
import { applyPagination, mockSkus, mockWarehouses } from './data';

const mockAssignees = ['张敏', '李雷', '王芳'];
const mockRule: StocktakeTaskRule = {
  split_dimension: 'AREA',
  assignment_mode: 'CLAIM'
};

const mockTasks: StocktakeTaskSummary[] = [];
const mockTaskItems = new Map<string, StocktakeTaskItem[]>();

const splitDimensionLabels: Record<StocktakeTaskRule['split_dimension'], string> = {
  AREA: '库区',
  AISLE: '通道',
  LOCATION_TYPE: '货位类型'
};

const seedTasks = () => {
  if (mockTasks.length > 0) {
    return;
  }
  const now = dayjs();
  const warehouse = mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT';
  const countId = 'COUNT-202501';
  const baseTasks: Array<{
    status: StocktakeTaskSummary['status'];
    assignee?: string | null;
    completed?: boolean;
  }> = [
    { status: 'PENDING', assignee: null },
    { status: 'IN_PROGRESS', assignee: mockAssignees[0] },
    { status: 'SUBMITTED', assignee: mockAssignees[1], completed: true }
  ];

  baseTasks.forEach((seed, index) => {
    const taskId = `STASK-${1000 + index}`;
    const createdAt = now.subtract(index + 1, 'day').toISOString();
    const updatedAt = now.subtract(index, 'hour').toISOString();
    mockTasks.push({
      task_id: taskId,
      count_id: countId,
      warehouse_id: warehouse,
      range_desc: `${splitDimensionLabels.AREA} ${String.fromCharCode(65 + index)}`,
      assignee: seed.assignee ?? null,
      status: seed.status,
      completed_at: seed.completed ? now.subtract(index, 'hour').toISOString() : null,
      created_at: createdAt,
      updated_at: updatedAt
    });
    const locationEnabled = index % 2 === 0;
    const items = buildTaskItems(taskId, index, locationEnabled);
    mockTaskItems.set(taskId, items);
  });
};

const buildTaskItems = (taskId: string, seedIndex: number, locationEnabled: boolean) => {
  const start = seedIndex * 4;
  return Array.from({ length: 4 }, (_, offset): StocktakeTaskItem => {
    const sku = mockSkus[(start + offset) % mockSkus.length];
    const barcode = sku?.barcode ?? sku?.sku_code ?? sku?.sku_id ?? `BAR-${start + offset}`;
    return {
      task_item_id: `${taskId}-ITEM-${offset + 1}`,
      sku_id: sku?.sku_id ?? `SKU-${start + offset}`,
      barcode,
      location_id: locationEnabled ? `A-${seedIndex + 1}-${offset + 1}` : null,
      system_qty: 10 + offset,
      counted_qty: seedIndex === 2 ? 10 + offset : 0
    };
  });
};

const getTaskItems = (taskId: string): StocktakeTaskItem[] =>
  mockTaskItems.get(taskId)?.map((item) => ({ ...item })) ?? [];

const touchTask = (taskId: string) => {
  const task = mockTasks.find((item) => item.task_id === taskId);
  if (task) {
    task.updated_at = dayjs().toISOString();
  }
};

const buildDetail = (taskId: string): StocktakeTaskDetail => {
  const task = mockTasks.find((item) => item.task_id === taskId);
  if (!task) {
    throw new Error('任务不存在');
  }
  const items = getTaskItems(taskId);
  const locationEnabled = items.some((item) => Boolean(item.location_id));
  return {
    task: { ...task },
    items,
    location_enabled: locationEnabled
  };
};

export const getMockStocktakeTaskRule = (): StocktakeTaskRule => ({ ...mockRule });

export const updateMockStocktakeTaskRule = (payload: StocktakeTaskRule): StocktakeTaskRule => {
  mockRule.split_dimension = payload.split_dimension;
  mockRule.assignment_mode = payload.assignment_mode;
  return { ...mockRule };
};

export const listMockStocktakeTasks = (
  query: StocktakeTaskListQuery
): StocktakeTaskListResponse => {
  seedTasks();
  const normalizedAssignee = query.assignee?.trim().toLowerCase();
  const filtered = mockTasks.filter((task) => {
    if (query.task_id && task.task_id !== query.task_id) {
      return false;
    }
    if (query.count_id && task.count_id !== query.count_id) {
      return false;
    }
    if (query.warehouse_id && task.warehouse_id !== query.warehouse_id) {
      return false;
    }
    if (query.status && task.status !== query.status) {
      return false;
    }
    if (normalizedAssignee && task.assignee?.toLowerCase() !== normalizedAssignee) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const getMockStocktakeTask = (taskId: string): StocktakeTaskDetail => {
  seedTasks();
  return buildDetail(taskId);
};

export const generateMockStocktakeTasks = (
  payload: StocktakeTaskGeneratePayload
): StocktakeTaskListResponse => {
  seedTasks();
  const dimension = payload.split_dimension ?? mockRule.split_dimension;
  const label = splitDimensionLabels[dimension];
  const warehouse = mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT';
  const createdTasks: StocktakeTaskSummary[] = [];

  Array.from({ length: 3 }, (_, index) => {
    const taskId = `STASK-${Date.now()}-${index + 1}`;
    const now = dayjs().toISOString();
    const task: StocktakeTaskSummary = {
      task_id: taskId,
      count_id: payload.count_id,
      warehouse_id: warehouse,
      range_desc: `${label} ${index + 1}`,
      assignee: null,
      status: 'PENDING',
      completed_at: null,
      created_at: now,
      updated_at: now
    };
    mockTasks.unshift(task);
    const items = buildTaskItems(taskId, index + 1, index % 2 === 0);
    mockTaskItems.set(taskId, items);
    createdTasks.push(task);
  });

  return { items: createdTasks, total: createdTasks.length };
};

export const assignMockStocktakeTask = (
  taskId: string,
  payload: StocktakeTaskAssignPayload
): StocktakeTaskSummary => {
  seedTasks();
  const task = mockTasks.find((item) => item.task_id === taskId);
  const now = dayjs().toISOString();
  if (!task) {
    const fallback: StocktakeTaskSummary = {
      task_id: taskId,
      count_id: 'COUNT-UNKNOWN',
      warehouse_id: mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
      range_desc: '未知范围',
      assignee: payload.assignee,
      status: 'IN_PROGRESS',
      completed_at: null,
      created_at: now,
      updated_at: now
    };
    mockTasks.unshift(fallback);
    mockTaskItems.set(taskId, buildTaskItems(taskId, 0, true));
    return fallback;
  }
  task.assignee = payload.assignee;
  task.status = task.status === 'SUBMITTED' ? task.status : 'IN_PROGRESS';
  task.updated_at = now;
  return { ...task };
};

export const claimMockStocktakeTask = (taskId: string): StocktakeTaskSummary => {
  return assignMockStocktakeTask(taskId, { assignee: '当前用户' });
};

export const cancelMockStocktakeTask = (taskId: string): StocktakeTaskSummary => {
  seedTasks();
  const task = mockTasks.find((item) => item.task_id === taskId);
  const now = dayjs().toISOString();
  if (!task) {
    const fallback: StocktakeTaskSummary = {
      task_id: taskId,
      count_id: 'COUNT-UNKNOWN',
      warehouse_id: mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT',
      range_desc: '未知范围',
      assignee: null,
      status: 'CANCELLED',
      completed_at: null,
      created_at: now,
      updated_at: now
    };
    mockTasks.unshift(fallback);
    mockTaskItems.set(taskId, buildTaskItems(taskId, 0, false));
    return fallback;
  }
  task.status = 'CANCELLED';
  task.updated_at = now;
  return { ...task };
};

export const updateMockStocktakeTaskItems = (
  taskId: string,
  items: StocktakeTaskItemUpdate[]
): StocktakeTaskDetail => {
  seedTasks();
  const storedItems = mockTaskItems.get(taskId);
  if (!storedItems) {
    mockTaskItems.set(taskId, []);
  }
  const mutableItems = mockTaskItems.get(taskId) ?? [];
  items.forEach((update) => {
    const target = mutableItems.find((item) => item.task_item_id === update.task_item_id);
    if (target) {
      target.counted_qty = update.counted_qty;
    }
  });
  touchTask(taskId);
  return buildDetail(taskId);
};

export const submitMockStocktakeTask = (
  taskId: string,
  items?: StocktakeTaskItemUpdate[]
): StocktakeTaskDetail => {
  if (items && items.length > 0) {
    updateMockStocktakeTaskItems(taskId, items);
  }
  const task = mockTasks.find((item) => item.task_id === taskId);
  const now = dayjs().toISOString();
  if (task) {
    task.status = 'SUBMITTED';
    task.completed_at = now;
    task.updated_at = now;
  }
  return buildDetail(taskId);
};

export const printMockStocktakeTasks = (taskIds: string[]): void => {
  seedTasks();
  taskIds.forEach((taskId) => {
    const task = mockTasks.find((item) => item.task_id === taskId);
    if (task) {
      task.updated_at = dayjs().toISOString();
    }
  });
};
