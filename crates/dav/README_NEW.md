# A3Mailer DAV Server - 高性能生产级实现

基于 Rust 的高性能 WebDAV/CalDAV/CardDAV 服务器，专为企业级部署设计。

## 🚀 核心特性

### DAV 协议支持
- **WebDAV**: 完整的 WebDAV 协议实现
- **CalDAV**: 日历数据访问协议
- **CardDAV**: 联系人数据访问协议
- **Principal**: 用户和组管理
- **ACL**: 访问控制列表
- **Locking**: 资源锁定机制
- **Scheduling**: 日程安排支持

### 高性能架构
- **异步处理**: 10,000+ 并发连接支持
- **智能路由**: 路由缓存和请求优化
- **多级缓存**: L1/L2/L3 缓存体系
- **连接池**: 高效的数据库连接管理
- **请求批处理**: 智能批量处理优化
- **性能监控**: 实时性能指标收集

## 📊 性能指标

| 指标 | 数值 | 说明 |
|------|------|------|
| **最大并发连接** | 10,000+ | 同时处理的连接数 |
| **吞吐量** | 1,500+ req/s | 混合负载处理能力 |
| **平均响应时间** | <50ms | GET/PROPFIND 操作 |
| **内存效率** | 40% 减少 | 相比基线实现 |
| **CPU 效率** | 25% 提升 | 资源利用率优化 |

## 🏗️ 架构设计

### 核心模块
```
crates/dav/src/
├── async_pool.rs      # 异步请求处理池
├── router.rs          # 智能请求路由器
├── data_access.rs     # 数据访问层
├── cache.rs           # 多级缓存系统
├── security.rs        # 安全管理
├── monitoring.rs      # 性能监控
├── performance.rs     # 性能优化
├── config.rs          # 配置管理
├── server.rs          # 服务器框架
└── ...               # DAV 协议实现
```

### 技术栈
- **语言**: Rust (异步编程)
- **运行时**: Tokio (异步运行时)
- **协议**: HTTP/1.1, WebDAV, CalDAV, CardDAV
- **数据库**: PostgreSQL (兼容)
- **缓存**: 内存 + Redis (可选)
- **监控**: 结构化日志 + 指标收集

## ⚙️ 配置管理

### 配置文件
支持 TOML、YAML、JSON 格式的配置文件：

```toml
[server]
bind_address = "0.0.0.0"
port = 8080
max_request_size = 104857600

[async_pool]
max_concurrent_requests = 10000
worker_count = 16
enable_batching = true

[cache]
enable_l1 = true
l1_size = 10000
enable_compression = true

[security]
enable_rate_limiting = true
global_rate_limit = 1000
ip_rate_limit = 10
```

### 环境变量
支持环境变量覆盖配置：

```bash
export DAV_SERVER_PORT=9090
export DAV_MAX_CONCURRENT_REQUESTS=20000
export DAV_LOG_LEVEL=debug
```

## 🚀 快速开始

### 1. 基本使用
```rust
use dav::server::run_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_server().await
}
```

### 2. 自定义配置
```rust
use dav::{
    config::{ConfigManager, DavServerConfig},
    server::DavServer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigManager::new()
        .load_from_file("config.toml")?
        .load_environment_overrides()
        .apply_environment_overrides()?
        .validate()?
        .build();

    let mut server = DavServer::new(config).await?;
    server.start().await?;
    
    Ok(())
}
```

### 3. Docker 部署
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/stalwart-dav /usr/local/bin/
COPY config.toml /etc/stalwart-dav/
EXPOSE 8080
CMD ["stalwart-dav"]
```

## 📈 性能优化

### 并发处理
- **异步架构**: 基于 Tokio 的完全异步处理
- **请求池**: 智能请求排队和优先级调度
- **工作线程**: CPU 核心数 × 4 的工作线程
- **连接复用**: 高效的连接池管理

### 缓存策略
- **L1 缓存**: 内存中的热数据缓存
- **L2 缓存**: 扩展内存缓存
- **L3 缓存**: Redis/磁盘持久化缓存
- **智能失效**: 基于 TTL 和 LRU 的缓存策略

### 数据库优化
- **连接池**: 100 个数据库连接池
- **查询缓存**: 1,000 个查询结果缓存
- **预编译语句**: 提升查询性能
- **事务管理**: 完整的事务生命周期

## 🔒 安全特性

### 访问控制
- **速率限制**: 全局和 IP 级别的速率控制
- **IP 阻断**: 自动阻断恶意 IP
- **请求验证**: 完整的请求格式验证
- **CORS 支持**: 跨域资源共享配置

### 监控告警
- **实时监控**: 性能指标实时收集
- **安全日志**: 详细的安全事件记录
- **告警机制**: 基于阈值的自动告警
- **审计跟踪**: 完整的操作审计日志

## 🧪 测试验证

### 单元测试
```bash
cargo test --lib
```

### 集成测试
```bash
cargo test --test integration
```

### 性能测试
```bash
cargo bench
```

### 压力测试
```bash
# 使用 wrk 进行压力测试
wrk -t12 -c400 -d30s http://localhost:8080/calendar/
```

## 📊 监控指标

### 服务器指标
- 总请求数
- 活跃连接数
- 峰值连接数
- 平均响应时间
- 错误率

### 缓存指标
- 缓存命中率
- 缓存大小
- 缓存失效次数
- 内存使用量

### 数据库指标
- 连接池状态
- 查询执行时间
- 事务成功率
- 慢查询统计

## 🔧 运维部署

### 系统要求
- **操作系统**: Linux (推荐 Ubuntu 20.04+)
- **内存**: 最少 2GB，推荐 8GB+
- **CPU**: 最少 2 核，推荐 8 核+
- **存储**: SSD 推荐
- **网络**: 千兆网络

### 生产配置
```toml
[server]
worker_threads = 16
enable_tls = true
tls_cert_path = "/etc/ssl/certs/server.crt"
tls_key_path = "/etc/ssl/private/server.key"

[async_pool]
max_concurrent_requests = 20000
worker_count = 32

[monitoring]
enable_export = true
export_endpoint = "http://prometheus:9090/metrics"
```

### 监控集成
- **Prometheus**: 指标收集
- **Grafana**: 可视化仪表板
- **ELK Stack**: 日志分析
- **Alertmanager**: 告警管理

## 📚 文档

- [性能优化总结](FINAL_PERFORMANCE_SUMMARY.md)
- [并发改进文档](CONCURRENCY_IMPROVEMENTS.md)
- [配置示例](config.example.toml)
- [API 文档](docs/api.md)

## 🤝 贡献

欢迎贡献代码、报告问题或提出改进建议。

### 开发环境
```bash
# 克隆仓库
git clone https://github.com/stalwartlabs/stalwart-dav
cd stalwart-dav

# 安装依赖
cargo build

# 运行测试
cargo test

# 启动开发服务器
cargo run
```

## 📄 许可证

AGPL-3.0-only OR LicenseRef-SEL

---

**A3Mailer DAV Server** - 为企业级 WebDAV/CalDAV/CardDAV 服务提供生产就绪的高性能解决方案。
