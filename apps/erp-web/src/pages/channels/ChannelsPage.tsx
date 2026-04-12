import {
  Button,
  Card,
  Checkbox,
  Col,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
  Input,
  Modal,
  Popconfirm,
  Row,
  Select,
  Space,
  Steps,
  Switch,
  Tag,
  Typography,
  message
} from 'antd';
import type { MenuProps, TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ReloadOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type Key,
  type MouseEvent as ReactMouseEvent
} from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Channel, ChannelCreatePayload, ChannelStatus } from '@/api/channels';
import { createChannel, deleteChannel, listChannels, updateChannel } from '@/api/channels';
import { listWarehouses } from '@/api/warehouses';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { PlatformTag } from '@/components/PlatformTag';
import {
  UiModule01QuickFiltersPanel,
  UiModule02ToolbarRight,
  UiModule03ContextMenu,
  UiModule03MainTableSection,
  UiModule03TableFooter,
  UiModule05DetailPanel
} from '@/components/ui-modules';
import type { PlatformCode } from '@/features/platform';
import { platformOptions } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

import './channelsPage.css';

type FormValues = {
  platform: PlatformCode;
  name: string;
  department?: string;
  owner?: string;
  allow_manual_order: boolean;
  allow_manual_after_sales: boolean;
  default_warehouse_id?: string;
  commission_formula?: string;
};

type PlatformPickerMode = 'create' | 'copy';

type ChannelStatusFilter = ChannelStatus | 'all';

type ManualFilter = 'allow' | 'deny';

