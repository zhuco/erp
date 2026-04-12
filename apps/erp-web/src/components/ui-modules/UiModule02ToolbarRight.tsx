import { Button, Space } from 'antd';

type UiModule02ToolbarRightProps = {
  canExport?: boolean;
  exporting?: boolean;
  onExport?: () => void;
  onPrint?: () => void;
  onHelp?: () => void;
  exportButtonTestId?: string;
  printButtonTestId?: string;
  helpButtonTestId?: string;
};

export const UiModule02ToolbarRight = ({
  canExport = true,
  exporting = false,
  onExport,
  onPrint,
  onHelp,
  exportButtonTestId,
  printButtonTestId,
  helpButtonTestId,
}: UiModule02ToolbarRightProps) => (
  <Space size={8}>
    <Button
      size="small"
      onClick={onExport}
      loading={exporting}
      disabled={!canExport || !onExport}
      data-testid={exportButtonTestId}
    >
      导出
    </Button>
    <Button size="small" onClick={onPrint} disabled={!onPrint} data-testid={printButtonTestId}>
      打印
    </Button>
    <Button size="small" onClick={onHelp} disabled={!onHelp} data-testid={helpButtonTestId}>
      帮助
    </Button>
  </Space>
);
