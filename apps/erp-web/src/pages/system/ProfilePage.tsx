import { Button, Descriptions, Form, Input, message } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import {
  fetchCurrentUser,
  updateCurrentUser,
  updateCurrentUserPassword,
  type CurrentUserProfile
} from '@/api/auth';
import { ErrorState, LoadingState } from '@/components/QueryState';
import { SettingsPageTemplate, SettingsPageSection } from '@/components/page-templates';
import { useSessionStore } from '@/features/sessionStore';

type ProfileFormValues = {
  email: string;
};

type PasswordFormValues = {
  current_password: string;
  new_password: string;
  confirm_password: string;
};

const formatDateTime = (value?: string) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

export const ProfilePage = () => {
  const [profileForm] = Form.useForm<ProfileFormValues>();
  const [passwordForm] = Form.useForm<PasswordFormValues>();
  const setUserName = useSessionStore((state) => state.setUserName);
  const [savingProfile, setSavingProfile] = useState(false);
  const [savingPassword, setSavingPassword] = useState(false);

  const profileQuery = useQuery({
    queryKey: ['current-user'],
    queryFn: () => fetchCurrentUser()
  });

  useEffect(() => {
    if (!profileQuery.data) {
      return;
    }
    profileForm.setFieldsValue({ email: profileQuery.data.user.email });
  }, [profileForm, profileQuery.data]);

  const handleProfileSubmit = async () => {
    try {
      const values = await profileForm.validateFields();
      const email = values.email.trim();
      setSavingProfile(true);
      const updated: CurrentUserProfile = await updateCurrentUser({ email });
      setUserName(updated.email);
      message.success('个人信息已更新');
      await profileQuery.refetch();
    } catch (error) {
      if (error instanceof Error) {
        message.error(error.message);
      }
    } finally {
      setSavingProfile(false);
    }
  };

  const handlePasswordSubmit = async () => {
    try {
      const values = await passwordForm.validateFields();
      if (values.new_password !== values.confirm_password) {
        message.error('两次输入的密码不一致');
        return;
      }
      setSavingPassword(true);
      await updateCurrentUserPassword({
        current_password: values.current_password,
        new_password: values.new_password
      });
      passwordForm.resetFields();
      message.success('密码已更新');
    } catch (error) {
      if (error instanceof Error) {
        message.error(error.message);
      }
    } finally {
      setSavingPassword(false);
    }
  };

  if (profileQuery.isError) {
    return <ErrorState error={profileQuery.error} onRetry={() => profileQuery.refetch()} />;
  }

  const profile = profileQuery.data?.user;

  return (
    <SettingsPageTemplate title="个人信息">
      <SettingsPageSection title="个人信息">
        <LoadingState loading={profileQuery.isLoading} />
        <Descriptions column={3} bordered size="small" style={{ marginBottom: 16 }}>
          <Descriptions.Item label="用户ID">{profile?.user_id ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="租户ID">{profile?.tenant_id ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="状态">{profile?.status ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="创建时间">
            {formatDateTime(profile?.created_at)}
          </Descriptions.Item>
        </Descriptions>
        <Form layout="vertical" form={profileForm} onFinish={handleProfileSubmit}>
          <Form.Item
            label="邮箱"
            name="email"
            rules={[
              { required: true, message: '请输入邮箱' },
              { type: 'email', message: '邮箱格式不正确' }
            ]}
          >
            <Input placeholder="请输入邮箱" />
          </Form.Item>
          <Button type="primary" htmlType="submit" loading={savingProfile}>
            保存资料
          </Button>
        </Form>
      </SettingsPageSection>
      <SettingsPageSection title="修改密码">
        <Form layout="vertical" form={passwordForm} onFinish={handlePasswordSubmit}>
          <Form.Item
            label="当前密码"
            name="current_password"
            rules={[{ required: true, message: '请输入当前密码' }]}
          >
            <Input.Password placeholder="请输入当前密码" />
          </Form.Item>
          <Form.Item
            label="新密码"
            name="new_password"
            rules={[{ required: true, message: '请输入新密码' }]}
          >
            <Input.Password placeholder="请输入新密码" />
          </Form.Item>
          <Form.Item
            label="确认新密码"
            name="confirm_password"
            dependencies={['new_password']}
            rules={[
              { required: true, message: '请确认新密码' },
              ({ getFieldValue }) => ({
                validator(_, value) {
                  if (!value || getFieldValue('new_password') === value) {
                    return Promise.resolve();
                  }
                  return Promise.reject(new Error('两次输入的密码不一致'));
                }
              })
            ]}
          >
            <Input.Password placeholder="请再次输入新密码" />
          </Form.Item>
          <Button type="primary" htmlType="submit" loading={savingPassword}>
            更新密码
          </Button>
        </Form>
      </SettingsPageSection>
    </SettingsPageTemplate>
  );
};
