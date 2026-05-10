import { Button, Card, Collapse, Form, Input, Space, Typography, message } from 'antd';
import { useLocation, useNavigate } from 'react-router-dom';
import { useState } from 'react';

import { fetchCurrentUser, login } from '@/api/auth';
import type { Permission } from '@/features/rbac';
import { useSessionStore } from '@/features/sessionStore';
import brandLogo from '@/assets/brand/hengceyun-logo.png';

type LoginFormValues = {
  tenantId?: string;
  username: string;
  password: string;
};

type TokenLoginValues = {
  token: string;
};

export const LoginPage = () => {
  const [form] = Form.useForm<LoginFormValues>();
  const [tokenForm] = Form.useForm<TokenLoginValues>();
  const navigate = useNavigate();
  const location = useLocation();
  const [loading, setLoading] = useState(false);
  const setToken = useSessionStore((state) => state.setToken);
  const setUserId = useSessionStore((state) => state.setUserId);
  const setUserName = useSessionStore((state) => state.setUserName);
  const setPermissions = useSessionStore((state) => state.setPermissions);
  const setTenantId = useSessionStore((state) => state.setSelectedTenantId);
  const setShopIds = useSessionStore((state) => state.setSelectedShopIds);
  const resetSession = useSessionStore((state) => state.resetSession);
  const from = (location.state as { from?: string } | undefined)?.from;

  const navigateToTarget = () => {
    const target = from && from !== '/login' ? from : '/sales/order-query';
    navigate(target, { replace: true });
  };

  const applyProfile = (profile: Awaited<ReturnType<typeof fetchCurrentUser>>) => {
    setUserId(profile.user.user_id);
    setUserName(profile.user.email);
    setPermissions(profile.permissions as Permission[]);
    setTenantId(profile.user.tenant_id);
    setShopIds(profile.data_scopes.shop_ids ?? []);
  };

  const handlePasswordSubmit = async (values: LoginFormValues) => {
    const tenantId = values.tenantId?.trim();
    const username = values.username?.trim();
    const password = values.password?.trim();
    if (!tenantId || !username || !password) {
      message.error('请填写商家ID、用户名和密码');
      return;
    }
    try {
      setLoading(true);
      const result = await login({ tenant_id: tenantId, email: username, password });
      setToken(result.access_token);
      const profile = await fetchCurrentUser();
      applyProfile(profile);
      navigateToTarget();
    } catch (error) {
      resetSession();
      message.error(error instanceof Error ? error.message : '登录失败');
    } finally {
      setLoading(false);
    }
  };

  const handleTokenSubmit = async (values: TokenLoginValues) => {
    const token = values.token?.trim();
    if (!token) {
      message.warning('请粘贴 Token');
      return;
    }
    try {
      setLoading(true);
      setToken(token);
      const profile = await fetchCurrentUser();
      applyProfile(profile);
      navigateToTarget();
    } catch (error) {
      resetSession();
      message.error(error instanceof Error ? error.message : 'Token 登录失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: 'flex', justifyContent: 'center', padding: '80px 16px' }}>
      <Card style={{ width: 480 }}>
        <Space orientation="vertical" size={16} style={{ width: '100%' }}>
          <div style={{ display: 'flex', justifyContent: 'center' }}>
            <img
              src={brandLogo}
              alt="衡策云"
              style={{ width: 96, height: 96, borderRadius: 16, objectFit: 'cover' }}
            />
          </div>
          <Typography.Title level={3} style={{ marginBottom: 0 }}>
            登录 衡策云
          </Typography.Title>
          <Typography.Text type="secondary">使用账号密码登录进入系统。</Typography.Text>
          <Form form={form} layout="vertical" onFinish={handlePasswordSubmit}>
            <Form.Item
              name="tenantId"
              label="商家ID"
              rules={[{ required: true, message: '请输入商家ID' }]}
            >
              <Input placeholder="请输入商家ID" data-testid="login-tenant-input" />
            </Form.Item>
            <Form.Item
              name="username"
              label="用户名"
              rules={[{ required: true, message: '请输入用户名' }]}
            >
              <Input placeholder="请输入用户名" data-testid="login-username-input" />
            </Form.Item>
            <Form.Item
              name="password"
              label="密码"
              rules={[{ required: true, message: '请输入密码' }]}
            >
              <Input.Password placeholder="请输入密码" data-testid="login-password-input" />
            </Form.Item>
            <Button
              type="primary"
              htmlType="submit"
              block
              loading={loading}
              data-testid="login-submit"
            >
              登录
            </Button>
          </Form>
          <Collapse
            ghost
            items={[
              {
                key: 'token-login',
                label: '高级：使用访问 Token 登录',
                children: (
                  <Form form={tokenForm} layout="vertical" onFinish={handleTokenSubmit}>
                    <Form.Item
                      name="token"
                      label="Token"
                      rules={[{ required: true, message: '请输入 Token' }]}
                    >
                      <Input.TextArea
                        rows={4}
                        placeholder="粘贴 Token"
                        data-testid="login-token-input"
                      />
                    </Form.Item>
                    <Button type="default" htmlType="submit" block loading={loading}>
                      使用 Token 登录
                    </Button>
                  </Form>
                )
              }
            ]}
          />
        </Space>
      </Card>
    </div>
  );
};
