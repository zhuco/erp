import { Dropdown } from 'antd';
import type { MenuProps } from 'antd';
import { useEffect } from 'react';

type UiModule03ContextMenuProps = {
  open: boolean;
  position?: { x: number; y: number } | null;
  menu: MenuProps;
  onClose: () => void;
  className?: string;
};

export const UiModule03ContextMenu = ({
  open,
  position,
  menu,
  onClose,
  className
}: UiModule03ContextMenuProps) => {
  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    window.dispatchEvent(
      new CustomEvent('erp-context-menu-toggle', {
        detail: { open }
      })
    );
  }, [open]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest('.order-workbench-context-menu')) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      onClose();
    };
    window.addEventListener('mousedown', handleClickOutside, true);
    return () => window.removeEventListener('mousedown', handleClickOutside, true);
  }, [open, onClose]);

  return (
    <Dropdown
      open={open}
      menu={menu}
      popupRender={(menuNode) => (
        <div className={['order-workbench-context-menu', className].filter(Boolean).join(' ')}>
          {menuNode}
        </div>
      )}
      onOpenChange={(nextOpen) => {
        if (!nextOpen) {
          onClose();
        }
      }}
      trigger={[]}
      placement="bottomLeft"
      getPopupContainer={() => document.body}
      styles={{
        root:
          open && position
            ? {
                position: 'fixed',
                left: position.x,
                top: position.y,
                zIndex: 2000
              }
            : undefined
      }}
    >
      <span
        style={{
          position: 'fixed',
          left: position?.x ?? -9999,
          top: position?.y ?? -9999,
          width: 0,
          height: 0
        }}
      />
    </Dropdown>
  );
};
