import { Collapse } from 'antd';
import type { CollapseProps } from 'antd';

type UiModule05DetailPanelProps = {
  panels: CollapseProps['items'];
  defaultActiveKeys?: Array<string | number>;
};

const resolveDefaultActiveKeys = (panels: CollapseProps['items']) => {
  if (!panels) {
    return [];
  }
  return panels
    .map((panel) => panel?.key)
    .filter((key): key is string | number => key != null);
};

export const UiModule05DetailPanel = ({
  panels,
  defaultActiveKeys
}: UiModule05DetailPanelProps) => (
  <Collapse
    size="small"
    defaultActiveKey={defaultActiveKeys ?? resolveDefaultActiveKeys(panels)}
    items={panels}
  />
);
