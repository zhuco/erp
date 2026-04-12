import { afterEach, describe, expect, it, vi } from 'vitest';

import { mockTenantId } from './mocks/data';

const loadWmsExecutionApiWithMocks = async () => {
  vi.resetModules();
  vi.doMock('@/utils/env', async () => {
    const actual = await vi.importActual<typeof import('@/utils/env')>('@/utils/env');
    return {
      ...actual,
      useMocks: true,
    };
  });

  return import('./wmsExecution');
};

afterEach(() => {
  vi.doUnmock('@/utils/env');
  vi.resetModules();
});

describe('WMS 现场执行 mock API', () => {
  it('labor 任务可以按分配 -> 开始 -> 完成推进状态', async () => {
    const {
      listWmsLaborTasks,
      assignWmsLaborTask,
      startWmsLaborTask,
      completeWmsLaborTask,
    } = await loadWmsExecutionApiWithMocks();

    const initial = await listWmsLaborTasks(mockTenantId, { status: 'UNASSIGNED' });
    expect(initial.items.length).toBeGreaterThan(0);
    const target = initial.items[0];
    expect(target).toBeDefined();

    await assignWmsLaborTask(mockTenantId, target.task_id, { assignee: 'picker-a' });
    await startWmsLaborTask(mockTenantId, target.task_id);
    await completeWmsLaborTask(mockTenantId, target.task_id);

    const latest = await listWmsLaborTasks(mockTenantId, {});
    const updated = latest.items.find((item) => item.task_id === target.task_id);
    expect(updated?.status).toBe('DONE');
    expect(updated?.assignee).toBe('picker-a');
  });

  it('slotting 建议与月台预约动作会持久化到后续查询', async () => {
    const {
      listWmsSlottingSuggestions,
      applyWmsSlottingSuggestion,
      listWmsDockAppointments,
      checkInWmsDockAppointment,
      startWmsDockLoading,
      departWmsDockAppointment,
    } = await loadWmsExecutionApiWithMocks();

    const slottingInitial = await listWmsSlottingSuggestions(mockTenantId, { status: 'PENDING' });
    expect(slottingInitial.items.length).toBeGreaterThan(0);
    const slottingTarget = slottingInitial.items[0];
    expect(slottingTarget).toBeDefined();
    await applyWmsSlottingSuggestion(mockTenantId, slottingTarget.suggestion_id);

    const slottingLatest = await listWmsSlottingSuggestions(mockTenantId, {});
    const slottingUpdated = slottingLatest.items.find(
      (item) => item.suggestion_id === slottingTarget.suggestion_id
    );
    expect(slottingUpdated?.status).toBe('APPLIED');

    const dockInitial = await listWmsDockAppointments(mockTenantId, { status: 'SCHEDULED' });
    expect(dockInitial.items.length).toBeGreaterThan(0);
    const dockTarget = dockInitial.items[0];
    expect(dockTarget).toBeDefined();
    await checkInWmsDockAppointment(mockTenantId, dockTarget.appointment_id);
    await startWmsDockLoading(mockTenantId, dockTarget.appointment_id);
    await departWmsDockAppointment(mockTenantId, dockTarget.appointment_id);

    const dockLatest = await listWmsDockAppointments(mockTenantId, {});
    const dockUpdated = dockLatest.items.find(
      (item) => item.appointment_id === dockTarget.appointment_id
    );
    expect(dockUpdated?.status).toBe('DEPARTED');
  });
});