type ChannelColumnFilters = {
  platform?: PlatformCode;
  department?: string;
  owner?: string;
  warehouseId?: string;
  allowManualOrder?: ManualFilter;
  allowManualAfterSales?: ManualFilter;
  updatedRange?: [dayjs.Dayjs, dayjs.Dayjs] | null;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const statusMeta: Record<ChannelStatus, { label: string; color: string }> = {
  ACTIVE: { label: '已启用', color: 'success' },
  DISABLED: { label: '已停用', color: 'default' }
};

const platformOrderLinks: Partial<Record<PlatformCode, string>> = {};
const platformAuthorizeLinks: Partial<Record<PlatformCode, string>> = {};

export const ChannelsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('channels.write');
  const queryClient = useQueryClient();
  const {
    setLeftPanel,
    setRightPanel,
    setBottomPanel,
    setLeftPanelVisible,
    setRightPanelVisible,
    setBottomPanelVisible,
    clearPanels
  } = useWorkbenchPanels();
  const [drawerForm] = Form.useForm<FormValues>();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [platformPickerOpen, setPlatformPickerOpen] = useState(false);
  const [platformPickerMode, setPlatformPickerMode] = useState<PlatformPickerMode>('create');
  const [selectedPlatform, setSelectedPlatform] = useState<PlatformCode | null>(null);
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);
  const [copySource, setCopySource] = useState<Channel | null>(null);
  const [saving, setSaving] = useState(false);
  const [statusFilter, setStatusFilter] = useState<ChannelStatusFilter>('ACTIVE');
  const [searchValue, setSearchValue] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [columnFilters, setColumnFilters] = useState<ChannelColumnFilters>({
    updatedRange: null
  });
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    channel: Channel;
  } | null>(null);
  const tableContainerRef = useRef<HTMLDivElement | null>(null);
  const lastSelectedIndexRef = useRef<number | null>(null);
  const pendingSelectionRef = useRef<Key[] | null>(null);
  const dragSelectionStartRef = useRef<{ x: number; y: number } | null>(null);
  const isDragSelectingRef = useRef(false);
  const suppressRowClickRef = useRef(false);
  const dragSelectionHandlersRef = useRef<{
    move: (event: MouseEvent) => void;
    up: (event: MouseEvent) => void;
  } | null>(null);
  const cellSelectionFrameRef = useRef<number | null>(null);
  const cellSelectionPendingRef = useRef<{
    startRow: number;
    endRow: number;
    startCol: number;
    endCol: number;
  } | null>(null);
  const cellSelectionStartRef = useRef<{ row: number; col: number } | null>(null);
  const cellSelectionAnchorRef = useRef<{ x: number; y: number } | null>(null);
  const cellSelectionActiveRef = useRef(false);
  const cellSelectionUserSelectRef = useRef<string | null>(null);
  const cellSelectionHandlersRef = useRef<{
    move: (event: MouseEvent) => void;
    up: (event: MouseEvent) => void;
  } | null>(null);
  const [cellSelection, setCellSelection] = useState<{
    startRow: number;
    endRow: number;
    startCol: number;
    endCol: number;
  } | null>(null);

  const openExternalLink = useCallback((url: string | undefined, fallbackMessage: string) => {
    if (!url) {
      message.info(fallbackMessage);
      return;
    }
    window.open(url, '_blank', 'noopener,noreferrer');
  }, []);

  const query = useMemo(
    () => ({
      status: statusFilter === 'all' ? undefined : statusFilter,
      keyword: searchKeyword.trim() || undefined,
      offset: 0,
      limit: 200
    }),
    [searchKeyword, statusFilter]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['channels', tenantId, query],
    queryFn: () => listChannels(tenantId, query),
    keepPreviousData: true
  });

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId],
    queryFn: () =>
      listWarehouses(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200
      }),
    enabled: Boolean(tenantId)
  });

  const warehouseMap = useMemo(() => {
    const map = new Map<string, string>();
    warehousesQuery.data?.items.forEach((warehouse) => {
      map.set(warehouse.warehouse_id, warehouse.name);
    });
    return map;
  }, [warehousesQuery.data?.items]);

  const warehouseOptions = useMemo(
    () =>
      warehousesQuery.data?.items.map((warehouse) => ({
        value: warehouse.warehouse_id,
        label: `${warehouse.name} (${warehouse.code})`
      })) ?? [],
    [warehousesQuery.data?.items]
  );

  const listItems = data?.items ?? [];

  const filteredItems = useMemo(() => {
    const keyword = searchKeyword.trim().toLowerCase();
    const departmentKeyword = columnFilters.department?.trim().toLowerCase();
    const ownerKeyword = columnFilters.owner?.trim().toLowerCase();
    return listItems.filter((item) => {
      if (statusFilter !== 'all' && item.status !== statusFilter) {
        return false;
      }
      if (keyword) {
        const name = item.name?.toLowerCase() ?? '';
        const department = item.department?.toLowerCase() ?? '';
        const owner = item.owner?.toLowerCase() ?? '';
        if (![name, department, owner].some((value) => value.includes(keyword))) {
          return false;
        }
      }
      if (columnFilters.platform && item.platform !== columnFilters.platform) {
        return false;
      }
      if (departmentKeyword && !(item.department ?? '').toLowerCase().includes(departmentKeyword)) {
        return false;
      }
      if (ownerKeyword && !(item.owner ?? '').toLowerCase().includes(ownerKeyword)) {
        return false;
      }
      if (
        columnFilters.warehouseId &&
        item.default_warehouse_id !== columnFilters.warehouseId
      ) {
        return false;
      }
      if (columnFilters.allowManualOrder) {
        const shouldAllow = columnFilters.allowManualOrder === 'allow';
        if (item.allow_manual_order !== shouldAllow) {
          return false;
        }
      }
      if (columnFilters.allowManualAfterSales) {
        const shouldAllow = columnFilters.allowManualAfterSales === 'allow';
        if (item.allow_manual_after_sales !== shouldAllow) {
          return false;
        }
      }
      if (columnFilters.updatedRange?.length) {
        const [start, end] = columnFilters.updatedRange;
        const target = dayjs(item.updated_at ?? item.created_at ?? '');
        if (!target.isValid()) {
          return false;
        }
        if (start && target.isBefore(start.startOf('day'))) {
          return false;
        }
        if (end && target.isAfter(end.endOf('day'))) {
          return false;
        }
      }
      return true;
    });
  }, [columnFilters, listItems, searchKeyword, statusFilter]);

  const pagedItems = useMemo(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    const start = (current - 1) * pageSize;
    return filteredItems.slice(start, start + pageSize);
  }, [filteredItems, pagination.current, pagination.pageSize]);

  const resolveChannelKey = useCallback((channel: Channel) => channel.channel_id, []);

  const syncSelection = useCallback(
    (keys: Key[], rows?: Channel[], active?: Channel | null) => {
      const uniqueKeys = Array.from(new Set(keys.map((key) => String(key))));
      setSelectedRowKeys(uniqueKeys);
      if (!uniqueKeys.length) {
        setSelectedChannel(null);
        lastSelectedIndexRef.current = null;
        return;
      }
      if (active) {
        setSelectedChannel(active);
      } else if (rows?.length) {
        setSelectedChannel(rows[0] ?? null);
      } else {
        setSelectedChannel((prev) => {
          if (prev && uniqueKeys.includes(prev.channel_id)) {
            return prev;
          }
          return (
            pagedItems.find((item) => uniqueKeys.includes(item.channel_id)) ?? null
          );
        });
      }
      if (uniqueKeys.length === pagedItems.length && pagedItems.length > 0) {
        lastSelectedIndexRef.current = pagedItems.length - 1;
      }
    },
    [pagedItems]
  );

  const currentPageKeys = useMemo(
    () => pagedItems.map((item) => item.channel_id),
    [pagedItems]
  );
  const selectionAllChecked =
    currentPageKeys.length > 0 &&
    currentPageKeys.every((key) => selectedRowKeys.includes(key));
  const selectionIndeterminate =
    currentPageKeys.some((key) => selectedRowKeys.includes(key)) && !selectionAllChecked;

  const handleSelectAllToggle = useCallback(
    (checked: boolean) => {
      if (checked) {
        syncSelection(currentPageKeys, pagedItems);
        return;
      }
      const remaining = selectedRowKeys.filter((key) => !currentPageKeys.includes(key));
      syncSelection(remaining);
    },
    [currentPageKeys, pagedItems, selectedRowKeys, syncSelection]
  );

  const handleSelectionHeaderMouseDown = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>) => {
      event.preventDefault();
      handleSelectAllToggle(!selectionAllChecked);
    },
    [handleSelectAllToggle, selectionAllChecked]
  );

  const selectionHeader = useMemo(
    () => (
      <div
        className="channels-selection-header"
        onMouseDownCapture={handleSelectionHeaderMouseDown}
      >
        <Checkbox checked={selectionAllChecked} indeterminate={selectionIndeterminate} />
      </div>
    ),
    [handleSelectionHeaderMouseDown, selectionAllChecked, selectionIndeterminate]
  );

  const isInteractiveTarget = useCallback((target: HTMLElement | null) => {
    return Boolean(
      target?.closest(
        'button, a, input, textarea, .ant-select, .ant-select-selector, .ant-input, .ant-input-affix-wrapper, .ant-input-number, .ant-picker, .ant-checkbox, .ant-checkbox-wrapper, .workbench-table-query'
      )
    );
  }, []);

  const stopDragSelectionTracking = useCallback(() => {
    const handlers = dragSelectionHandlersRef.current;
    if (!handlers) {
      return;
    }
    window.removeEventListener('mousemove', handlers.move);
    window.removeEventListener('mouseup', handlers.up);
    dragSelectionHandlersRef.current = null;
  }, []);

  const startDragSelectionTracking = useCallback(
    (event: ReactMouseEvent<HTMLElement>) => {
      if (event.button !== 0) {
        return;
      }
      const target = event.target as HTMLElement;
      if (isInteractiveTarget(target)) {
        return;
      }
      stopDragSelectionTracking();
      dragSelectionStartRef.current = { x: event.clientX, y: event.clientY };
      isDragSelectingRef.current = false;
      suppressRowClickRef.current = false;

      const handleMove = (moveEvent: MouseEvent) => {
        if (!dragSelectionStartRef.current || moveEvent.buttons !== 1) {
          return;
        }
        const dx = Math.abs(moveEvent.clientX - dragSelectionStartRef.current.x);
        const dy = Math.abs(moveEvent.clientY - dragSelectionStartRef.current.y);
        if (dx > 4 || dy > 4) {
          isDragSelectingRef.current = true;
          suppressRowClickRef.current = true;
        }
      };

      const handleUp = () => {
        dragSelectionStartRef.current = null;
        const selection = window.getSelection?.();
        const selectionActive = Boolean(selection && !selection.isCollapsed);
        if (selectionActive || isDragSelectingRef.current) {
          suppressRowClickRef.current = true;
          setTimeout(() => {
            isDragSelectingRef.current = false;
          }, 0);
          setTimeout(() => {
            suppressRowClickRef.current = false;
          }, 200);
        }
        stopDragSelectionTracking();
      };

      dragSelectionHandlersRef.current = { move: handleMove, up: handleUp };
      window.addEventListener('mousemove', handleMove);
      window.addEventListener('mouseup', handleUp);
    },
    [isInteractiveTarget, stopDragSelectionTracking]
  );

  useEffect(() => () => stopDragSelectionTracking(), [stopDragSelectionTracking]);

  const stopCellSelectionTracking = useCallback(() => {
    const handlers = cellSelectionHandlersRef.current;
    if (handlers) {
      window.removeEventListener('mousemove', handlers.move);
      window.removeEventListener('mouseup', handlers.up);
      cellSelectionHandlersRef.current = null;
    }
    if (cellSelectionFrameRef.current !== null) {
      window.cancelAnimationFrame(cellSelectionFrameRef.current);
      cellSelectionFrameRef.current = null;
    }
    cellSelectionPendingRef.current = null;
    cellSelectionActiveRef.current = false;
    cellSelectionStartRef.current = null;
    cellSelectionAnchorRef.current = null;
    if (cellSelectionUserSelectRef.current !== null) {
      document.body.style.userSelect = cellSelectionUserSelectRef.current;
      cellSelectionUserSelectRef.current = null;
    }
  }, []);

  const updateCellSelection = useCallback((rowIndex: number, colIndex: number) => {
    const start = cellSelectionStartRef.current;
    if (!start) {
      return;
    }
    const next = {
      startRow: Math.min(start.row, rowIndex),
      endRow: Math.max(start.row, rowIndex),
      startCol: Math.min(start.col, colIndex),
      endCol: Math.max(start.col, colIndex)
    };
    cellSelectionPendingRef.current = next;
    if (cellSelectionFrameRef.current !== null) {
      return;
    }
    cellSelectionFrameRef.current = window.requestAnimationFrame(() => {
      cellSelectionFrameRef.current = null;
      const pending = cellSelectionPendingRef.current;
      cellSelectionPendingRef.current = null;
      if (!pending) {
        return;
      }
      setCellSelection((prev) => {
        if (
          prev &&
          prev.startRow === pending.startRow &&
          prev.endRow === pending.endRow &&
          prev.startCol === pending.startCol &&
          prev.endCol === pending.endCol
        ) {
          return prev;
        }
        return pending;
      });
    });
  }, []);

  const handleCellSelectionMouseDown = useCallback(
    (event: MouseEvent | ReactMouseEvent<HTMLElement>) => {
      if (event.button !== 0) {
        return;
      }
      const target = event.target as HTMLElement;
      if (isInteractiveTarget(target)) {
        return;
      }
      if (!target.closest('.ant-table')) {
        return;
      }
      if (!target.closest('.ant-table-tbody')) {
        return;
      }
      if (target.closest('.workbench-table-query')) {
        return;
      }
      if (target.closest('thead')) {
        return;
      }
      const cell = target.closest('td') as HTMLTableCellElement | null;
      if (!cell) {
        return;
      }
      const row = cell.parentElement as HTMLTableRowElement | null;
      const body = tableContainerRef.current?.querySelector('.ant-table-tbody');
      if (!row || !body || !body.contains(row)) {
        return;
      }
      if (cell.classList.contains('ant-table-selection-column') || cell.cellIndex === 0) {
        return;
      }
      event.preventDefault();
      stopCellSelectionTracking();
      const rowIndex = row.sectionRowIndex;
      const colIndex = cell.cellIndex;
      cellSelectionStartRef.current = { row: rowIndex, col: colIndex };
      cellSelectionAnchorRef.current = { x: event.clientX, y: event.clientY };
      cellSelectionActiveRef.current = false;

      const handleMove = (moveEvent: MouseEvent) => {
        if (moveEvent.buttons !== 1) {
          return;
        }
        const start = cellSelectionStartRef.current;
        const anchor = cellSelectionAnchorRef.current;
        if (!start || !anchor) {
          return;
        }
        if (!cellSelectionActiveRef.current) {
          const dx = Math.abs(moveEvent.clientX - anchor.x);
          const dy = Math.abs(moveEvent.clientY - anchor.y);
          if (dx < 4 && dy < 4) {
            return;
          }
          cellSelectionActiveRef.current = true;
          suppressRowClickRef.current = true;
          cellSelectionUserSelectRef.current = document.body.style.userSelect;
          document.body.style.userSelect = 'none';
          updateCellSelection(start.row, start.col);
        }
        if (!cellSelectionActiveRef.current) {
          return;
        }
        const element = document.elementFromPoint(
          moveEvent.clientX,
          moveEvent.clientY
        ) as HTMLElement | null;
        const moveCell = element?.closest('td') as HTMLTableCellElement | null;
        if (!moveCell) {
          return;
        }
        const moveRow = moveCell.parentElement as HTMLTableRowElement | null;
        if (!moveRow || !body.contains(moveRow)) {
          return;
        }
        if (
          moveCell.classList.contains('ant-table-selection-column') ||
          moveCell.cellIndex === 0
        ) {
          return;
        }
        updateCellSelection(moveRow.sectionRowIndex, moveCell.cellIndex);
      };

      const handleUp = () => {
        const wasActive = cellSelectionActiveRef.current;
        stopCellSelectionTracking();
        if (wasActive) {
          suppressRowClickRef.current = true;
          setTimeout(() => {
            suppressRowClickRef.current = false;
          }, 200);
        }
      };

      cellSelectionHandlersRef.current = { move: handleMove, up: handleUp };
      window.addEventListener('mousemove', handleMove);
      window.addEventListener('mouseup', handleUp);
    },
    [isInteractiveTarget, stopCellSelectionTracking, updateCellSelection]
  );

  useEffect(() => () => stopCellSelectionTracking(), [stopCellSelectionTracking]);

  useEffect(() => {
    const body = tableContainerRef.current?.querySelector('.ant-table-tbody');
    if (!body) {
      return;
    }
    const handleMouseDown = (event: MouseEvent) => {
      handleCellSelectionMouseDown(event);
    };
    body.addEventListener('mousedown', handleMouseDown);
    return () => body.removeEventListener('mousedown', handleMouseDown);
  }, [handleCellSelectionMouseDown, pagedItems]);

  useEffect(() => {
    setCellSelection(null);
  }, [pagedItems]);

  useEffect(() => {
    const container = tableContainerRef.current;
    if (!container) {
      return;
    }
    const rows = Array.from(container.querySelectorAll('tbody tr'));
    rows.forEach((row) =>
      Array.from(row.querySelectorAll('td')).forEach((cell) =>
        cell.classList.remove('channels-cell-selected')
      )
    );
    if (!cellSelection) {
      return;
    }
    for (let rowIndex = cellSelection.startRow; rowIndex <= cellSelection.endRow; rowIndex += 1) {
      const row = rows[rowIndex] as HTMLTableRowElement | undefined;
      if (!row) {
        continue;
      }
      const cells = Array.from(row.querySelectorAll('td'));
      for (
        let colIndex = cellSelection.startCol;
        colIndex <= cellSelection.endCol;
        colIndex += 1
      ) {
        const cell = cells[colIndex];
        if (cell && !cell.classList.contains('ant-table-selection-column')) {
          cell.classList.add('channels-cell-selected');
        }
      }
    }
  }, [cellSelection]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!cellSelection) {
        return;
      }
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'c') {
        return;
      }
      const active = document.activeElement as HTMLElement | null;
      if (active && (active.tagName === 'INPUT' || active.tagName === 'TEXTAREA')) {
        return;
      }
      const container = tableContainerRef.current;
      if (!container) {
        return;
      }
      const rows = Array.from(container.querySelectorAll('tbody tr'));
      const lines: string[] = [];
      for (let rowIndex = cellSelection.startRow; rowIndex <= cellSelection.endRow; rowIndex += 1) {
        const row = rows[rowIndex] as HTMLTableRowElement | undefined;
        if (!row) {
          continue;
        }
        const cells = Array.from(row.querySelectorAll('td'));
        const rowValues: string[] = [];
        for (
          let colIndex = cellSelection.startCol;
          colIndex <= cellSelection.endCol;
          colIndex += 1
        ) {
          const cell = cells[colIndex];
          if (!cell || cell.classList.contains('ant-table-selection-column')) {
            continue;
          }
          rowValues.push(cell.innerText.trim());
        }
        if (rowValues.length) {
          lines.push(rowValues.join('\t'));
        }
      }
      if (!lines.length) {
        return;
      }
      const text = lines.join('\n');
      event.preventDefault();
      if (navigator.clipboard?.writeText) {
        navigator.clipboard.writeText(text).catch(() => {
          const textarea = document.createElement('textarea');
          textarea.value = text;
          textarea.style.position = 'fixed';
          textarea.style.opacity = '0';
          document.body.appendChild(textarea);
          textarea.select();
          document.execCommand('copy');
          document.body.removeChild(textarea);
        });
      } else {
        const textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand('copy');
        document.body.removeChild(textarea);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [cellSelection]);

  useEffect(() => {
    const maxPage = Math.max(
      1,
      Math.ceil(filteredItems.length / (pagination.pageSize ?? 20))
    );
    setPagination((prev) => ({
      ...prev,
      current: Math.min(prev.current ?? 1, maxPage)
    }));
  }, [filteredItems.length, pagination.pageSize]);

  useEffect(() => {
    if (!selectedRowKeys.length) {
      return;
    }
    setSelectedRowKeys((prev) =>
      prev.filter((key) => filteredItems.some((item) => item.channel_id === key))
    );
  }, [filteredItems, selectedRowKeys.length]);

  useEffect(() => {
    if (!selectedChannel) {
      return;
    }
    const exists = filteredItems.find(
      (item) => item.channel_id === selectedChannel.channel_id
    );
    if (!exists) {
      setSelectedChannel(null);
    }
  }, [filteredItems, selectedChannel]);

  const handleSearch = useCallback((value: string) => {
    const keyword = value.trim();
    setSearchKeyword(keyword);
    setSelectedChannel(null);
    setSelectedRowKeys([]);
    setPagination((prev) => ({ ...prev, current: 1 }));
  }, []);

  const handleSearchChange = useCallback((event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setSearchValue(value);
    if (!value.trim()) {
      setSearchKeyword('');
      setSelectedChannel(null);
      setSelectedRowKeys([]);
      setPagination((prev) => ({ ...prev, current: 1 }));
    }
  }, []);

  const updateColumnFilters = useCallback((patch: Partial<ChannelColumnFilters>) => {
    setColumnFilters((prev) => ({ ...prev, ...patch }));
    setSelectedChannel(null);
    setSelectedRowKeys([]);
    setPagination((prev) => ({ ...prev, current: 1 }));
  }, []);

  useEffect(() => {
    const trimmedKeyword = searchValue.trim();
    if (!trimmedKeyword || trimmedKeyword === searchKeyword) {
      return;
    }
    const timer = window.setTimeout(() => {
      setSearchKeyword(trimmedKeyword);
      setSelectedChannel(null);
      setSelectedRowKeys([]);
      setPagination((prev) => ({ ...prev, current: 1 }));
    }, 300);
    return () => window.clearTimeout(timer);
  }, [searchKeyword, searchValue]);

  const handleQuickFilterSelect = useCallback((filterId: string) => {
    const nextFilter = filterId === 'all' ? 'all' : (filterId as ChannelStatus);
    setStatusFilter(nextFilter);
    setSelectedChannel(null);
    setSelectedRowKeys([]);
    setPagination((prev) => ({ ...prev, current: 1 }));
  }, []);

  const quickFilterItems = useMemo(
    () => [
      { id: 'all', name: '全部渠道' },
      { id: 'ACTIVE', name: '启用渠道' },
      { id: 'DISABLED', name: '停用渠道' }
    ],
    []
  );

  const quickFilterPanel = useMemo(
    () => (
      <UiModule01QuickFiltersPanel
        filters={quickFilterItems}
        activeFilterId={statusFilter}
        onOpenSettings={() => message.info('快速查询设置功能建设中')}
        onSelectFilter={handleQuickFilterSelect}
      />
    ),
    [handleQuickFilterSelect, quickFilterItems, statusFilter]
  );

  const deleteMutation = useMutation({
    mutationFn: (channelId: string) => deleteChannel(tenantId, channelId),
    onSuccess: () => {
      message.success('渠道已删除');
      setSelectedChannel(null);
      queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除渠道失败');
    }
  });

  const statusMutation = useMutation({
    mutationFn: ({ channelId, status }: { channelId: string; status: ChannelStatus }) =>
      updateChannel(tenantId, channelId, { status }),
    onSuccess: (_, variables) => {
      message.success(variables.status === 'DISABLED' ? '渠道已停用' : '渠道已启用');
      queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '更新渠道状态失败');
    }
  });

  const handleOpenPlatformPicker = (mode: PlatformPickerMode) => {
    setPlatformPickerMode(mode);
    setSelectedPlatform(null);
    setPlatformPickerOpen(true);
  };

  const openCreate = () => {
    setEditingChannel(null);
    setCopySource(null);
    handleOpenPlatformPicker('create');
  };

  const openCopy = () => {
    if (!selectedChannel) {
      message.warning('请先选择渠道');
      return;
    }
    setEditingChannel(null);
    setCopySource(selectedChannel);
    handleOpenPlatformPicker('copy');
  };

  const openEditWithChannel = useCallback(
    (channel: Channel) => {
      setCopySource(null);
      setEditingChannel(channel);
      drawerForm.setFieldsValue({
        platform: channel.platform,
        name: channel.name,
        department: channel.department ?? undefined,
        owner: channel.owner ?? undefined,
        allow_manual_order: channel.allow_manual_order,
        allow_manual_after_sales: channel.allow_manual_after_sales,
        default_warehouse_id: channel.default_warehouse_id ?? undefined,
        commission_formula: channel.commission_formula ?? undefined
      });
      setDrawerOpen(true);
    },
    [drawerForm]
  );

  const openEdit = () => {
    if (!selectedChannel) {
      message.warning('请先选择渠道');
      return;
    }
    openEditWithChannel(selectedChannel);
  };

  const handlePlatformConfirm = () => {
    if (!selectedPlatform) {
      message.warning('请选择平台');
      return;
    }
    const base = platformPickerMode === 'copy' ? copySource : null;
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      platform: selectedPlatform,
      name: base ? `${base.name}-复制` : '',
      department: base?.department ?? undefined,
      owner: base?.owner ?? undefined,
      allow_manual_order: base?.allow_manual_order ?? false,
      allow_manual_after_sales: base?.allow_manual_after_sales ?? false,
      default_warehouse_id: base?.default_warehouse_id ?? undefined,
      commission_formula: base?.commission_formula ?? undefined
    });
    setPlatformPickerOpen(false);
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: ChannelCreatePayload = {
        platform: values.platform,
        name: values.name.trim(),
        department: values.department?.trim() || undefined,
        owner: values.owner?.trim() || undefined,
        allow_manual_order: values.allow_manual_order,
        allow_manual_after_sales: values.allow_manual_after_sales,
        default_warehouse_id: values.default_warehouse_id,
        commission_formula: values.commission_formula?.trim() || undefined
      };
      setSaving(true);
      if (editingChannel) {
        await updateChannel(tenantId, editingChannel.channel_id, payload);
        message.success('渠道已更新');
      } else {
        await createChannel(tenantId, payload);
        message.success('渠道已创建');
      }
      setDrawerOpen(false);
      setEditingChannel(null);
      setCopySource(null);
      await queryClient.invalidateQueries({ queryKey: ['channels', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const authFlowContent = useMemo(() => {
    if (!selectedChannel) {
      return <Typography.Text type="secondary">请选择渠道查看授权流程。</Typography.Text>;
    }
    const orderLink = platformOrderLinks[selectedChannel.platform];
    const authorizeLink = platformAuthorizeLinks[selectedChannel.platform];
    return (
      <Space direction="vertical" size={12} data-testid="channels-auth-flow">
        <Space size={8}>
          <Typography.Text type="secondary">当前平台：</Typography.Text>
          <PlatformTag platform={selectedChannel.platform} />
        </Space>
        <Steps
          size="small"
          direction="vertical"
          data-testid="channels-auth-steps"
          items={[
            {
              title: '去订购',
              description: (
                <Space direction="vertical" size={4}>
                  <Typography.Text type="secondary">跳转到平台订购软件。</Typography.Text>
                  <Button
                    size="small"
                    type="primary"
                    onClick={() => openExternalLink(orderLink, '平台订购入口待接入')}
                  >
                    去订购
                  </Button>
                </Space>
              )
            },
            {
              title: '去授权',
              description: (
                <Space direction="vertical" size={4}>
                  <Typography.Text type="secondary">授权软件访问店铺数据。</Typography.Text>
                  <Button
                    size="small"
                    type="primary"
                    onClick={() => openExternalLink(authorizeLink, '授权入口待接入')}
                  >
                    去授权
                  </Button>
                </Space>
              )
            },
            {
              title: '去返款',
              description: (
                <Space direction="vertical" size={4}>
                  <Typography.Text type="secondary">
                    用于店铺订购退款申请（待接入，计划新建 UI 申请）。
                  </Typography.Text>
                  <Button size="small" disabled>
                    去返款
                  </Button>
                </Space>
              )
            }
          ]}
        />
      </Space>
    );
  }, [openExternalLink, selectedChannel]);

  const detailPanels = useMemo(() => {
    const detailContent = selectedChannel ? (
      <Descriptions size="small" column={1}>
        <Descriptions.Item label="平台">
          <PlatformTag platform={selectedChannel.platform} />
        </Descriptions.Item>
        <Descriptions.Item label="渠道名称">{selectedChannel.name}</Descriptions.Item>
        <Descriptions.Item label="负责部门">
          {selectedChannel.department ?? '-'}
        </Descriptions.Item>
        <Descriptions.Item label="负责人">{selectedChannel.owner ?? '-'}</Descriptions.Item>
        <Descriptions.Item label="默认仓库">
          {selectedChannel.default_warehouse_id
            ? warehouseMap.get(selectedChannel.default_warehouse_id) ??
              selectedChannel.default_warehouse_id
            : '-'}
        </Descriptions.Item>
        <Descriptions.Item label="手工建单">
          {selectedChannel.allow_manual_order ? (
            <Tag color="green">允许</Tag>
          ) : (
            <Tag>不允许</Tag>
          )}
        </Descriptions.Item>
        <Descriptions.Item label="手工售后">
          {selectedChannel.allow_manual_after_sales ? (
            <Tag color="green">允许</Tag>
          ) : (
            <Tag>不允许</Tag>
          )}
        </Descriptions.Item>
        <Descriptions.Item label="抽佣公式">
          {selectedChannel.commission_formula ?? '-'}
        </Descriptions.Item>
        <Descriptions.Item label="状态">
          <Tag color={statusMeta[selectedChannel.status]?.color ?? 'default'}>
            {statusMeta[selectedChannel.status]?.label ?? selectedChannel.status}
          </Tag>
        </Descriptions.Item>
        <Descriptions.Item label="更新时间">
          {formatDateTime(selectedChannel.updated_at ?? selectedChannel.created_at)}
        </Descriptions.Item>
      </Descriptions>
    ) : (
      <Typography.Text type="secondary">请选择渠道查看详情。</Typography.Text>
    );

    return [
      { key: 'detail', label: '渠道详情', children: detailContent },
      { key: 'auth', label: '授权流程', children: authFlowContent }
    ];
  }, [authFlowContent, selectedChannel, warehouseMap]);

  const rightPanel = useMemo(
    () => (
      <UiModule05DetailPanel panels={detailPanels} defaultActiveKeys={['detail', 'auth']} />
    ),
    [detailPanels]
  );

  const columnQueryCells = useMemo(
    () => ({
      name: (
        <Input
          size="small"
          allowClear
          placeholder="渠道名称"
          value={searchValue}
          onChange={handleSearchChange}
          onPressEnter={(event) =>
            handleSearch((event.target as HTMLInputElement).value ?? '')
          }
        />
      ),
      platform: (
        <Select
          size="small"
          allowClear
          placeholder="平台"
          options={platformOptions}
          value={columnFilters.platform}
          onChange={(value) => updateColumnFilters({ platform: value ?? undefined })}
        />
      ),
      department: (
        <Input
          size="small"
          allowClear
          placeholder="负责部门"
          value={columnFilters.department}
          onChange={(event) => updateColumnFilters({ department: event.target.value })}
        />
      ),
      owner: (
        <Input
          size="small"
          allowClear
          placeholder="负责人"
          value={columnFilters.owner}
          onChange={(event) => updateColumnFilters({ owner: event.target.value })}
        />
      ),
      default_warehouse_id: (
        <Select
          size="small"
          allowClear
          placeholder="默认仓库"
          options={warehouseOptions}
          loading={warehousesQuery.isFetching}
          value={columnFilters.warehouseId}
          onChange={(value) => updateColumnFilters({ warehouseId: value ?? undefined })}
        />
      ),
      allow_manual_order: (
        <Select
          size="small"
          allowClear
          placeholder="手工建单"
          options={[
            { label: '允许', value: 'allow' },
            { label: '不允许', value: 'deny' }
          ]}
          value={columnFilters.allowManualOrder}
          onChange={(value) =>
            updateColumnFilters({ allowManualOrder: value ?? undefined })
          }
        />
      ),
      allow_manual_after_sales: (
        <Select
          size="small"
          allowClear
          placeholder="手工售后"
          options={[
            { label: '允许', value: 'allow' },
            { label: '不允许', value: 'deny' }
          ]}
          value={columnFilters.allowManualAfterSales}
          onChange={(value) =>
            updateColumnFilters({ allowManualAfterSales: value ?? undefined })
          }
        />
      ),
      status: (
        <Select
          size="small"
          allowClear
          placeholder="状态"
          options={[
            { label: '启用', value: 'ACTIVE' },
            { label: '停用', value: 'DISABLED' }
          ]}
          value={statusFilter === 'all' ? undefined : statusFilter}
          onChange={(value) => {
            setStatusFilter((value as ChannelStatus) ?? 'all');
            setSelectedChannel(null);
            setSelectedRowKeys([]);
            setPagination((prev) => ({ ...prev, current: 1 }));
          }}
        />
      ),
      updated_at: (
        <DatePicker.RangePicker
          size="small"
          allowClear
          value={columnFilters.updatedRange ?? undefined}
          onChange={(range) => updateColumnFilters({ updatedRange: range ?? null })}
        />
      )
    }),
    [
      columnFilters.allowManualAfterSales,
      columnFilters.allowManualOrder,
      columnFilters.department,
      columnFilters.owner,
      columnFilters.platform,
      columnFilters.updatedRange,
      columnFilters.warehouseId,
      handleSearch,
      handleSearchChange,
      searchValue,
      statusFilter,
      updateColumnFilters,
      warehouseOptions,
      warehousesQuery.isFetching
    ]
  );

  useEffect(() => {
    setLeftPanelVisible(true);
    setRightPanelVisible(true);
    setBottomPanelVisible(false);
    setLeftPanel(quickFilterPanel);
    setRightPanel(rightPanel);
    setBottomPanel(undefined);
    return () => clearPanels();
  }, [
    clearPanels,
    quickFilterPanel,
    rightPanel,
    setBottomPanel,
    setBottomPanelVisible,
    setLeftPanel,
    setLeftPanelVisible,
    setRightPanel,
    setRightPanelVisible
  ]);

  const contextMenuItems: MenuProps['items'] = [
    {
      key: 'toggle',
      label: contextMenu?.channel.status === 'DISABLED' ? '启用渠道' : '停用渠道',
      disabled: !canWrite
    }
  ];

  const columns = useMemo<ColumnsType<Channel>>(
    () => {
      const current = pagination.current ?? 1;
      const pageSize = pagination.pageSize ?? 20;
      return [
        {
          title: '序号',
          key: 'index',
          width: 72,
          align: 'center',
          render: (_: Channel, __: Channel, index: number) =>
            (current - 1) * pageSize + index + 1
        },
        {
          title: '渠道名称',
          dataIndex: 'name'
        },
      {
        title: '平台',
        dataIndex: 'platform',
        render: (_, record) => <PlatformTag platform={record.platform} />
      },
      {
        title: '负责部门',
        dataIndex: 'department',
        render: (value) => value ?? '-'
      },
      {
        title: '负责人',
        dataIndex: 'owner',
        render: (value) => value ?? '-'
      },
      {
        title: '默认仓库',
        dataIndex: 'default_warehouse_id',
        render: (value: string | null | undefined) =>
          value ? warehouseMap.get(value) ?? value : '-'
      },
      {
        title: '手工建单',
        dataIndex: 'allow_manual_order',
        render: (value: boolean) => (value ? <Tag color="green">允许</Tag> : <Tag>不允许</Tag>)
      },
      {
        title: '手工售后',
        dataIndex: 'allow_manual_after_sales',
        render: (value: boolean) => (value ? <Tag color="green">允许</Tag> : <Tag>不允许</Tag>)
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: ChannelStatus) => {
          const meta = statusMeta[value] ?? { label: value, color: 'default' };
          return <Tag color={meta.color}>{meta.label}</Tag>;
        }
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      }
      ];
    },
    [pagination.current, pagination.pageSize, warehouseMap]
  );

  const toolbarLeft = (
    <Space size={8}>
      <Button
        type="primary"
        onClick={openCreate}
        disabled={!canWrite}
        data-testid="channels-create"
      >
        新增
      </Button>
      <Button
        onClick={openCopy}
        disabled={!canWrite || !selectedChannel}
        data-testid="channels-copy"
      >
        复制
      </Button>
      <Button
        onClick={openEdit}
        disabled={!canWrite || !selectedChannel}
        data-testid="channels-edit"
      >
        修改
      </Button>
      <Popconfirm
        title="确认删除选中的渠道？"
        okText="删除"
        cancelText="取消"
        onConfirm={() => selectedChannel && deleteMutation.mutate(selectedChannel.channel_id)}
        disabled={!selectedChannel || !canWrite}
      >
        <Button danger disabled={!selectedChannel || !canWrite} data-testid="channels-delete">
          删除
        </Button>
      </Popconfirm>
      <Button icon={<ReloadOutlined />} onClick={() => refetch()}>
        刷新
      </Button>
    </Space>
  );

  const toolbarCenter = (
    <Input.Search
      allowClear
      value={searchValue}
      placeholder="搜索渠道名称/负责人"
      onSearch={handleSearch}
      onChange={handleSearchChange}
      style={{ width: 240 }}
      data-testid="channels-search"
    />
  );

  const toolbarRight = (
    <UiModule02ToolbarRight
      onExport={() => message.info('导出功能待接入')}
      onPrint={() => message.info('打印功能待接入')}
      onHelp={() => message.info('帮助功能待接入')}
    />
  );

  const tableFooterSyncKey = useMemo(
    () => `${columns.length}-${filteredItems.length}`,
    [columns.length, filteredItems.length]
  );

  const tableFooter = (
    <UiModule03TableFooter
      tableContainerRef={tableContainerRef}
      syncKey={tableFooterSyncKey}
      summary={`共 ${filteredItems.length} 条，已勾选 ${selectedRowKeys.length} 条`}
      total={filteredItems.length}
      pagination={pagination}
      onPaginationChange={setPagination}
    />
  );

  return (
    <>
      <UiModule03MainTableSection<Channel>
        tableContainerRef={tableContainerRef}
        tableClassName="channels-table"
        toolbarLeft={toolbarLeft}
        toolbarCenter={toolbarCenter}
        toolbarRight={toolbarRight}
        queryCells={columnQueryCells}
        columns={columns}
        data={pagedItems}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="渠道列表加载失败"
        emptyText="暂无渠道"
        total={filteredItems.length}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="channels.table.columns"
        rowKey="channel_id"
        enablePanels={false}
        contextMenuSelectsRow={false}
        rowSelection={{
          selectedRowKeys,
          columnTitle: selectionHeader,
          onChange: (keys, rows, info) => {
            if (info?.type === 'multiple' && pendingSelectionRef.current) {
              const pending = pendingSelectionRef.current;
              pendingSelectionRef.current = null;
              syncSelection(pending);
              return;
            }
            pendingSelectionRef.current = null;
            syncSelection(keys, rows as Channel[]);
          },
          onSelect: (record, _selected, _rows, event) => {
            const index = pagedItems.findIndex(
              (item) => resolveChannelKey(item) === resolveChannelKey(record)
            );
            if (index < 0) {
              return;
            }
            const mouseEvent = event as MouseEvent;
            if (mouseEvent?.shiftKey && lastSelectedIndexRef.current != null) {
              const start = Math.min(lastSelectedIndexRef.current, index);
              const end = Math.max(lastSelectedIndexRef.current, index);
              const rangeKeys = pagedItems
                .slice(start, end + 1)
                .map((item) => resolveChannelKey(item));
              const merged =
                mouseEvent.ctrlKey || mouseEvent.metaKey
                  ? Array.from(new Set([...selectedRowKeys, ...rangeKeys]))
                  : rangeKeys;
              pendingSelectionRef.current = merged;
              syncSelection(merged, undefined, record);
            }
            lastSelectedIndexRef.current = index;
          }
        }}
        rowClassName={(record) =>
          record.channel_id === selectedChannel?.channel_id ? 'ant-table-row-selected' : ''
        }
        onRow={(record, index) => ({
          onMouseDown: startDragSelectionTracking,
          onClick: (event) => {
            const target = event.target as HTMLElement;
            if (suppressRowClickRef.current || isDragSelectingRef.current) {
              suppressRowClickRef.current = false;
              isDragSelectingRef.current = false;
              dragSelectionStartRef.current = null;
              return;
            }
            if (isInteractiveTarget(target)) {
              setSelectedChannel(record);
              return;
            }
            const selection = window.getSelection?.();
            if (selection && !selection.isCollapsed) {
              suppressRowClickRef.current = false;
              isDragSelectingRef.current = false;
              dragSelectionStartRef.current = null;
              return;
            }
            const currentIndex =
              index ??
              pagedItems.findIndex(
                (item) => resolveChannelKey(item) === resolveChannelKey(record)
              );
            if (currentIndex < 0) {
              setSelectedChannel(record);
              return;
            }
            const key = resolveChannelKey(record);
            if (event.shiftKey && lastSelectedIndexRef.current != null) {
              const start = Math.min(lastSelectedIndexRef.current, currentIndex);
              const end = Math.max(lastSelectedIndexRef.current, currentIndex);
              const rangeKeys = pagedItems
                .slice(start, end + 1)
                .map((item) => resolveChannelKey(item));
              const merged =
                event.ctrlKey || event.metaKey
                  ? Array.from(new Set([...selectedRowKeys, ...rangeKeys]))
                  : rangeKeys;
              syncSelection(merged, undefined, record);
              lastSelectedIndexRef.current = currentIndex;
            } else if (event.ctrlKey || event.metaKey) {
              const keySet = new Set(selectedRowKeys);
              if (keySet.has(key)) {
                keySet.delete(key);
              } else {
                keySet.add(key);
              }
              syncSelection(Array.from(keySet), undefined, record);
              lastSelectedIndexRef.current = currentIndex;
            } else {
              syncSelection([key], [record], record);
              lastSelectedIndexRef.current = currentIndex;
            }
          },
          onDoubleClick: (event) => {
            if (!canWrite) {
              message.warning('暂无编辑权限');
              return;
            }
            const target = event.target as HTMLElement;
            if (isInteractiveTarget(target)) {
              return;
            }
            setSelectedChannel(record);
            openEditWithChannel(record);
          },
          onContextMenu: (event) => {
            event.preventDefault();
            const currentIndex =
              index ??
              pagedItems.findIndex(
                (item) => resolveChannelKey(item) === resolveChannelKey(record)
              );
            if (currentIndex >= 0) {
              lastSelectedIndexRef.current = currentIndex;
            }
            syncSelection([resolveChannelKey(record)], [record], record);
            setContextMenu({ x: event.clientX, y: event.clientY, channel: record });
          }
        })}
        tableFooter={tableFooter}
      />

      <UiModule03ContextMenu
        open={Boolean(contextMenu)}
        position={contextMenu}
        menu={{
          items: contextMenuItems,
          onClick: () => {
            if (!contextMenu || !canWrite) {
              return;
            }
            const nextStatus = contextMenu.channel.status === 'DISABLED' ? 'ACTIVE' : 'DISABLED';
            statusMutation.mutate({
              channelId: contextMenu.channel.channel_id,
              status: nextStatus
            });
            setContextMenu(null);
          }
        }}
        onClose={() => setContextMenu(null)}
      />

      <Modal
        title="选择支持的平台"
        open={platformPickerOpen}
        onCancel={() => setPlatformPickerOpen(false)}
        onOk={handlePlatformConfirm}
        okText="确定"
        cancelText="取消"
      >
        <Row gutter={[12, 12]}>
          {platformOptions.map((platform) => (
            <Col span={12} key={platform.value}>
              <Card
                hoverable
                onClick={() => setSelectedPlatform(platform.value as PlatformCode)}
                style={{
                  borderColor:
                    selectedPlatform === platform.value ? 'var(--ant-color-primary)' : undefined
                }}
              >
                <Space direction="vertical" size={4}>
                  <PlatformTag platform={platform.value as PlatformCode} />
                  <Typography.Text>{platform.label}</Typography.Text>
                </Space>
              </Card>
            </Col>
          ))}
        </Row>
      </Modal>

      <Drawer
        title={editingChannel ? '修改渠道' : copySource ? '复制渠道' : '新增渠道'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={560}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              保存
            </Button>
          </Space>
        }
      >
        <Form
          form={drawerForm}
          layout="vertical"
          initialValues={{ allow_manual_order: false, allow_manual_after_sales: false }}
        >
          <Form.Item
            name="platform"
            label="平台"
            rules={[{ required: true, message: '请选择平台' }]}
          >
            <Select options={platformOptions} disabled />
          </Form.Item>
          <Form.Item
            name="name"
            label="渠道名称"
            rules={[{ required: true, message: '请输入渠道名称' }]}
          >
            <Input placeholder="例如：淘宝旗舰店渠道" />
          </Form.Item>
          <Form.Item name="department" label="负责部门">
            <Input placeholder="例如：电商事业部" />
          </Form.Item>
          <Form.Item name="owner" label="负责人">
            <Input placeholder="例如：张三" />
          </Form.Item>
          <Form.Item name="allow_manual_order" label="是否允许手工建单" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item
            name="allow_manual_after_sales"
            label="是否允许手工新增售后订单"
            valuePropName="checked"
          >
            <Switch />
          </Form.Item>
          <Form.Item name="default_warehouse_id" label="默认发货仓库">
            <Select
              allowClear
              placeholder="选择默认发货仓库"
              options={warehouseOptions}
              loading={warehousesQuery.isLoading}
            />
          </Form.Item>
          <Form.Item name="commission_formula" label="销售平台抽佣公式">
            <Input.TextArea rows={3} placeholder="例如：销售额*0.05" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
