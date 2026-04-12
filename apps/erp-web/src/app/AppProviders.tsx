import { ConfigProvider, theme } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useEffect, useMemo } from 'react';

import { RequestErrorToast } from '@/components/RequestErrorToast';
import { FeatureGateProvider } from '@/features/featureGate';
import { TenantProvider } from '@/features/tenant';
import { useUiSettingsStore } from '@/features/uiSettings';

import { queryClient } from './queryClient';

export const AppProviders = ({ children }: { children: ReactNode }) => {
  const themeMode = useUiSettingsStore((state) => state.themeMode);
  const denseMode = useUiSettingsStore((state) => state.denseMode);

  useEffect(() => {
    document.body.classList.toggle('erp-dense', denseMode);
  }, [denseMode]);

  useEffect(() => {
    document.body.dataset.theme = themeMode;
  }, [themeMode]);

  const themeTokens = useMemo(
    () => {
      const baseTokens = {
        fontSize: denseMode ? 12 : 14,
        lineHeight: denseMode ? 1.4 : 1.6,
        borderRadius: denseMode ? 6 : 8,
        padding: denseMode ? 8 : 12,
        paddingLG: denseMode ? 16 : 24,
        paddingSM: denseMode ? 6 : 8,
        controlHeight: denseMode ? 28 : 32,
        controlHeightSM: denseMode ? 24 : 28,
        controlHeightLG: denseMode ? 36 : 40
      };

      if (themeMode === 'dark') {
        return {
          ...baseTokens,
          colorPrimary: '#4B78A6',
          colorPrimaryHover: '#5B8CC0',
          colorPrimaryActive: '#5B8CC0',
          colorInfo: '#4B78A6',
          colorLink: '#6FA8DC',
          colorLinkHover: '#5B8CC0',
          colorLinkActive: '#5B8CC0',
          colorSuccess: '#45B8A6',
          colorWarning: '#D6B44C',
          colorError: '#D06A6A',
          colorText: '#E6EBF0',
          colorTextSecondary: '#A9B4C2',
          colorTextTertiary: '#6F7680',
          colorTextQuaternary: '#6F7680',
          colorTextDisabled: '#6F7680',
          colorBorder: '#2A3441',
          colorBorderSecondary: '#2A3441',
          colorBgLayout: '#0F1722',
          colorBgContainer: '#182230',
          colorBgElevated: '#182230',
          colorBgContainerDisabled: '#2A2F36',
          colorFillAlter: '#141E2B'
        };
      }

      return {
        ...baseTokens,
        colorPrimary: '#1F3A5F',
        colorPrimaryHover: '#274C77',
        colorPrimaryActive: '#274C77',
        colorInfo: '#1F3A5F',
        colorLink: '#2E6F95',
        colorLinkHover: '#274C77',
        colorLinkActive: '#274C77',
        colorSuccess: '#3FA796',
        colorWarning: '#C9A227',
        colorError: '#C94A4A',
        colorText: '#2B2F33',
        colorTextSecondary: '#5F6B7A',
        colorTextTertiary: '#8A9BA8',
        colorTextQuaternary: '#A0A7AF',
        colorTextDisabled: '#A0A7AF',
        colorBorder: '#8A9BA8',
        colorBorderSecondary: '#8A9BA8',
        colorBgLayout: '#F5F7FA',
        colorBgContainer: '#FFFFFF',
        colorBgElevated: '#FFFFFF',
        colorBgContainerDisabled: '#E1E6EB',
        colorFillAlter: '#EEF2F6'
      };
    },
    [denseMode, themeMode]
  );

  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: themeMode === 'dark' ? theme.darkAlgorithm : theme.defaultAlgorithm,
        token: themeTokens
      }}
    >
      <QueryClientProvider client={queryClient}>
        <RequestErrorToast />
        <TenantProvider>
          <FeatureGateProvider>{children}</FeatureGateProvider>
        </TenantProvider>
      </QueryClientProvider>
    </ConfigProvider>
  );
};
