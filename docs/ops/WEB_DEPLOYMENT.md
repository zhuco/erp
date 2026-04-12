# Web UI Deployment Guide

## Overview

This guide covers deploying the 衡策云 Web UI to production using Docker containers.

---

## Quick Start

### Local Docker Build

```bash
# Navigate to repository root
cd /path/to/erp

# Build Docker image
docker build \
  --build-arg VITE_BUILD_VERSION=$(git describe --tags --always) \
  --build-arg BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ") \
  --build-arg GIT_COMMIT=$(git rev-parse HEAD) \
  -t erp-web:latest \
  -f apps/erp-web/Dockerfile \
  .

# Run container
docker run -d \
  --name erp-web \
  -p 8080:80 \
  -e API_BASE_URL=https://api.example.com \
  erp-web:latest
```

Visit `http://localhost:8080`

---

## Configuration

### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `VITE_API_BASE_URL` | Backend API URL (build-time) | `/api` | No |
| `VITE_BUILD_VERSION` | Version string (build-time) | `unknown` | No |
| `VITE_ROUTER_MODE` | Routing mode (`browser` or `hash`) | `browser` | No |
| `VITE_ROUTER_BASENAME` | Base path when UI served under subpath | `/` | No |
| `API_BACKEND_URL` | Backend URL for nginx proxy (runtime) | - | No |

### Build-Time vs Runtime Configuration

**Build-Time** (Dockerfile `ARG`):

- `VITE_API_BASE_URL` - Compiled into JavaScript bundle
- `VITE_BUILD_VERSION` - Displayed in UI and console
- `VITE_ROUTER_MODE` - Switch to hash routing for static hosts without SPA fallback
- `VITE_ROUTER_BASENAME` - Base path for subpath deployments (e.g. `/erp/`)

**Runtime** (Docker `ENV`):

- `API_BACKEND_URL` - Used by nginx for `/api` proxy (if enabled)

---

## Production Deployment

### 1. Multi-Stage Build

The Dockerfile uses multi-stage builds for minimal image size:

```dockerfile
# Stage 1: Build (node:20-alpine + pnpm)
FROM node:20-alpine AS build
# ... build static files ...

# Stage 2: Serve (nginx:1.27-alpine)
FROM nginx:1.27-alpine
COPY --from=build /workspace/apps/erp-web/dist /usr/share/nginx/html
```

**Benefits**:

- Final image ~40MB (nginx + static files only)
- No Node.js or build tools in production
- Faster deployments and startups

### 2. Build with Version Metadata

```bash
export VERSION=$(git describe --tags --always)
export BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
export GIT_COMMIT=$(git rev-parse HEAD)

docker build \
  --build-arg VITE_BUILD_VERSION=$VERSION \
  --build-arg BUILD_DATE=$BUILD_DATE \
  --build-arg GIT_COMMIT=$GIT_COMMIT \
  -t erp-web:$VERSION \
  -f apps/erp-web/Dockerfile \
  .
```

### 3. Run with API Backend

**Option A: Frontend calls API directly**

```bash
docker run -d \
  --name erp-web \
  -p 80:80 \
  -e API_BASE_URL=/api \
  erp-web:$VERSION
```

Frontend will call `/api/*` (same-origin). Backend must be accessible separately.

**Option B: nginx proxies API**

Uncomment the `/api` location block in `nginx.conf`:

```nginx
location /api/ {
  proxy_pass ${API_BACKEND_URL}/;
  # ...
}
```

Then run:

```bash
docker run -d \
  --name erp-web \
  -p 80:80 \
  -e API_BACKEND_URL=http://erp-api:8080 \
  erp-web:$VERSION
```

### 4. Docker Compose Example

```yaml
version: '3.8'

services:
  web:
    image: erp-web:latest
    build:
      context: .
      dockerfile: apps/erp-web/Dockerfile
      args:
        VITE_BUILD_VERSION: ${VERSION:-dev}
    ports:
      - "80:80"
    environment:
      API_BACKEND_URL: http://api:8080
    healthcheck:
      test: ["CMD", "/usr/local/bin/healthcheck"]
      interval: 30s
      timeout: 3s
      retries: 3
    restart: unless-stopped
    
  api:
    image: erp-api:latest
    ports:
      - "8080:8080"
    # ... API configuration
```

---

## Production Best Practices

### 1. HTTPS/TLS

**Option A: Reverse Proxy (Recommended)**

Use a reverse proxy (Traefik, nginx, Caddy) in front:

```yaml
# docker-compose.yml with Traefik
services:
  traefik:
    image: traefik:v2.10
    command:
      - "--providers.docker=true"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.le.acme.tlschallenge=true"
      - "--certificatesresolvers.le.acme.email=admin@example.com"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - ./acme.json:/acme.json
  
  web:
    image: erp-web:latest
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.erp-web.rule=Host(`erp.example.com`)"
      - "traefik.http.routers.erp-web.entrypoints=websecure"
      - "traefik.http.routers.erp-web.tls.certresolver=le"
```

**Option B: nginx SSL**

Mount SSL certificates and update nginx config:

```bash
docker run -d \
  --name erp-web \
  -p 443:443 \
  -v /path/to/ssl:/etc/nginx/ssl:ro \
  -v /path/to/nginx-ssl.conf:/etc/nginx/nginx.conf:ro \
  erp-web:latest
```

### 2. Caching Strategy

The nginx configuration includes optimal caching:

