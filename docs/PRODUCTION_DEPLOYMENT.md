# A3Mailer Mail Server - 生产环境部署指南

本指南提供了在生产环境中部署 A3Mailer Mail Server 的详细说明，包括性能优化、安全加固、监控配置和运维最佳实践。

## 📋 目录

- [系统要求](#系统要求)
- [安装部署](#安装部署)
- [配置管理](#配置管理)
- [性能优化](#性能优化)
- [安全配置](#安全配置)
- [监控设置](#监控设置)
- [备份策略](#备份策略)
- [故障排除](#故障排除)
- [运维指南](#运维指南)

## 🖥️ 系统要求

### 最低配置
- **CPU**: 2 核心 (推荐 4+ 核心)
- **内存**: 4GB RAM (推荐 8GB+)
- **存储**: 50GB SSD (推荐 100GB+ NVMe SSD)
- **网络**: 100Mbps (推荐 1Gbps+)
- **操作系统**: Linux (Ubuntu 20.04+, CentOS 8+, RHEL 8+)

### 推荐配置（高负载环境）
- **CPU**: 8+ 核心
- **内存**: 16GB+ RAM
- **存储**: 500GB+ NVMe SSD
- **网络**: 10Gbps
- **负载均衡**: 多实例部署

### 依赖软件
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev postgresql-client redis-tools

# CentOS/RHEL
sudo yum groupinstall -y "Development Tools"
sudo yum install -y openssl-devel postgresql redis
```

## 🚀 安装部署

### 1. 编译安装

```bash
# 克隆代码库
git clone https://github.com/stalwartlabs/mail-server.git
cd mail-server

# 编译生产版本
cargo build --release --features production

# 安装二进制文件
sudo cp target/release/stalwart-mail /usr/local/bin/
sudo chmod +x /usr/local/bin/stalwart-mail
```

### 2. 创建系统用户

```bash
# 创建专用用户
sudo useradd -r -s /bin/false -d /var/lib/stalwart stalwart

# 创建目录结构
sudo mkdir -p /etc/stalwart
sudo mkdir -p /var/lib/stalwart
sudo mkdir -p /var/log/stalwart
sudo mkdir -p /var/run/stalwart

# 设置权限
sudo chown -R stalwart:stalwart /var/lib/stalwart
sudo chown -R stalwart:stalwart /var/log/stalwart
sudo chown -R stalwart:stalwart /var/run/stalwart
sudo chown -R root:stalwart /etc/stalwart
sudo chmod 750 /etc/stalwart
```

### 3. 系统服务配置

创建 systemd 服务文件：

```bash
sudo tee /etc/systemd/system/stalwart-mail.service > /dev/null <<EOF
[Unit]
Description=A3Mailer Mail Server
Documentation=https://stalw.art/docs
After=network.target postgresql.service redis.service
Wants=postgresql.service redis.service

[Service]
Type=notify
User=stalwart
Group=stalwart
ExecStart=/usr/local/bin/stalwart-mail --config /etc/stalwart/config.toml
ExecReload=/bin/kill -HUP \$MAINPID
KillMode=mixed
KillSignal=SIGTERM
TimeoutStopSec=30
Restart=always
RestartSec=5
StartLimitInterval=60
StartLimitBurst=3

# 安全设置
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/stalwart /var/log/stalwart /var/run/stalwart
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
AmbientCapabilities=CAP_NET_BIND_SERVICE

# 资源限制
LimitNOFILE=65536
LimitNPROC=4096
MemoryMax=8G

[Install]
WantedBy=multi-user.target
EOF
```

启用并启动服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable stalwart-mail
sudo systemctl start stalwart-mail
```

## ⚙️ 配置管理

### 主配置文件 `/etc/stalwart/config.toml`

```toml
# A3Mailer Mail Server 生产配置

[server]
hostname = "mail.example.com"
bind = ["0.0.0.0:25", "0.0.0.0:587", "0.0.0.0:993", "0.0.0.0:995"]
max_connections = 10000
timeout = 300

[tls]
certificate = "/etc/stalwart/certs/fullchain.pem"
private-key = "/etc/stalwart/certs/privkey.pem"
protocols = ["TLSv1.2", "TLSv1.3"]
ciphers = "ECDHE+AESGCM:ECDHE+CHACHA20:DHE+AESGCM:DHE+CHACHA20:!aNULL:!MD5:!DSS"

[database]
type = "postgresql"
host = "localhost"
port = 5432
database = "stalwart"
username = "stalwart"
password = "your_secure_password"
pool_size = 20
timeout = 30

[cache]
type = "redis"
host = "localhost"
port = 6379
database = 0
pool_size = 10
timeout = 5

[smtp]
max_message_size = 50_000_000  # 50MB
max_recipients = 100
relay_host = "smtp.example.com"
relay_port = 587
relay_auth = true

[imap]
max_connections_per_user = 10
idle_timeout = 1800
search_timeout = 30

[pop3]
max_connections_per_user = 5
timeout = 600

[security]
enable_rate_limiting = true
max_requests_per_minute = 1000
enable_security_headers = true
enable_audit_logging = true
blocked_ips = []
trusted_proxies = ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"]

[monitoring]
enabled = true
prometheus_port = 9090
health_check_interval = 30
metrics_retention = 86400  # 24 hours

[logging]
level = "info"
format = "json"
output = "/var/log/stalwart/stalwart.log"
max_size = "100MB"
max_files = 10
```

### 环境变量配置

创建 `/etc/stalwart/environment`:

```bash
# 数据库配置
STALWART_DB_PASSWORD=your_secure_password
STALWART_REDIS_PASSWORD=your_redis_password

# TLS 证书路径
STALWART_TLS_CERT=/etc/stalwart/certs/fullchain.pem
STALWART_TLS_KEY=/etc/stalwart/certs/privkey.pem

# 监控配置
STALWART_PROMETHEUS_ENABLED=true
STALWART_METRICS_PORT=9090

# 安全配置
STALWART_SECURITY_ENABLED=true
STALWART_AUDIT_ENABLED=true
```

## 🚄 性能优化

### 1. 系统级优化

```bash
# 内核参数优化
sudo tee /etc/sysctl.d/99-stalwart.conf > /dev/null <<EOF
# 网络优化
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv4.tcp_rmem = 4096 87380 134217728
net.ipv4.tcp_wmem = 4096 65536 134217728
net.ipv4.tcp_congestion_control = bbr
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_max_syn_backlog = 8192

# 文件描述符限制
fs.file-max = 2097152
fs.nr_open = 2097152

# 内存管理
vm.swappiness = 10
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5
EOF

sudo sysctl -p /etc/sysctl.d/99-stalwart.conf
```

### 2. 应用级优化

```toml
# 在 config.toml 中添加性能配置
[performance]
# 连接池优化
connection_pool_size = 50
connection_pool_timeout = 30
connection_pool_max_idle = 10

# 缓存优化
cache_size = "1GB"
cache_ttl = 3600
cache_compression = true

# 并发优化
worker_threads = 8  # CPU 核心数
max_concurrent_requests = 10000
request_timeout = 300

# I/O 优化
io_buffer_size = 65536
batch_size = 1000
async_io = true
```

### 3. 数据库优化

PostgreSQL 配置优化：

```sql
-- postgresql.conf 优化建议
shared_buffers = 2GB                    -- 25% of RAM
effective_cache_size = 6GB              -- 75% of RAM
work_mem = 64MB
maintenance_work_mem = 512MB
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1                  -- SSD 优化
effective_io_concurrency = 200          -- SSD 优化
```

### 4. Redis 优化

```conf
# redis.conf 优化
maxmemory 2gb
maxmemory-policy allkeys-lru
save 900 1
save 300 10
save 60 10000
tcp-keepalive 300
timeout 0
```

## 🔒 安全配置

### 1. TLS/SSL 配置

```bash
# 使用 Let's Encrypt 获取证书
sudo apt install certbot
sudo certbot certonly --standalone -d mail.example.com

# 设置证书自动更新
sudo crontab -e
# 添加以下行
0 2 * * * /usr/bin/certbot renew --quiet && systemctl reload stalwart-mail
```

### 2. 防火墙配置

```bash
# UFW 防火墙规则
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow 25/tcp    # SMTP
sudo ufw allow 587/tcp   # SMTP Submission
sudo ufw allow 993/tcp   # IMAPS
sudo ufw allow 995/tcp   # POP3S
sudo ufw allow 9090/tcp  # Prometheus (限制访问)
sudo ufw enable
```

### 3. 安全加固

```toml
# 安全配置增强
[security]
# 速率限制
enable_rate_limiting = true
rate_limit_window = 60
max_requests_per_window = 1000
max_connections_per_ip = 100

# 输入验证
enable_input_validation = true
max_request_size = 50_000_000
strict_validation = true

# 安全头部
enable_security_headers = true
hsts_max_age = 31536000
enable_csp = true
csp_policy = "default-src 'self'; script-src 'self' 'unsafe-inline'"

# 审计日志
enable_audit_logging = true
audit_log_level = "info"
audit_retention_days = 90

# IP 黑名单
blocked_ips = [
    "192.0.2.0/24",    # 示例恶意 IP 段
]

# 可信代理
trusted_proxies = [
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16"
]
```

### 4. 访问控制

```bash
# 创建访问控制列表
sudo tee /etc/stalwart/access.conf > /dev/null <<EOF
# 允许的发送域
allow_domains = [
    "example.com",
    "trusted-partner.com"
]

# 阻止的发送域
block_domains = [
    "spam-domain.com",
    "malicious-site.net"
]

# 地理位置限制
allow_countries = ["US", "CA", "GB", "DE", "FR"]
block_countries = ["CN", "RU", "KP"]
EOF
```

## 📊 监控设置

### 1. Prometheus 配置

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'stalwart-mail'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 30s
    metrics_path: /metrics
```

### 2. Grafana 仪表板

创建 Grafana 仪表板配置：

```json
{
  "dashboard": {
    "title": "A3Mailer Mail Server",
    "panels": [
      {
        "title": "系统指标",
        "targets": [
          {
            "expr": "stalwart_cpu_usage_percent",
            "legendFormat": "CPU 使用率"
          },
          {
            "expr": "stalwart_memory_usage_bytes / 1024 / 1024 / 1024",
            "legendFormat": "内存使用 (GB)"
          }
        ]
      },
      {
        "title": "邮件处理",
        "targets": [
          {
            "expr": "rate(stalwart_emails_processed_total[5m])",
            "legendFormat": "邮件处理速率"
          },
          {
            "expr": "stalwart_queue_size",
            "legendFormat": "队列大小"
          }
        ]
      }
    ]
  }
}
```

### 3. 告警规则

```yaml
# alerting_rules.yml
groups:
  - name: stalwart_alerts
    rules:
      - alert: HighCPUUsage
        expr: stalwart_cpu_usage_percent > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Stalwart CPU 使用率过高"
          description: "CPU 使用率已超过 80% 持续 5 分钟"

      - alert: HighMemoryUsage
        expr: stalwart_memory_usage_percent > 85
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Stalwart 内存使用率过高"
          description: "内存使用率已超过 85% 持续 5 分钟"

      - alert: ServiceDown
        expr: up{job="stalwart-mail"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "A3Mailer Mail Server 服务停止"
          description: "A3Mailer Mail Server 无法访问"

      - alert: HighErrorRate
        expr: rate(stalwart_errors_total[5m]) > 10
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "错误率过高"
          description: "错误率超过每分钟 10 个错误"
```

### 4. 健康检查

```bash
# 创建健康检查脚本
sudo tee /usr/local/bin/stalwart-health-check.sh > /dev/null <<'EOF'
#!/bin/bash

# 检查服务状态
if ! systemctl is-active --quiet stalwart-mail; then
    echo "ERROR: Stalwart service is not running"
    exit 1
fi

# 检查端口监听
for port in 25 587 993 995; do
    if ! netstat -ln | grep -q ":$port "; then
        echo "ERROR: Port $port is not listening"
        exit 1
    fi
done

# 检查数据库连接
if ! pg_isready -h localhost -p 5432 -U stalwart; then
    echo "ERROR: Database connection failed"
    exit 1
fi

# 检查 Redis 连接
if ! redis-cli ping > /dev/null 2>&1; then
    echo "ERROR: Redis connection failed"
    exit 1
fi

echo "OK: All health checks passed"
exit 0
EOF

sudo chmod +x /usr/local/bin/stalwart-health-check.sh
```

## 💾 备份策略

### 1. 数据库备份

```bash
# 创建备份脚本
sudo tee /usr/local/bin/stalwart-backup.sh > /dev/null <<'EOF'
#!/bin/bash

BACKUP_DIR="/var/backups/stalwart"
DATE=$(date +%Y%m%d_%H%M%S)
DB_NAME="stalwart"
DB_USER="stalwart"

# 创建备份目录
mkdir -p "$BACKUP_DIR"

# 数据库备份
pg_dump -h localhost -U "$DB_USER" -d "$DB_NAME" | gzip > "$BACKUP_DIR/db_backup_$DATE.sql.gz"

# 配置文件备份
tar -czf "$BACKUP_DIR/config_backup_$DATE.tar.gz" /etc/stalwart/

# 日志备份
tar -czf "$BACKUP_DIR/logs_backup_$DATE.tar.gz" /var/log/stalwart/

# 清理旧备份（保留 30 天）
find "$BACKUP_DIR" -name "*.gz" -mtime +30 -delete

echo "Backup completed: $DATE"
EOF

sudo chmod +x /usr/local/bin/stalwart-backup.sh

# 设置定时备份
sudo crontab -e
# 添加以下行（每天凌晨 2 点备份）
0 2 * * * /usr/local/bin/stalwart-backup.sh
```

### 2. 增量备份

```bash
# 增量备份脚本
sudo tee /usr/local/bin/stalwart-incremental-backup.sh > /dev/null <<'EOF'
#!/bin/bash

BACKUP_DIR="/var/backups/stalwart/incremental"
DATE=$(date +%Y%m%d_%H%M%S)
LAST_BACKUP_FILE="/var/lib/stalwart/.last_backup"

mkdir -p "$BACKUP_DIR"

# 获取上次备份时间
if [ -f "$LAST_BACKUP_FILE" ]; then
    LAST_BACKUP=$(cat "$LAST_BACKUP_FILE")
else
    LAST_BACKUP="1970-01-01 00:00:00"
fi

# 增量数据库备份
pg_dump -h localhost -U stalwart -d stalwart \
    --where="updated_at > '$LAST_BACKUP'" | \
    gzip > "$BACKUP_DIR/incremental_$DATE.sql.gz"

# 更新备份时间戳
echo "$(date '+%Y-%m-%d %H:%M:%S')" > "$LAST_BACKUP_FILE"

echo "Incremental backup completed: $DATE"
EOF

sudo chmod +x /usr/local/bin/stalwart-incremental-backup.sh
```

这个生产部署指南涵盖了系统要求、安装配置、性能优化、安全加固、监控设置和备份策略等关键方面。让我继续创建更多的文档。
