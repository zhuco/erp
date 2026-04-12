# SSO Runbook（Keycloak）

## 1. 启动 Keycloak

```bash
docker compose -f infra/keycloak/docker-compose.yml up -d
```

默认管理入口：
- 控制台：`http://localhost:18080`
- 用户名：`admin`
- 密码：`admin`

## 2. 创建 ERP Realm 与 Client

1. 登录管理控制台，创建 Realm：`erp`
2. 在 `Clients` 中创建客户端：
- `Client ID`: `erp-web`
- `Client authentication`: 关闭（Public Client）
- `Valid redirect URIs`: `http://localhost:5173/*`
- `Web origins`: `http://localhost:5173`
3. 记录 Issuer：
- `http://localhost:18080/realms/erp`

## 3. ERP 环境变量

后端环境变量示例：

```bash
ERP_KEYCLOAK_ISSUER=http://localhost:18080/realms/erp
```

可选（用于统计健康态展示）：

```bash
ERP_CLICKHOUSE_URL=http://localhost:8123
```

## 4. 健康检查

启动后可通过接口查看统计与 SSO 基础设施状态：

```bash
GET /v1/tenants/{tenant_id}/stats/health
```

返回字段：
- `analytics_backend`
- `clickhouse_ready`
- `clickhouse_endpoint`
- `sso_ready`
- `sso_issuer`
- `checked_at`

## 5. 常见故障排查

- `sso_ready=false`
  - 检查 `ERP_KEYCLOAK_ISSUER` 是否配置且可访问
- 浏览器回跳失败
  - 检查 Keycloak Client 的 `Valid redirect URIs`
- 前端跨域问题
  - 检查 `Web origins` 是否包含前端域名与端口