```nginx
# Static assets - 1 year cache
location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
  expires 1y;
  add_header Cache-Control "public, immutable";
}

# index.html - no cache (always fresh)
location = /index.html {
  expires -1;
  add_header Cache-Control "no-store, no-cache, must-revalidate";
}
```

**Vite automatically adds content hashes** to asset filenames (`main-abc123.js`), enabling long-term caching.

### 3. Logging

**Access Logs**:

```bash
docker logs -f erp-web
```

**Export to File**:

```bash
docker run -d \
  -v /var/log/nginx:/var/log/nginx \
  erp-web:latest
```

**Log Aggregation** (recommended):

- Use Docker logging drivers (fluentd, splunk, etc.)
- Or ship logs to centralized system (ELK, Loki, etc.)

### 4. Health Checks

The container includes a health check:

```bash
# Check container health
docker ps
# Look for "healthy" status

# Manual health check
docker exec erp-web /usr/local/bin/healthcheck
```

**Kubernetes Example**:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: erp-web
spec:
  containers:
  - name: web
    image: erp-web:latest
    livenessProbe:
      httpGet:
        path: /health
        port: 80
      initialDelaySeconds: 5
      periodSeconds: 10
    readinessProbe:
      httpGet:
        path: /health
        port: 80
      initialDelaySeconds: 3
      periodSeconds: 5
```

### 5. Resource Limits

Limit container resources:

```bash
docker run -d \
  --name erp-web \
  --memory="256m" \
  --cpus="0.5" \
  -p 80:80 \
  erp-web:latest
```

**Kubernetes**:

```yaml
resources:
  requests:
    memory: "128Mi"
    cpu: "100m"
  limits:
    memory: "256Mi"
    cpu: "500m"
```

### 6. Security Headers

The nginx config includes security headers:

- `X-Frame-Options: SAMEORIGIN`
- `X-Content-Type-Options: nosniff`
- `X-XSS-Protection: 1; mode=block`
- `Referrer-Policy: no-referrer-when-downgrade`

**Add CSP (Content Security Policy)**:

```nginx
add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline';" always;
```

### 7. Monitoring

**Version Tracking**:

- Version displayed in UI footer (see `AppVersion` component)
- Version logged to console on startup
- Version in Docker image labels

**Metrics** (optional):

- Add nginx-prometheus-exporter sidecar
- Monitor response times, error rates
- Alert on health check failures

---

## Deployment Strategies

### Rolling Update

```bash
# Build new version
docker build -t erp-web:v2.0.0 -f apps/erp-web/Dockerfile .

# Stop old container gracefully
docker stop erp-web

# Start new container
docker run -d --name erp-web -p 80:80 erp-web:v2.0.0

# Remove old container
docker rm erp-web-old
```

### Blue-Green Deployment

```bash
# Deploy green (new version)
docker run -d --name erp-web-green -p 8081:80 erp-web:v2.0.0

# Test green
curl http://localhost:8081/health

# Switch traffic (update load balancer)
# ...

# Remove blue (old version)
docker stop erp-web-blue
docker rm erp-web-blue
```

### Canary Deployment

Use load balancer to route percentage of traffic to new version.

---

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker logs erp-web

# Check health
docker inspect erp-web | grep Health
```

**Common issues**:

- Port already in use: Change `-p 8080:80`
- Permission errors: Check file permissions in Dockerfile

### 404 Errors on Refresh

**Cause**: nginx not configured for SPA routing

**Fix**:

1. Verify `try_files $uri $uri/ /index.html;` in nginx.conf
2. If server routing cannot be changed, rebuild with `VITE_ROUTER_MODE=hash`

### API Calls Failing

**Cause**: `VITE_API_BASE_URL` not set correctly

**Fix**:

1. Check console logs for API URL
2. Rebuild with correct `--build-arg VITE_API_BASE_URL=https://api.example.com`
3. Or proxy via nginx (set `API_BACKEND_URL` env var)

### Version Not Displayed

**Cause**: `VITE_BUILD_VERSION` not set in build

**Fix**: Add `--build-arg VITE_BUILD_VERSION=$(git describe --tags --always)`

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build and Deploy Web UI

on:
  push:
    branches: [main]
    tags: ['v*']

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Docker meta
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/${{ github.repository }}/web
          tags: |
            type=ref,event=branch
            type=semver,pattern={{version}}
            type=sha
      
      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          file: apps/erp-web/Dockerfile
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          build-args: |
            VITE_BUILD_VERSION=${{ steps.meta.outputs.version }}
            BUILD_DATE=${{ steps.meta.outputs.created }}
            GIT_COMMIT=${{ github.sha }}
```

---

## Summary

| Task | Command |
|------|---------|
| Build locally | `docker build -t erp-web -f apps/erp-web/Dockerfile .` |
| Run locally | `docker run -d -p 8080:80 erp-web` |
| View logs | `docker logs -f erp-web` |
| Health check | `curl http://localhost:8080/health` |
| Stop container | `docker stop erp-web` |
| Remove container | `docker rm erp-web` |

**Key Files**:

- [`Dockerfile`](file:///f:/erp/apps/erp-web/Dockerfile) - Multi-stage build configuration
- [`nginx.conf`](file:///f:/erp/apps/erp-web/nginx.conf) - Production nginx configuration
- [`docker-healthcheck.sh`](file:///f:/erp/apps/erp-web/docker-healthcheck.sh) - Container health check
- [`.env.example`](file:///f:/erp/apps/erp-web/.env.example) - Environment variable template

---

**Last updated**: 2026-01-08
