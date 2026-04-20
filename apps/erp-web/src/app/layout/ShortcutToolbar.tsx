import { MenuOutlined } from '@ant-design/icons';
import {
  Alert,
  Button,
  Card,
  Col,
  Empty,
  Input,
  Modal,
  Row,
  Space,
  Tag,
  Tree,
  Typography,
  message,
} from 'antd';
import type { TreeDataNode, TreeProps } from 'antd/es/tree';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { KeyboardEvent as ReactKeyboardEvent } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Key } from 'react';
import { useNavigate } from 'react-router-dom';

import type { UserShortcut } from '@/api/shortcuts';
import { getUserShortcuts, updateUserShortcuts } from '@/api/shortcuts';
import type { AppMenuItem, AppMenuLeaf } from '@/app/layout/menu';
import { flattenMenuItems } from '@/app/layout/menu';
import { useFocusReturn } from '@/app/layout/useFocusReturn';
import { isTenantSelected, useTenantId } from '@/features/tenant';

import './ShortcutToolbar.css';

type ShortcutToolbarProps = {
  menuItems: AppMenuItem[];
};

const buildShortcut = (item: AppMenuLeaf): UserShortcut => ({
  path: item.path,
  name: item.name,
});

const getMenuKey = (item: AppMenuItem, parentKey: string, index: number) => {
  if (item.path) {
    return item.path;
  }
  const name = typeof item.name === 'string' && item.name ? item.name : 'group';
  return `${parentKey}-${name}-${index}`;
};

const getMenuLabel = (item: AppMenuItem) => {
  if (typeof item.name === 'string' && item.name.trim()) {
    return item.name;
  }
  if (item.path) {
    return item.path;
  }
  return '未命名菜单';
};

const filterMenuItems = (items: AppMenuItem[], keyword: string): AppMenuItem[] => {
  if (!keyword) {
    return items;
  }
  return items
    .map((item) => {
      const name = typeof item.name === 'string' ? item.name : '';
      const path = item.path ?? '';
      const children = item.children
        ? filterMenuItems(item.children as AppMenuItem[], keyword)
        : [];
      const matched =
        name.toLowerCase().includes(keyword) || path.toLowerCase().includes(keyword);
      if (!matched && children.length === 0) {
        return null;
      }
      return {
        ...item,
        children: children.length ? children : undefined,
      } as AppMenuItem;
    })
    .filter((item): item is AppMenuItem => Boolean(item));
};

const buildTreeData = (items: AppMenuItem[], parentKey = 'menu'): TreeDataNode[] => {
  return items.map((item, index) => {
    const children = (item.children as AppMenuItem[] | undefined) ?? [];
    const hasChildren = children.length > 0;
    const key = getMenuKey(item, parentKey, index);
    const label = getMenuLabel(item);
    return {
      key,
      title: (
        <Space size={6} wrap>
          <span>{label}</span>
          {item.path ? <Tag>{item.path}</Tag> : null}
        </Space>
      ),
      children: hasChildren ? buildTreeData(children, key) : undefined,
      selectable: false,
      disableCheckbox: !item.path,
      isLeaf: !hasChildren,
    };
  });
};

const collectExpandedKeys = (nodes: TreeDataNode[]): string[] => {
  return nodes.flatMap((node) => {
    if (!node.children || node.children.length === 0) {
      return [];
    }
    return [String(node.key), ...collectExpandedKeys(node.children)];
  });
};

