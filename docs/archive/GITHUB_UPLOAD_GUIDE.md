# GitHub 上传指南 - A3Mailer 项目

## 🔐 安全认证方式

### 方法 1: GitHub CLI (最推荐) ⭐

#### 1.1 安装 GitHub CLI
```bash
# macOS
brew install gh

# Windows (使用 Chocolatey)
choco install gh

# Linux (Ubuntu/Debian)
sudo apt install gh

# 或从官网下载: https://cli.github.com/
```

#### 1.2 认证和上传
```bash
# 1. 认证登录
gh auth login
# 选择: GitHub.com
# 选择: HTTPS
# 选择: Login with a web browser
# 按照提示在浏览器中登录

# 2. 验证认证状态
gh auth status

# 3. 运行上传脚本
./push_to_github.sh
```

### 方法 2: Personal Access Token (推荐) ⭐

#### 2.1 创建 Personal Access Token
1. 访问: https://github.com/settings/tokens
2. 点击 "Generate new token (classic)"
3. 设置名称: "A3Mailer Upload"
4. 选择权限:
   - ✅ `repo` (完整仓库访问)
   - ✅ `workflow` (GitHub Actions)
   - ✅ `write:packages` (如果需要发布包)
5. 点击 "Generate token"
6. **重要**: 复制并保存令牌 (只显示一次)

#### 2.2 使用 Token 上传
```bash
# 1. 设置远程仓库 URL (使用您的 token)
git remote set-url origin https://YOUR_TOKEN@github.com/arkCyber/A3Mailer.git

# 2. 推送到 GitHub
git push -u origin main
```

### 方法 3: SSH Key (高级用户)

#### 3.1 生成 SSH Key
```bash
# 生成新的 SSH key
ssh-keygen -t ed25519 -C "arksong2018@gmail.com"

# 启动 ssh-agent
eval "$(ssh-agent -s)"

# 添加 SSH key
ssh-add ~/.ssh/id_ed25519
```

#### 3.2 添加到 GitHub
1. 复制公钥: `cat ~/.ssh/id_ed25519.pub`
2. 访问: https://github.com/settings/keys
3. 点击 "New SSH key"
4. 粘贴公钥内容
5. 保存

#### 3.3 使用 SSH 上传
```bash
# 1. 设置 SSH 远程 URL
git remote set-url origin git@github.com:arkCyber/A3Mailer.git

# 2. 推送到 GitHub
git push -u origin main
```

## 🚀 快速上传步骤

### 选项 A: 使用 GitHub CLI (最简单)
```bash
# 1. 安装并认证 GitHub CLI
gh auth login

# 2. 运行上传脚本
./push_to_github.sh
```

### 选项 B: 使用 Personal Access Token
```bash
# 1. 创建 PAT (见上方步骤)
# 2. 替换 YOUR_TOKEN 为实际令牌
git remote set-url origin https://YOUR_TOKEN@github.com/arkCyber/A3Mailer.git

# 3. 推送
git push -u origin main
```

## 🔍 故障排除

### 问题 1: 认证失败
```bash
# 解决方案: 检查认证状态
gh auth status
# 或重新认证
gh auth login
```

### 问题 2: 仓库不存在
```bash
# 解决方案: 创建仓库
gh repo create arkCyber/A3Mailer --public
```

### 问题 3: 权限被拒绝
```bash
# 解决方案: 检查 token 权限或重新生成
# 确保 token 有 'repo' 权限
```

### 问题 4: 网络连接问题
```bash
# 解决方案: 检查网络连接
ping github.com

# 或使用代理
git config --global http.proxy http://proxy.example.com:8080
```

## 📋 上传后的验证

上传成功后，您可以访问:
- **仓库地址**: https://github.com/arkCyber/A3Mailer
- **项目主页**: https://arkCyber.github.io/A3Mailer (如果启用了 Pages)

## 🎯 下一步操作

上传成功后建议:
1. **设置仓库描述**: "AI-Powered Web3-Native Mail Server"
2. **添加主题标签**: `ai`, `web3`, `blockchain`, `rust`, `email-server`
3. **启用 GitHub Pages**: 用于项目文档
4. **设置 GitHub Actions**: 自动化 CI/CD
5. **创建 Release**: 发布第一个版本

## 🔒 安全提醒

- ❌ 永远不要在代码中硬编码密码或令牌
- ✅ 使用环境变量存储敏感信息
- ✅ 定期轮换 Personal Access Token
- ✅ 使用最小权限原则设置 token 权限

---

如果遇到任何问题，请参考 GitHub 官方文档或联系技术支持。
