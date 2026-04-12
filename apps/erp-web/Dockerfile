# Multi-stage build for ERP Web UI
FROM node:20-alpine AS build

WORKDIR /workspace

# Enable pnpm
RUN corepack enable

# Copy package files for dependency installation
COPY package.json pnpm-workspace.yaml ./
COPY apps/erp-web/package.json apps/erp-web/

# Install dependencies (cached layer)
RUN pnpm install --filter erp-web... --frozen-lockfile

# Copy source code
COPY apps/erp-web apps/erp-web
COPY openapi openapi

# Build arguments for env variables
ARG VITE_API_BASE_URL=/api
ARG VITE_BUILD_VERSION=unknown
ARG VITE_ROUTER_MODE=browser
ARG VITE_ROUTER_BASENAME=/
ARG BUILD_DATE
ARG GIT_COMMIT

ENV VITE_API_BASE_URL=$VITE_API_BASE_URL
ENV VITE_BUILD_VERSION=$VITE_BUILD_VERSION
ENV VITE_ROUTER_MODE=$VITE_ROUTER_MODE
ENV VITE_ROUTER_BASENAME=$VITE_ROUTER_BASENAME

# Build production bundle
RUN pnpm --filter erp-web build

# Production stage with nginx
FROM nginx:1.27-alpine

# Install envsubst for runtime config injection
RUN apk add --no-cache gettext

# Copy nginx configuration
COPY apps/erp-web/nginx.conf /etc/nginx/nginx.conf

# Copy built static files
COPY --from=build /workspace/apps/erp-web/dist /usr/share/nginx/html

# Add health check endpoint
COPY apps/erp-web/docker-healthcheck.sh /usr/local/bin/healthcheck
RUN chmod +x /usr/local/bin/healthcheck

# Metadata labels
LABEL org.opencontainers.image.title="ERP Web UI"
LABEL org.opencontainers.image.description="Multi-platform ERP Web Interface"
LABEL org.opencontainers.image.version="${VITE_BUILD_VERSION}"
LABEL org.opencontainers.image.created="${BUILD_DATE}"
LABEL org.opencontainers.image.revision="${GIT_COMMIT}"

# Expose HTTP port
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD /usr/local/bin/healthcheck

# Run nginx in foreground
CMD ["nginx", "-g", "daemon off;"]