export const ShortcutToolbar = ({ menuItems }: ShortcutToolbarProps) => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [modalOpen, setModalOpen] = useState(false);
  const [searchText, setSearchText] = useState('');
  const [draftPaths, setDraftPaths] = useState<string[]>([]);
  const [activeDraftIndex, setActiveDraftIndex] = useState(0);
  const [clearUndoSnapshot, setClearUndoSnapshot] = useState<string[] | null>(null);
  const dragIndexRef = useRef<number | null>(null);
  const clearUndoTimerRef = useRef<number | null>(null);

  useFocusReturn(modalOpen);

  const availableItems = useMemo(() => flattenMenuItems(menuItems), [menuItems]);

  const menuMap = useMemo(() => {
    return new Map(availableItems.map((item) => [item.path, item]));
  }, [availableItems]);

  const shortcutsQuery = useQuery({
    queryKey: ['shortcuts', tenantId],
    queryFn: () => getUserShortcuts(tenantId),
    enabled: tenantReady,
    retry: false,
  });

  const resolvedShortcuts = useMemo(() => {
    const stored = shortcutsQuery.data?.shortcuts ?? [];
    return stored
      .map((item) => menuMap.get(item.path) ?? item)
      .filter((item): item is AppMenuLeaf | UserShortcut => Boolean(item));
  }, [menuMap, shortcutsQuery.data?.shortcuts]);

  const currentPaths = useMemo(() => {
    return resolvedShortcuts.map((item) => item.path);
  }, [resolvedShortcuts]);

  const clearPendingUndo = useCallback(() => {
    if (clearUndoTimerRef.current != null) {
      window.clearTimeout(clearUndoTimerRef.current);
      clearUndoTimerRef.current = null;
    }
    setClearUndoSnapshot(null);
  }, []);

  useEffect(() => () => clearPendingUndo(), [clearPendingUndo]);

  const updateMutation = useMutation({
    mutationFn: (shortcuts: UserShortcut[]) => updateUserShortcuts(tenantId, shortcuts),
    onSuccess: (data) => {
      queryClient.setQueryData(['shortcuts', tenantId], data);
      clearPendingUndo();
      setModalOpen(false);
      message.success('快捷入口已更新');
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '更新快捷入口失败');
    },
  });

  const openModal = () => {
    if (!tenantReady) {
      message.error('请先选择商家');
      return;
    }
    clearPendingUndo();
    setDraftPaths(currentPaths);
    setActiveDraftIndex(0);
    setSearchText('');
    setModalOpen(true);
  };

  const normalizedKeyword = useMemo(() => searchText.trim().toLowerCase(), [searchText]);

  const filteredMenuItems = useMemo(
    () => filterMenuItems(menuItems, normalizedKeyword),
    [menuItems, normalizedKeyword],
  );

  const treeData = useMemo(() => buildTreeData(filteredMenuItems), [filteredMenuItems]);

  const [expandedKeys, setExpandedKeys] = useState<Key[]>([]);
  const expandedKeysRef = useRef<Key[]>([]);

  useEffect(() => {
    if (normalizedKeyword) {
      setExpandedKeys(collectExpandedKeys(treeData));
      return;
    }
    setExpandedKeys(expandedKeysRef.current);
  }, [normalizedKeyword, treeData]);

  useEffect(() => {
    setActiveDraftIndex((current) => {
      if (draftPaths.length === 0) {
        return 0;
      }
      return Math.max(0, Math.min(current, draftPaths.length - 1));
    });
  }, [draftPaths.length]);

  const handleTreeCheck: TreeProps['onCheck'] = (checkedKeysValue) => {
    const keys = Array.isArray(checkedKeysValue)
      ? checkedKeysValue
      : checkedKeysValue.checked;
    const nextKeys = keys.filter(
      (key): key is string => typeof key === 'string' && menuMap.has(key),
    );
    setDraftPaths((prev) => {
      const next = prev.filter((path) => nextKeys.includes(path));
      nextKeys.forEach((path) => {
        if (!next.includes(path)) {
          next.push(path);
        }
      });
      return next;
    });
  };

  const handleTreeExpand = (keys: Key[]) => {
    expandedKeysRef.current = keys;
    setExpandedKeys(keys);
  };

  const handleDragStart = (index: number) => {
    dragIndexRef.current = index;
  };

  const handleDrop = (index: number) => {
    const from = dragIndexRef.current;
    if (from == null || from === index) {
      dragIndexRef.current = null;
      return;
    }
    setDraftPaths((prev) => {
      const next = [...prev];
      const [moved] = next.splice(from, 1);
      next.splice(index, 0, moved);
      return next;
    });
    setActiveDraftIndex(index);
    dragIndexRef.current = null;
  };

  const removeDraftPathAt = useCallback((index: number) => {
    setDraftPaths((prev) => prev.filter((_, currentIndex) => currentIndex !== index));
    setActiveDraftIndex((current) => Math.max(0, current > index ? current - 1 : current));
  }, []);

  const handleRemove = (path: string) => {
    const index = draftPaths.findIndex((item) => item === path);
    if (index < 0) {
      return;
    }
    removeDraftPathAt(index);
  };

  const moveDraftPath = useCallback(
    (direction: -1 | 1) => {
      setDraftPaths((prev) => {
        if (prev.length <= 1) {
          return prev;
        }
        const fromIndex = Math.max(0, Math.min(activeDraftIndex, prev.length - 1));
        const toIndex = fromIndex + direction;
        if (toIndex < 0 || toIndex >= prev.length) {
          return prev;
        }
        const next = [...prev];
        const [moved] = next.splice(fromIndex, 1);
        next.splice(toIndex, 0, moved);
        setActiveDraftIndex(toIndex);
        return next;
      });
    },
    [activeDraftIndex],
  );

  const handleDraftListKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      if (draftPaths.length === 0) {
        return;
      }
      if (event.altKey && !event.ctrlKey && !event.metaKey) {
        if (event.key === 'ArrowUp') {
          event.preventDefault();
          moveDraftPath(-1);
          return;
        }
        if (event.key === 'ArrowDown') {
          event.preventDefault();
          moveDraftPath(1);
          return;
        }
      }

      if (event.key === 'ArrowUp') {
        event.preventDefault();
        setActiveDraftIndex((current) => Math.max(0, current - 1));
        return;
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        setActiveDraftIndex((current) => Math.min(draftPaths.length - 1, current + 1));
        return;
      }
      if (event.key === 'Delete' || event.key === 'Backspace') {
        event.preventDefault();
        removeDraftPathAt(activeDraftIndex);
      }
    },
    [activeDraftIndex, draftPaths.length, moveDraftPath, removeDraftPathAt],
  );

  const handleClear = () => {
    if (!draftPaths.length) {
      return;
    }
    setClearUndoSnapshot(draftPaths);
    setDraftPaths([]);
    setActiveDraftIndex(0);
    if (clearUndoTimerRef.current != null) {
      window.clearTimeout(clearUndoTimerRef.current);
    }
    clearUndoTimerRef.current = window.setTimeout(() => {
      clearUndoTimerRef.current = null;
      setClearUndoSnapshot(null);
    }, 8000);
  };

  const handleUndoClear = () => {
    if (!clearUndoSnapshot) {
      return;
    }
    setDraftPaths(clearUndoSnapshot);
    setActiveDraftIndex(0);
    clearPendingUndo();
  };

  const handleSave = () => {
    clearPendingUndo();
    const shortcuts = draftPaths
      .map((path) => {
        const item = menuMap.get(path);
        return item ? buildShortcut(item) : null;
      })
      .filter((item): item is UserShortcut => Boolean(item));
    updateMutation.mutate(shortcuts);
  };

  const toolbarItems = resolvedShortcuts.map((item) => ({
    path: item.path,
    name: 'name' in item && item.name ? item.name : item.path,
  }));

  return (
    <>
      <div className="shortcut-toolbar">
        <Typography.Text strong className="shortcut-toolbar-title">
          快捷工具栏
        </Typography.Text>
        <div className="shortcut-toolbar-scroller">
          {toolbarItems.length ? (
            <Space size={8} className="shortcut-toolbar-items">
              {toolbarItems.map((item) => (
                <Button
                  key={item.path}
                  size="small"
                  onClick={() => navigate(item.path)}
                  className="shortcut-toolbar-link"
                  data-testid={`shortcut-${item.path}`}
                >
                  {item.name}
                </Button>
              ))}
            </Space>
          ) : (
            <Typography.Text type="secondary" className="shortcut-toolbar-empty">
              暂无快捷入口
            </Typography.Text>
          )}
        </div>
        <Button size="small" onClick={openModal} className="shortcut-toolbar-action">
          设置
        </Button>
      </div>
      <Modal
        width={960}
        title="工具栏菜单设置"
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSave}
        okText="保存"
        cancelText="取消"
        confirmLoading={updateMutation.isPending}
        destroyOnHidden
      >
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          {clearUndoSnapshot ? (
            <Alert
              type="warning"
              showIcon
              message="已清空快捷入口，可在 8 秒内撤销"
              action={
                <Button type="link" size="small" onClick={handleUndoClear}>
                  撤销
                </Button>
              }
            />
          ) : null}
          <Typography.Text type="secondary">
            左侧按菜单顺序展示，右侧支持拖拽排序、键盘重排（Alt+↑/↓）和快速移除（Delete）。
          </Typography.Text>
          <Row gutter={16}>
            <Col span={12}>
              <Card
                size="small"
                title={
                  <Space size={6}>
                    <span>菜单列表</span>
                    <Typography.Text type="secondary">（勾选菜单，可快速设置）</Typography.Text>
                  </Space>
                }
                extra={<Typography.Text type="secondary">共 {availableItems.length} 项</Typography.Text>}
              >
                <Space direction="vertical" size={12} style={{ width: '100%' }}>
                  <Input.Search
                    placeholder="搜索功能名称或路径"
                    value={searchText}
                    onChange={(event) => setSearchText(event.target.value)}
                    allowClear
                  />
                  <div style={{ maxHeight: 420, overflowY: 'auto', paddingRight: 8 }}>
                    {treeData.length ? (
                      <Tree
                        checkable
                        checkStrictly
                        blockNode
                        showLine={{ showLeafIcon: false }}
                        autoExpandParent
                        treeData={treeData}
                        expandedKeys={expandedKeys}
                        onExpand={handleTreeExpand}
                        checkedKeys={{ checked: draftPaths, halfChecked: [] }}
                        onCheck={handleTreeCheck}
                      />
                    ) : (
                      <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无匹配功能" />
                    )}
                  </div>
                </Space>
              </Card>
            </Col>
            <Col span={12}>
              <Card
                size="small"
                title={
                  <Space size={6}>
                    <span>已选工具栏菜单</span>
                    <Typography.Text type="secondary">（支持拖拽或键盘重排）</Typography.Text>
                  </Space>
                }
                extra={
                  <Space size={8}>
                    <Typography.Text type="secondary">
                      已选 {draftPaths.length}/{availableItems.length}
                    </Typography.Text>
                    <Button size="small" onClick={handleClear} disabled={!draftPaths.length}>
                      清空
                    </Button>
                  </Space>
                }
              >
                <div
                  style={{ maxHeight: 420, overflowY: 'auto', paddingRight: 8 }}
                  role="listbox"
                  tabIndex={0}
                  aria-label="已选快捷入口列表"
                  onKeyDown={handleDraftListKeyDown}
                >
                  <Space direction="vertical" style={{ width: '100%' }} size={8}>
                    {draftPaths.map((path, index) => {
                      const item = menuMap.get(path);
                      const label = item?.name ?? path;
                      const active = index === activeDraftIndex;
                      return (
                        <div
                          key={path}
                          role="option"
                          aria-selected={active}
                          tabIndex={0}
                          draggable
                          onFocus={() => setActiveDraftIndex(index)}
                          onClick={() => setActiveDraftIndex(index)}
                          onDragStart={() => handleDragStart(index)}
                          onDragOver={(event) => event.preventDefault()}
                          onDrop={() => handleDrop(index)}
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: 12,
                            padding: '8px 12px',
                            border: active ? '1px solid #1677ff' : '1px solid #f0f0f0',
                            borderRadius: 6,
                            background: active ? '#e6f4ff' : '#fafafa',
                          }}
                        >
                          <MenuOutlined style={{ color: '#999' }} />
                          <Typography.Text style={{ flex: 1 }}>{label}</Typography.Text>
                          <Tag color="blue">{path}</Tag>
                          <Button size="small" type="text" onClick={() => handleRemove(path)}>
                            移除
                          </Button>
                        </div>
                      );
                    })}
                    {draftPaths.length === 0 ? (
                      <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="请先选择快捷入口" />
                    ) : null}
                  </Space>
                </div>
              </Card>
            </Col>
          </Row>
        </Space>
      </Modal>
    </>
  );
};
