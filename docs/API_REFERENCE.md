# A3Mailer Mail Server - API 参考文档

本文档提供了 A3Mailer Mail Server 的完整 API 参考，包括 REST API、管理接口、监控端点和安全 API。

## 📋 目录

- [认证](#认证)
- [管理 API](#管理-api)
- [监控 API](#监控-api)
- [安全 API](#安全-api)
- [性能 API](#性能-api)
- [健康检查 API](#健康检查-api)
- [错误处理](#错误处理)
- [SDK 和客户端](#sdk-和客户端)

## 🔐 认证

### API 密钥认证

所有 API 请求都需要在请求头中包含有效的 API 密钥：

```http
Authorization: Bearer your_api_key_here
Content-Type: application/json
```

### 获取 API 密钥

```bash
# 生成新的 API 密钥
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -d '{
    "name": "monitoring-key",
    "permissions": ["read", "write"],
    "expires_at": "2024-12-31T23:59:59Z"
  }'
```

响应：
```json
{
  "key_id": "key_123456789",
  "api_key": "sk_live_abcdef123456789",
  "name": "monitoring-key",
  "permissions": ["read", "write"],
  "created_at": "2024-01-15T10:30:00Z",
  "expires_at": "2024-12-31T23:59:59Z"
}
```

## 🛠️ 管理 API

### 服务器状态

#### 获取服务器信息
```http
GET /api/v1/server/info
```

响应：
```json
{
  "version": "0.13.1",
  "build_date": "2024-01-15T10:00:00Z",
  "uptime": 86400,
  "hostname": "mail.example.com",
  "features": ["smtp", "imap", "pop3", "monitoring", "security"],
  "status": "running"
}
```

#### 重启服务器
```http
POST /api/v1/server/restart
```

#### 重新加载配置
```http
POST /api/v1/server/reload
```

### 用户管理

#### 创建用户
```http
POST /api/v1/users
Content-Type: application/json

{
  "username": "john.doe",
  "email": "john.doe@example.com",
  "password": "secure_password_123",
  "quota": 1073741824,
  "enabled": true,
  "groups": ["users", "staff"]
}
```

#### 获取用户列表
```http
GET /api/v1/users?page=1&limit=50&search=john
```

响应：
```json
{
  "users": [
    {
      "id": "user_123",
      "username": "john.doe",
      "email": "john.doe@example.com",
      "quota": 1073741824,
      "used_quota": 536870912,
      "enabled": true,
      "created_at": "2024-01-01T00:00:00Z",
      "last_login": "2024-01-15T09:30:00Z"
    }
  ],
  "total": 1,
  "page": 1,
  "limit": 50
}
```

#### 更新用户
```http
PUT /api/v1/users/{user_id}
Content-Type: application/json

{
  "quota": 2147483648,
  "enabled": true,
  "groups": ["users", "staff", "admin"]
}
```

#### 删除用户
```http
DELETE /api/v1/users/{user_id}
```

### 域名管理

#### 添加域名
```http
POST /api/v1/domains
Content-Type: application/json

{
  "domain": "example.com",
  "enabled": true,
  "max_users": 1000,
  "max_quota": 107374182400
}
```

#### 获取域名列表
```http
GET /api/v1/domains
```

### 邮件队列管理

#### 获取队列状态
```http
GET /api/v1/queue/status
```

响应：
```json
{
  "total_messages": 150,
  "active_messages": 10,
  "deferred_messages": 5,
  "bounced_messages": 2,
  "queues": {
    "incoming": 50,
    "outgoing": 75,
    "retry": 20,
    "bounce": 5
  }
}
```

#### 清空队列
```http
DELETE /api/v1/queue/{queue_name}
```

#### 重试失败邮件
```http
POST /api/v1/queue/retry
```

## 📊 监控 API

### 系统指标

#### 获取系统指标
```http
GET /api/v1/metrics/system
```

响应：
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "cpu_usage": 45.2,
  "memory_usage": 2147483648,
  "memory_total": 8589934592,
  "disk_usage": 53687091200,
  "disk_total": 107374182400,
  "network_rx_bytes": 1048576000,
  "network_tx_bytes": 524288000,
  "load_average": {
    "1m": 1.25,
    "5m": 1.15,
    "15m": 1.05
  },
  "active_connections": 234,
  "uptime": 86400
}
```

#### 获取应用指标
```http
GET /api/v1/metrics/application
```

响应：
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "total_requests": 12345,
  "successful_requests": 12000,
  "failed_requests": 345,
  "avg_response_time": 150.5,
  "p95_response_time": 500.0,
  "p99_response_time": 1000.0,
  "requests_per_second": 25.5,
  "active_sessions": 150,
  "queue_sizes": {
    "incoming": 50,
    "outgoing": 75,
    "processing": 25
  },
  "cache_hit_rates": {
    "user_cache": 0.85,
    "domain_cache": 0.92,
    "config_cache": 0.98
  }
}
```

### Prometheus 指标

#### 获取 Prometheus 格式指标
```http
GET /metrics
```

响应：
```
# HELP stalwart_cpu_usage_percent CPU usage percentage
# TYPE stalwart_cpu_usage_percent gauge
stalwart_cpu_usage_percent 45.2

# HELP stalwart_memory_usage_bytes Memory usage in bytes
# TYPE stalwart_memory_usage_bytes gauge
stalwart_memory_usage_bytes 2147483648

# HELP stalwart_http_requests_total Total HTTP requests
# TYPE stalwart_http_requests_total counter
stalwart_http_requests_total{method="GET",status="200"} 12000
stalwart_http_requests_total{method="POST",status="201"} 300
stalwart_http_requests_total{method="GET",status="404"} 45

# HELP stalwart_response_time_seconds Response time in seconds
# TYPE stalwart_response_time_seconds histogram
stalwart_response_time_seconds_bucket{le="0.1"} 8000
stalwart_response_time_seconds_bucket{le="0.5"} 11500
stalwart_response_time_seconds_bucket{le="1.0"} 12000
stalwart_response_time_seconds_bucket{le="+Inf"} 12345
stalwart_response_time_seconds_sum 1850.5
stalwart_response_time_seconds_count 12345
```

### 自定义指标

#### 设置自定义指标
```http
POST /api/v1/metrics/custom
Content-Type: application/json

{
  "name": "custom_counter",
  "type": "counter",
  "value": 42,
  "labels": {
    "service": "email_processor",
    "environment": "production"
  }
}
```

#### 获取自定义指标
```http
GET /api/v1/metrics/custom/{metric_name}
```

## 🔒 安全 API

### 安全状态

#### 获取安全概览
```http
GET /api/v1/security/overview
```

响应：
```json
{
  "security_score": 0.95,
  "threat_level": "low",
  "active_threats": 2,
  "blocked_ips": 15,
  "failed_auth_attempts": 45,
  "rate_limit_violations": 8,
  "last_security_scan": "2024-01-15T09:00:00Z",
  "security_events": {
    "last_24h": 23,
    "last_7d": 156,
    "last_30d": 678
  }
}
```

### IP 管理

#### 获取被阻止的 IP
```http
GET /api/v1/security/blocked-ips
```

#### 阻止 IP 地址
```http
POST /api/v1/security/block-ip
Content-Type: application/json

{
  "ip_address": "192.0.2.100",
  "reason": "Brute force attack detected",
  "duration": 3600,
  "permanent": false
}
```

#### 解除 IP 阻止
```http
DELETE /api/v1/security/blocked-ips/{ip_address}
```

### 审计日志

#### 获取审计日志
```http
GET /api/v1/security/audit-logs?start_date=2024-01-01&end_date=2024-01-15&event_type=authentication&limit=100
```

响应：
```json
{
  "logs": [
    {
      "id": "audit_123456",
      "timestamp": "2024-01-15T10:30:00Z",
      "event_type": "authentication",
      "severity": "warning",
      "outcome": "failure",
      "source_ip": "192.0.2.100",
      "user_id": "user_123",
      "action": "login_failed",
      "description": "Failed login attempt for user john.doe",
      "metadata": {
        "user_agent": "Mozilla/5.0...",
        "method": "password"
      }
    }
  ],
  "total": 1,
  "page": 1,
  "limit": 100
}
```

### 速率限制

#### 获取速率限制状态
```http
GET /api/v1/security/rate-limits
```

#### 配置速率限制
```http
PUT /api/v1/security/rate-limits
Content-Type: application/json

{
  "global": {
    "max_requests": 1000,
    "window_seconds": 60
  },
  "per_ip": {
    "max_requests": 100,
    "window_seconds": 60
  },
  "per_user": {
    "max_requests": 500,
    "window_seconds": 60
  }
}
```

## ⚡ 性能 API

### 性能统计

#### 获取性能统计
```http
GET /api/v1/performance/stats
```

响应：
```json
{
  "connection_pool": {
    "total_connections": 50,
    "active_connections": 25,
    "idle_connections": 20,
    "failed_connections": 5,
    "avg_connection_time": 15.5,
    "connection_reuse_rate": 0.85
  },
  "cache_performance": {
    "hit_rate": 0.92,
    "miss_rate": 0.08,
    "eviction_rate": 0.02,
    "avg_lookup_time": 2.5,
    "cache_size": 1073741824,
    "cache_utilization": 0.75
  },
  "request_performance": {
    "avg_response_time": 150.5,
    "p50_response_time": 100.0,
    "p95_response_time": 500.0,
    "p99_response_time": 1000.0,
    "throughput": 25.5,
    "error_rate": 0.028
  }
}
```

### 性能优化

#### 获取优化建议
```http
GET /api/v1/performance/recommendations
```

响应：
```json
{
  "recommendations": [
    {
      "category": "cache",
      "priority": "high",
      "title": "增加缓存大小",
      "description": "当前缓存命中率为 85%，建议增加缓存大小以提高性能",
      "impact": "可提升 15% 的响应速度",
      "action": "将缓存大小从 1GB 增加到 2GB"
    },
    {
      "category": "connection_pool",
      "priority": "medium",
      "title": "优化连接池配置",
      "description": "连接复用率较低，建议调整连接池参数",
      "impact": "可减少 20% 的连接开销",
      "action": "增加最大空闲连接数到 30"
    }
  ]
}
```

#### 应用性能优化
```http
POST /api/v1/performance/optimize
Content-Type: application/json

{
  "optimizations": [
    {
      "type": "cache_size",
      "value": 2147483648
    },
    {
      "type": "connection_pool_size",
      "value": 100
    }
  ]
}
```

## 🏥 健康检查 API

### 健康状态

#### 获取整体健康状态
```http
GET /health
```

响应：
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "uptime": 86400,
  "version": "0.13.1",
  "checks": {
    "database": {
      "status": "healthy",
      "response_time": 5,
      "details": {
        "connection_pool": "10/20 active",
        "query_time": "5ms"
      }
    },
    "cache": {
      "status": "healthy",
      "response_time": 2,
      "details": {
        "hit_rate": "85%",
        "memory_usage": "512MB"
      }
    },
    "disk_space": {
      "status": "healthy",
      "response_time": 1,
      "details": {
        "usage_percent": "75%",
        "available_space": "25GB"
      }
    },
    "memory": {
      "status": "healthy",
      "response_time": 1,
      "details": {
        "usage_percent": "65%",
        "available_memory": "2GB"
      }
    }
  }
}
```

#### 获取详细健康检查
```http
GET /api/v1/health/detailed
```

#### 运行特定健康检查
```http
POST /api/v1/health/check/{component}
```

### 就绪状态检查

#### 检查服务就绪状态
```http
GET /ready
```

响应：
```json
{
  "ready": true,
  "timestamp": "2024-01-15T10:30:00Z",
  "services": {
    "database": true,
    "cache": true,
    "smtp": true,
    "imap": true,
    "pop3": true
  }
}
```

## ❌ 错误处理

### 错误响应格式

所有 API 错误都遵循统一的响应格式：

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "请求参数无效",
    "details": "字段 'email' 是必需的",
    "timestamp": "2024-01-15T10:30:00Z",
    "request_id": "req_123456789"
  }
}
```

### 常见错误代码

| 错误代码 | HTTP 状态码 | 描述 |
|---------|------------|------|
| `INVALID_REQUEST` | 400 | 请求参数无效 |
| `UNAUTHORIZED` | 401 | 未授权访问 |
| `FORBIDDEN` | 403 | 权限不足 |
| `NOT_FOUND` | 404 | 资源不存在 |
| `RATE_LIMITED` | 429 | 请求频率超限 |
| `INTERNAL_ERROR` | 500 | 服务器内部错误 |
| `SERVICE_UNAVAILABLE` | 503 | 服务不可用 |

### 错误处理最佳实践

```javascript
// JavaScript 示例
async function apiRequest(endpoint, options = {}) {
  try {
    const response = await fetch(endpoint, {
      headers: {
        'Authorization': 'Bearer your_api_key',
        'Content-Type': 'application/json',
        ...options.headers
      },
      ...options
    });
    
    if (!response.ok) {
      const error = await response.json();
      throw new Error(`API Error: ${error.error.message}`);
    }
    
    return await response.json();
  } catch (error) {
    console.error('API request failed:', error);
    throw error;
  }
}
```

## 📚 SDK 和客户端

### 官方 SDK

#### JavaScript/Node.js
```bash
npm install @stalwart/mail-server-sdk
```

```javascript
import { StalwartClient } from '@stalwart/mail-server-sdk';

const client = new StalwartClient({
  baseUrl: 'https://mail.example.com',
  apiKey: 'your_api_key'
});

// 获取服务器信息
const serverInfo = await client.server.getInfo();

// 创建用户
const user = await client.users.create({
  username: 'john.doe',
  email: 'john.doe@example.com',
  password: 'secure_password'
});
```

#### Python
```bash
pip install stalwart-mail-server
```

```python
from stalwart import StalwartClient

client = StalwartClient(
    base_url='https://mail.example.com',
    api_key='your_api_key'
)

# 获取系统指标
metrics = client.metrics.get_system_metrics()

# 获取健康状态
health = client.health.get_status()
```

#### Go
```bash
go get github.com/stalwartlabs/stalwart-go-sdk
```

```go
package main

import (
    "github.com/stalwartlabs/stalwart-go-sdk"
)

func main() {
    client := stalwart.NewClient("https://mail.example.com", "your_api_key")
    
    // 获取用户列表
    users, err := client.Users.List(&stalwart.UserListOptions{
        Page:  1,
        Limit: 50,
    })
    if err != nil {
        log.Fatal(err)
    }
}
```

### API 客户端示例

#### cURL 示例
```bash
# 获取服务器信息
curl -H "Authorization: Bearer your_api_key" \
     https://mail.example.com/api/v1/server/info

# 创建用户
curl -X POST \
     -H "Authorization: Bearer your_api_key" \
     -H "Content-Type: application/json" \
     -d '{"username":"john.doe","email":"john.doe@example.com","password":"secure_password"}' \
     https://mail.example.com/api/v1/users

# 获取指标
curl -H "Authorization: Bearer your_api_key" \
     https://mail.example.com/api/v1/metrics/system
```

这个 API 参考文档提供了完整的 API 接口说明，包括认证、管理、监控、安全、性能和健康检查等各个方面的 API。
