import dayjs from 'dayjs';

import type {
  WmsDockAppointment,
  WmsDockAppointmentQuery,
  WmsExecutionListResponse,
  WmsLaborTask,
  WmsLaborTaskQuery,
  WmsReplenishmentTask,
  WmsReplenishmentTaskQuery,
  WmsSlottingSuggestion,
  WmsSlottingSuggestionQuery
} from '../wmsExecution';
import { applyPagination, mockSkus, mockWarehouses } from './data';

const defaultWarehouseId = mockWarehouses[0]?.warehouse_id ?? 'WH-DEFAULT';

const mockLaborTasks: WmsLaborTask[] = Array.from({ length: 12 }, (_, index) => ({
  task_id: `LAB-${String(index + 1).padStart(4, '0')}`,
  warehouse_id: defaultWarehouseId,
  zone_code: `ZONE-${(index % 4) + 1}`,
  task_type: index % 3 === 0 ? 'PICK' : index % 3 === 1 ? 'PUTAWAY' : 'COUNT',
  assignee: null,
  planned_qty: 20 + index * 3,
  finished_qty: 0,
  status: 'UNASSIGNED',
  updated_at: dayjs().subtract(index, 'hour').toISOString()
}));

const mockSlottingSuggestions: WmsSlottingSuggestion[] = Array.from({ length: 8 }, (_, index) => {
  const sku = mockSkus[index % mockSkus.length];
  return {
    suggestion_id: `SLOT-${String(index + 1).padStart(4, '0')}`,
    warehouse_id: defaultWarehouseId,
    sku_id: sku?.sku_id ?? `SKU-${index + 1}`,
    from_location_id: `A-${index + 1}-01`,
    to_location_id: `B-${index + 1}-03`,
    reason: index % 2 === 0 ? '高频商品前移' : '减少跨区拣选路径',
    priority: index % 3 === 0 ? 'HIGH' : index % 3 === 1 ? 'MEDIUM' : 'LOW',
    status: 'PENDING',
    updated_at: dayjs().subtract(index, 'day').toISOString()
  };
});

const mockReplenishmentTasks: WmsReplenishmentTask[] = Array.from({ length: 10 }, (_, index) => {
  const sku = mockSkus[(index + 2) % mockSkus.length];
  return {
    task_id: `REP-${String(index + 1).padStart(4, '0')}`,
    warehouse_id: defaultWarehouseId,
    sku_id: sku?.sku_id ?? `SKU-R-${index + 1}`,
    from_location_id: `BK-${(index % 5) + 1}-01`,
    to_location_id: `PK-${(index % 3) + 1}-02`,
    qty: 10 + index,
    assignee: null,
    status: 'PENDING',
    updated_at: dayjs().subtract(index, 'hour').toISOString()
  };
});

const mockDockAppointments: WmsDockAppointment[] = Array.from({ length: 6 }, (_, index) => ({
  appointment_id: `DOCK-${String(index + 1).padStart(4, '0')}`,
  warehouse_id: defaultWarehouseId,
  dock_no: `D-${(index % 3) + 1}`,
  carrier_name: index % 2 === 0 ? '顺丰' : '中通',
  vehicle_no: `沪A${1000 + index}`,
  eta_at: dayjs().add(index + 1, 'hour').toISOString(),
  status: 'SCHEDULED',
  updated_at: dayjs().subtract(index, 'minute').toISOString()
}));

const byUpdatedAtDesc = <T extends { updated_at: string }>(a: T, b: T) =>
  dayjs(b.updated_at).valueOf() - dayjs(a.updated_at).valueOf();

const touch = <T extends { updated_at: string }>(item: T): T => ({
  ...item,
  updated_at: dayjs().toISOString()
});

const listByQuery = <
  T extends { warehouse_id: string; status: string; updated_at: string },
  Q extends { warehouse_id?: string; status?: string; offset?: number; limit?: number }
