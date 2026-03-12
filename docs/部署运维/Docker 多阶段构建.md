# Docker 多阶段构建

## 基本概念

多阶段构建（Multi-stage builds）允许在一个 Dockerfile 中使用多个 FROM 指令，每个 FROM 开始一个新的构建阶段。

**优势：**
- 减小最终镜像体积
- 提高安全性（不包含构建工具）
- 简化 Dockerfile 维护

## 基本语法

```dockerfile
# 第一阶段：构建阶段
FROM golang:1.21 AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN go build -o myapp .

# 第二阶段：运行阶段
FROM alpine:3.18

WORKDIR /app
# 从构建阶段复制编译好的二进制
COPY --from=builder /app/myapp .

CMD ["./myapp"]
```

## 实际案例

### Node.js 应用

```dockerfile
# 构建阶段
FROM node:20-alpine AS builder

WORKDIR /app
COPY package*.json ./
RUN npm ci

COPY . .
RUN npm run build

# 生产阶段
FROM node:20-alpine

WORKDIR /app
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY package*.json ./

ENV NODE_ENV=production
CMD ["node", "dist/index.js"]
```

### React 前端

```dockerfile
# 构建阶段
FROM node:20-alpine AS builder

WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Nginx 服务阶段
FROM nginx:alpine

# 复制构建产物
COPY --from=builder /app/build /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

### Rust 应用

```dockerfile
# 构建阶段
FROM rust:1.74 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# 运行阶段
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/myapp /usr/local/bin/

CMD ["myapp"]
```

## 最佳实践

### 1. 命名构建阶段

```dockerfile
FROM golang:1.21 AS build
FROM alpine:3.18 AS runtime
```

### 2. 使用 ARG 传递参数

```dockerfile
ARG VERSION=latest
FROM node:${VERSION} AS builder
```

### 3. 复制特定文件

```dockerfile
COPY --from=builder /app/target/release/myapp /usr/local/bin/
```

### 4. 利用缓存层

```dockerfile
# 先复制依赖文件，利用 Docker 缓存
COPY package*.json ./
RUN npm ci

# 再复制源代码
COPY . .
```

## 镜像大小对比

| 方案 | 镜像大小 |
|------|----------|
| 单阶段（包含构建工具） | ~1GB |
| 多阶段（仅运行环境） | ~50MB |
