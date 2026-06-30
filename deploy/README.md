# 有毒订单监控系统 - 部署和问题修复工具

## 概述

本目录包含用于部署和修复有毒订单监控系统的工具脚本。如果服务器上的前端页面无法访问，可以使用这些工具快速排查和解决问题。

## 文件说明

- `troubleshoot.sh` - Linux系统上的问题排查脚本
- `troubleshoot.ps1` - Windows系统上的问题排查脚本
- `quick-fix.sh` - 快速修复脚本（自动尝试最常见的解决方案）
- `TROUBLESHOOTING.md` - 详细的问题排查和修复指南
- `README.md` - 本文档

## 快速开始

### 第一步：上传文件到服务器

将整个项目（包括 `deploy/` 目录）上传到服务器。

### 第二步：快速修复（推荐先试这个）

在服务器上，进入项目目录后运行：

```bash
# 给脚本添加执行权限
chmod +x deploy/quick-fix.sh

# 运行快速修复脚本
./deploy/quick-fix.sh
```

这个脚本会自动：
1. 检查并创建 `.env` 文件
2. 停止现有服务
3. 清理Docker缓存
4. 重新构建并启动服务
5. 验证服务状态

### 第三步：如果快速修复不行，运行详细排查

```bash
chmod +x deploy/troubleshoot.sh
./deploy/troubleshoot.sh
```

这个脚本会详细检查各个方面并给出修复建议。

### 第四步：查看详细文档

如果仍然有问题，请查看 `TROUBLESHOOTING.md` 获取完整的问题排查指南。

## 使用指南

### Linux/Mac系统

```bash
# 进入项目目录
cd /path/to/有毒订单监控-rs

# 方式1：快速修复（推荐）
./deploy/quick-fix.sh

# 方式2：详细排查
./deploy/troubleshoot.sh
```

### Windows系统（PowerShell）

```powershell
# 进入项目目录
cd C:\path\to\有毒订单监控-rs

# 运行PowerShell排查脚本
.\deploy\troubleshoot.ps1
```

## 部署后的验证

修复完成后，请验证：

1. **检查容器状态**
   ```bash
   docker ps
   ```
   应该看到 `toxic-bot` 和 `toxic-frontend` 都在运行。

2. **检查端口监听**
   ```bash
   netstat -tlnp | grep -E "5173|5174"
   ```

3. **本地访问测试**
   ```bash
   curl http://localhost:5173/contract-whale
   curl http://localhost:5173/dashboard
   ```

4. **浏览器访问**
   在浏览器中访问 `http://服务器IP:5173/dashboard`

## 常见问题快速解答

### Q: 前端页面显示 404 或 502？
A: 先运行 `quick-fix.sh` 重新构建和部署，再执行 `scripts/check_frontend_prod.sh` 检查 `/contract-whale`、`/dashboard` 和 `/api/*`。

### Q: 容器启动了但无法访问？
A: 先检查宿主机 nginx 是否已加载 `deploy/nginx-site.toxic-order-monitor.conf`，再确认 5173 端口已开放。现在公网入口由宿主 nginx 统一接入，但前端页面本身会反代到 `127.0.0.1:5174` 的 `toxic-frontend` 容器，所以要同时确认 nginx 和前端容器都健康。

### Q: 提示 OPERATOR_TOKEN 未设置？
A: 编辑 `.env` 文件，设置一个安全的令牌值。

### Q: 前端可以打开但无法连接后端？
A: 检查容器间网络连接，确保两个容器都在同一个Docker网络中。

## Git同步

修复问题后，如果需要将更改同步回git：

```bash
# 检查状态
git status

# 添加更改
git add .

# 提交更改
git commit -m "修复部署问题"

# 推送到远程仓库
git push
```

## 获取帮助

- 查看 `TROUBLESHOOTING.md` 了解详细的排查步骤
- 查看项目根目录的 `README.md` 了解项目概述
- 查看 `docs/server-deployment-runbook.md` 了解服务器部署的详细说明