>(
  items: T[],
  query: Q
): WmsExecutionListResponse<T> => {
  const filtered = items
    .filter((item) => {
      if (query.warehouse_id && item.warehouse_id !== query.warehouse_id) {
        return false;
      }
      if (query.status && item.status !== query.status) {
        return false;
      }
      return true;
    })
    .sort(byUpdatedAtDesc);
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

const findIndexOrThrow = <T extends { [K in Key]: string }, Key extends string>(
  items: T[],
  key: Key,
  value: string,
  message: string
) => {
  const index = items.findIndex((item) => item[key] === value);
  if (index < 0) {
    throw new Error(message);
  }
  return index;
};

export const listMockWmsLaborTasks = (
  query: WmsLaborTaskQuery
): WmsExecutionListResponse<WmsLaborTask> => listByQuery(mockLaborTasks, query);

export const assignMockWmsLaborTask = (
  taskId: string,
  payload: { assignee: string }
): WmsLaborTask => {
  const index = findIndexOrThrow(mockLaborTasks, 'task_id', taskId, 'labor 任务不存在');
  const current = mockLaborTasks[index];
  const status = current.status === 'DONE' ? current.status : 'ASSIGNED';
  const updated = touch({
    ...current,
    assignee: payload.assignee,
    status
  });
  mockLaborTasks[index] = updated;
  return updated;
};

export const startMockWmsLaborTask = (taskId: string): WmsLaborTask => {
  const index = findIndexOrThrow(mockLaborTasks, 'task_id', taskId, 'labor 任务不存在');
  const current = mockLaborTasks[index];
  const updated = touch({
    ...current,
    status: 'IN_PROGRESS'
  });
  mockLaborTasks[index] = updated;
  return updated;
};

export const completeMockWmsLaborTask = (taskId: string): WmsLaborTask => {
  const index = findIndexOrThrow(mockLaborTasks, 'task_id', taskId, 'labor 任务不存在');
  const current = mockLaborTasks[index];
  const updated = touch({
    ...current,
    status: 'DONE',
    finished_qty: Math.max(current.finished_qty, current.planned_qty)
  });
  mockLaborTasks[index] = updated;
  return updated;
};

export const listMockWmsSlottingSuggestions = (
  query: WmsSlottingSuggestionQuery
): WmsExecutionListResponse<WmsSlottingSuggestion> => listByQuery(mockSlottingSuggestions, query);

export const applyMockWmsSlottingSuggestion = (suggestionId: string): WmsSlottingSuggestion => {
  const index = findIndexOrThrow(
    mockSlottingSuggestions,
    'suggestion_id',
    suggestionId,
    'slotting 建议不存在'
  );
  const updated = touch({
    ...mockSlottingSuggestions[index],
    status: 'APPLIED'
  });
  mockSlottingSuggestions[index] = updated;
  return updated;
};

export const rejectMockWmsSlottingSuggestion = (suggestionId: string): WmsSlottingSuggestion => {
  const index = findIndexOrThrow(
    mockSlottingSuggestions,
    'suggestion_id',
    suggestionId,
    'slotting 建议不存在'
  );
  const updated = touch({
    ...mockSlottingSuggestions[index],
    status: 'REJECTED'
  });
  mockSlottingSuggestions[index] = updated;
  return updated;
};

export const listMockWmsReplenishmentTasks = (
  query: WmsReplenishmentTaskQuery
): WmsExecutionListResponse<WmsReplenishmentTask> => listByQuery(mockReplenishmentTasks, query);

export const claimMockWmsReplenishmentTask = (
  taskId: string,
  payload: { assignee: string }
): WmsReplenishmentTask => {
  const index = findIndexOrThrow(
    mockReplenishmentTasks,
    'task_id',
    taskId,
    '补货任务不存在'
  );
  const updated = touch({
    ...mockReplenishmentTasks[index],
    assignee: payload.assignee,
    status: 'IN_PROGRESS'
  });
  mockReplenishmentTasks[index] = updated;
  return updated;
};

export const completeMockWmsReplenishmentTask = (taskId: string): WmsReplenishmentTask => {
  const index = findIndexOrThrow(
    mockReplenishmentTasks,
    'task_id',
    taskId,
    '补货任务不存在'
  );
  const updated = touch({
    ...mockReplenishmentTasks[index],
    status: 'DONE'
  });
  mockReplenishmentTasks[index] = updated;
  return updated;
};

export const listMockWmsDockAppointments = (
  query: WmsDockAppointmentQuery
): WmsExecutionListResponse<WmsDockAppointment> => listByQuery(mockDockAppointments, query);

export const checkInMockWmsDockAppointment = (appointmentId: string): WmsDockAppointment => {
  const index = findIndexOrThrow(
    mockDockAppointments,
    'appointment_id',
    appointmentId,
    '月台预约不存在'
  );
  const updated = touch({
    ...mockDockAppointments[index],
    status: 'CHECKED_IN'
  });
  mockDockAppointments[index] = updated;
  return updated;
};

export const startMockWmsDockLoading = (appointmentId: string): WmsDockAppointment => {
  const index = findIndexOrThrow(
    mockDockAppointments,
    'appointment_id',
    appointmentId,
    '月台预约不存在'
  );
  const updated = touch({
    ...mockDockAppointments[index],
    status: 'LOADING'
  });
  mockDockAppointments[index] = updated;
  return updated;
};

export const departMockWmsDockAppointment = (appointmentId: string): WmsDockAppointment => {
  const index = findIndexOrThrow(
    mockDockAppointments,
    'appointment_id',
    appointmentId,
    '月台预约不存在'
  );
  const updated = touch({
    ...mockDockAppointments[index],
    status: 'DEPARTED'
  });
  mockDockAppointments[index] = updated;
  return updated;
};
