# Windows 本地运行指南（PowerShell）

适用于 Windows 10/11 的本地开发与联调，覆盖 Web UI Mock 与全链路本地联调两种模式。

## 前置依赖
- Git（建议安装 Git for Windows）
- Docker Desktop（建议启用 WSL2）
- WSL2（可选但推荐）
- Node.js 20.x（含 Corepack）
- pnpm（通过 `corepack enable` 启用）
- Rust stable（含 Cargo）
- sqlx-cli（可选：用于本地迁移）
- Postgres（如使用容器则无需本地安装）

## 端口表
| 服务 | 端口 | 说明 |
| --- | --- | --- |
| Web UI (Vite) | 5173 | `apps/erp-web` |
| erp-api | 8080 | `http://localhost:8080/healthz` |
| erp-channel | 8081 | `http://localhost:8081/healthz` |
| Postgres | 5432 | 容器内数据库 |
| Redis | 6379 | 容器内缓存 |
| NATS | 4222 | 消息总线 |
| NATS Monitor | 8222 | 管理端口 |

## 模式 A：全 Mock（仅 Web UI）
Web UI 使用内置 mocks，后端服务可不启动。

### 一键启动
```powershell
.\scripts\dev-mocks.ps1
```

### 手动启动
```powershell
corepack enable
pnpm install
cd apps/erp-web
$env:VITE_USE_MOCKS="true"
$env:VITE_API_BASE_URL="http://localhost:8080"
pnpm dev
```

打开 `http://localhost:5173`。登录 Token 可任意字符串，租户选择“主租户”或 `11111111-1111-1111-1111-111111111111`。

## 模式 B：本地联调（API + Worker + Channel + DB + Web）
### 一键启动
```powershell
.\scripts\dev.ps1
```

### 手动启动
1) 准备本地环境变量：
```powershell
Copy-Item env/.env.example .env
```
根据需要编辑 `.env`（尤其是 `DATABASE_URL`、`JWT_SECRET`）。

2) 启动本地基础设施：
```powershell
docker compose -f docker-compose.local.yml up -d
```

3) 运行数据库迁移（推荐）：
```powershell
cargo install sqlx-cli --no-default-features --features rustls,postgres
sqlx migrate run
```

4) 启动后端服务（建议开 3 个终端）：
```powershell
cargo run -p erp-api
cargo run -p erp-worker
cargo run -p erp-channel
```

5) 启动 Web UI：
```powershell
corepack enable
pnpm install
$env:VITE_USE_MOCKS="false"
$env:VITE_API_BASE_URL="http://localhost:8080"
pnpm --filter erp-web dev
```

打开 `http://localhost:5173`，API 健康检查：`http://localhost:8080/healthz`，Channel 健康检查：`http://localhost:8081/healthz`。

### 停止服务
```powershell
.\scripts\stop.ps1
```

## 官方门禁（统一主入口）
```powershell
pnpm run repo:verify
pnpm run accept:web
pnpm run accept:backend
pnpm run accept:all
```

## 可选补充（非主入口）
```powershell
pnpm run test:e2e
```

兼容层（历史入口，不作为主推荐）：
- `scripts/accept_*.sh`
- `Makefile`

## 常见错误排查
- pnpm 不可用：`corepack enable` 或 `corepack prepare pnpm@9.1.0 --activate`
- Playwright 浏览器未安装：`pnpm --filter erp-web exec playwright install chromium`
- 端口占用：`Get-NetTCPConnection -LocalPort 5173` 并结合 `Stop-Process -Id <PID>`
- Docker 启动失败：确认 Docker Desktop 正在运行，必要时重启
- 迁移失败：检查 `.env` 中 `DATABASE_URL`、Postgres 容器是否 healthy
