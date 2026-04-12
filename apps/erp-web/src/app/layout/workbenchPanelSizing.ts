export const WORKBENCH_LEFT_PANEL_MIN_WIDTH = 160;
export const WORKBENCH_LEFT_PANEL_MAX_WIDTH = 420;
export const WORKBENCH_RIGHT_PANEL_MIN_WIDTH = 260;
export const WORKBENCH_RIGHT_PANEL_MAX_WIDTH = 520;
export const WORKBENCH_DEFAULT_LEFT_PANEL_WIDTH = 240;
export const WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH = 360;
export const WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT = 280;
export const WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT = 200;
export const WORKBENCH_MAIN_TOP_MIN_HEIGHT = 240;

export const clampWorkbenchPanelSize = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, value));
