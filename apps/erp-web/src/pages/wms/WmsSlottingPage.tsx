import { Button, Space, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';

import {
  applyWmsSlottingSuggestion,
  listWmsSlottingSuggestions,
  rejectWmsSlottingSuggestion,
  type WmsSlottingSuggestion
} from '@/api/wmsExecution';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';

const statusColorMap: Record<WmsSlottingSuggestion['status'], string> = {
  PENDING: 'gold',
  APPLIED: 'green',
  REJECTED: 'red'
};

const priorityColorMap: Record<WmsSlottingSuggestion['priority'], string> = {
  HIGH: 'red',
  MEDIUM: 'gold',
  LOW: 'blue'
};

export const WmsSlottingPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const [pendingSuggestionActions, setPendingSuggestionActions] = useState<
    Record<string, string>
  >({});

  const query = useQuery({
    queryKey: ['wms-slotting-suggestions', tenantId],
    queryFn: () => listWmsSlottingSuggestions(tenantId, { offset: 0, limit: 50 }),
    placeholderData: (previousData) => previousData
  });

  const applyMutation = useMutation({
    mutationFn: (suggestionId: string) => applyWmsSlottingSuggestion(tenantId, suggestionId),
    onSuccess: async () => {
      message.success('已应用库位优化建议');
      await queryClient.invalidateQueries({ queryKey: ['wms-slotting-suggestions', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '应用建议失败');
    }
  });

  const rejectMutation = useMutation({
    mutationFn: (suggestionId: string) => rejectWmsSlottingSuggestion(tenantId, suggestionId),
    onSuccess: async () => {
      message.success('已驳回建议');
      await queryClient.invalidateQueries({ queryKey: ['wms-slotting-suggestions', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '驳回建议失败');
    }
  });

  const runSuggestionAction = async (
    suggestionId: string,
    action: 'apply' | 'reject',
    execute: () => Promise<unknown>
  ) => {
    if (pendingSuggestionActions[suggestionId]) {
      return;
    }
    setPendingSuggestionActions((prev) => ({ ...prev, [suggestionId]: action }));
    try {
      await execute();
    } finally {
      setPendingSuggestionActions((prev) => {
        const next = { ...prev };
        delete next[suggestionId];
        return next;
      });
    }
  };

  const columns: ColumnsType<WmsSlottingSuggestion> = [
    { title: '建议ID', dataIndex: 'suggestion_id' },
    { title: 'SKU', dataIndex: 'sku_id' },
    { title: '原库位', dataIndex: 'from_location_id' },
    { title: '目标库位', dataIndex: 'to_location_id' },
    { title: '原因', dataIndex: 'reason' },
    {
      title: '优先级',
      dataIndex: 'priority',
      render: (value: WmsSlottingSuggestion['priority']) => (
        <Tag color={priorityColorMap[value] ?? 'default'}>{value}</Tag>
      )
    },
    {
      title: '状态',
      dataIndex: 'status',
      render: (value: WmsSlottingSuggestion['status']) => (
        <Tag color={statusColorMap[value] ?? 'default'}>{value}</Tag>
      )
    },
    {
      title: '操作',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Button
            size="small"
            data-testid="wms-slotting-apply"
            disabled={
              record.status !== 'PENDING' || Boolean(pendingSuggestionActions[record.suggestion_id])
            }
            loading={pendingSuggestionActions[record.suggestion_id] === 'apply'}
            onClick={() =>
              void runSuggestionAction(record.suggestion_id, 'apply', () =>
                applyMutation.mutateAsync(record.suggestion_id)
              )
            }
          >
            应用
          </Button>
          <Button
            size="small"
            danger
            disabled={
              record.status !== 'PENDING' || Boolean(pendingSuggestionActions[record.suggestion_id])
            }
            loading={pendingSuggestionActions[record.suggestion_id] === 'reject'}
            onClick={() =>
              void runSuggestionAction(record.suggestion_id, 'reject', () =>
                rejectMutation.mutateAsync(record.suggestion_id)
              )
            }
          >
            驳回
          </Button>
        </Space>
      )
    }
  ];

  const tableData = query.data?.items ?? [];

  return (
    <UiModule03MainTableSection<WmsSlottingSuggestion>
      toolbarLeft={
        <Typography.Title level={3} data-testid="wms-slotting-title" style={{ margin: 0 }}>
          Slotting 库位优化建议
        </Typography.Title>
      }
      rowKey="suggestion_id"
      columns={columns}
      data={tableData}
      loading={query.isLoading || query.isFetching}
      error={query.error}
      onRetry={() => query.refetch()}
      errorTitle="slotting 建议加载失败"
      emptyText="暂无库位优化建议"
      total={tableData.length}
      pagination={{ current: 1, pageSize: 50 }}
      onPaginationChange={() => undefined}
      dataTestId="wms-slotting-main-grid"
      gridPreset="industrial-review-compact"
      selectionCellMode="custom"
      selectionHeaderMode="allInvert"
      showColumnSettings
      columnSettingsKey="wms.slotting.table.columns"
    />
  );
};
