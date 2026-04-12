import dayjs from 'dayjs';

import type {
  LogisticsInstallationAppointment,
  LogisticsInstallationAppointmentCreatePayload,
  LogisticsInstallationTask,
  LogisticsInstallationTaskCreatePayload,
  LogisticsInstallationTaskListQuery,
  LogisticsInstallationTaskListResponse,
  LogisticsInstallationTaskUpdatePayload
} from '../logisticsInstallations';
import { applyPagination } from './data';

const mockTasks: LogisticsInstallationTask[] = [
  {
    task_id: 'INS-1001',
    tenant_id: 'TENANT-DEFAULT',
    order_id: 'ORD-100230',
    customer_name: '张先生',
    address: '上海市浦东新区示例路 100 号',
    service_type: '送装一体',
    assigned_team: '华东安装组',
    status: 'SCHEDULED',
    created_at: dayjs().subtract(1, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString(),
    appointments: [
      {
        appointment_id: 'APP-1001',
        task_id: 'INS-1001',
        scheduled_at: dayjs().add(1, 'day').toISOString(),
        status: 'BOOKED',
        created_at: dayjs().subtract(1, 'day').toISOString()
      }
    ]
  },
  {
    task_id: 'INS-1002',
    tenant_id: 'TENANT-DEFAULT',
    order_id: 'ORD-100231',
    customer_name: '李女士',
    address: '深圳市南山区科技园 66 号',
    service_type: '安装服务',
    assigned_team: '华南服务组',
    status: 'IN_PROGRESS',
    created_at: dayjs().subtract(2, 'day').toISOString(),
    updated_at: dayjs().subtract(5, 'hour').toISOString(),
    appointments: [
      {
        appointment_id: 'APP-1002',
        task_id: 'INS-1002',
        scheduled_at: dayjs().subtract(1, 'day').toISOString(),
        status: 'DONE',
        created_at: dayjs().subtract(2, 'day').toISOString()
      },
      {
        appointment_id: 'APP-1003',
        task_id: 'INS-1002',
        scheduled_at: dayjs().add(1, 'day').toISOString(),
        status: 'BOOKED',
        created_at: dayjs().subtract(1, 'day').toISOString()
      }
    ]
  }
];

export const listMockInstallationTasks = (
  query: LogisticsInstallationTaskListQuery
): LogisticsInstallationTaskListResponse => {
  const filtered = mockTasks.filter((task) => {
    if (query.status && task.status !== query.status) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const createMockInstallationTask = (
  payload: LogisticsInstallationTaskCreatePayload
): LogisticsInstallationTask => {
  const now = dayjs().toISOString();
  const taskId = `INS-${Date.now()}`;
  const appointments: LogisticsInstallationAppointment[] = payload.scheduled_at
    ? [
        {
          appointment_id: `APP-${Date.now()}`,
          task_id: taskId,
          scheduled_at: payload.scheduled_at ?? now,
          status: 'BOOKED',
          created_at: now
        }
      ]
    : [];
  const record: LogisticsInstallationTask = {
    task_id: taskId,
    tenant_id: 'TENANT-DEFAULT',
    order_id: payload.order_id,
    customer_name: payload.customer_name,
    address: payload.address,
    service_type: payload.service_type,
    assigned_team: payload.assigned_team ?? null,
    status: 'SCHEDULED',
    created_at: now,
    updated_at: now,
    appointments
  };
  mockTasks.unshift(record);
  return record;
};

export const updateMockInstallationTask = (
  taskId: string,
  payload: LogisticsInstallationTaskUpdatePayload
): LogisticsInstallationTask => {
  const target = mockTasks.find((task) => task.task_id === taskId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: LogisticsInstallationTask = {
      task_id: taskId,
      tenant_id: 'TENANT-DEFAULT',
      order_id: 'ORD-UNKNOWN',
      customer_name: '未知客户',
      address: '-',
      service_type: '安装服务',
      assigned_team: payload.assigned_team ?? null,
      status: payload.status ?? 'SCHEDULED',
      remedy_ticket_id: payload.remedy_ticket_id ?? null,
      exception_note: payload.exception_note ?? null,
      created_at: now,
      updated_at: now,
      appointments: []
    };
    mockTasks.unshift(fallback);
    return fallback;
  }
  Object.assign(target, {
    assigned_team: payload.assigned_team ?? target.assigned_team,
    status: payload.status ?? target.status,
    remedy_ticket_id: payload.remedy_ticket_id ?? target.remedy_ticket_id,
    exception_note: payload.exception_note ?? target.exception_note,
    updated_at: now
  });
  return target;
};

export const addMockInstallationAppointment = (
  taskId: string,
  payload: LogisticsInstallationAppointmentCreatePayload
): LogisticsInstallationAppointment => {
  const target = mockTasks.find((task) => task.task_id === taskId);
  const now = dayjs().toISOString();
  const appointment: LogisticsInstallationAppointment = {
    appointment_id: `APP-${Date.now()}`,
    task_id: taskId,
    scheduled_at: payload.scheduled_at,
    status: payload.status ?? 'BOOKED',
    created_at: now
  };
  if (target) {
    target.appointments = [...(target.appointments ?? []), appointment];
    target.status = 'SCHEDULED';
    target.updated_at = now;
  } else {
    mockTasks.unshift({
      task_id: taskId,
      tenant_id: 'TENANT-DEFAULT',
      order_id: 'ORD-UNKNOWN',
      customer_name: '未知客户',
      address: '-',
      service_type: '安装服务',
      assigned_team: null,
      status: 'SCHEDULED',
      created_at: now,
      updated_at: now,
      appointments: [appointment]
    });
  }
  return appointment;
};
