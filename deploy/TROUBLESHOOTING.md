# 有毒订单监控系统 - 前端问题排查和修复指南

## 问题概述

如果服务器上访问 `http://服务器IP:5173/dashboard` 打不开前端页面，请按照本指南进行排查和修复。

## 快速开始

### 1. 上传文件到服务器

首先将以下文件上传到服务器：

```
有毒订单监控-rs/
├── deploy/
│   ├── troubleshoot.sh          # Linux排查脚本
│   ├── troubleshoot.ps1         # Windows排查脚本
│   └── TROUBLESHOOTING.md      # 本文档
└── ... (其他项目文件)
```

### 2. 运行排查脚本

#### Linux服务器

```bash
chmod +x deploy/troubleshoot.sh
./deploy/troubleshoot.sh
```

#### Windows服务器

```powershell
.\deploy\troubleshoot.ps1
```

## 常见问题和解决方案

### 问题1: 前端容器未启动或启动失败

**症状**: `docker ps` 中看不到 `toxic-frontend` 容器

**解决方案**:

```bash
# 查看详细日志
docker logs toxic-frontend

# 重新启动容器
docker-compose up -d frontend
# 或者使用新的Docker命令
docker compose up -d frontend
```

### 问题2: Docker构建失败

**症状**: `docker-compose build frontend` 失败

**解决方案**:

```bash
# 1. 检查是否有足够的内存
free -h

# 2. 清理Docker缓存
docker system prune -f

# 3. 重新构建（不使用缓存）
docker-compose build --no-cache frontend

# 4. 启动服务
docker-compose up -d
```

### 问题3: Nginx配置问题

**症状**: 容器启动了但页面打不开，Nginx报错

**解决方案**:

```bash
# 检查Nginx配置
docker exec toxic-frontend nginx -t

# 查看实际的Nginx配置文件
docker exec toxic-frontend cat /etc/nginx/conf.d/default.conf

# 检查环境变量是否正确传递
docker exec toxic-frontend env | grep -E "(OPERATOR_TOKEN|INTERNAL_API_ORIGIN)"
```

### 问题4: 环境变量未设置

**症状**: `.env` 文件不存在或缺少关键配置

**解决方案**:

```bash
# 1. 创建或编辑 .env 文件
cd /path/to/项目目录
cat > .env << 'EOF'
OPERATOR_TOKEN=your_secure_token_here
INTERNAL_API_ORIGIN=http://127.0.0.1:3000
DASHBOARD_BIND_HOST=127.0.0.1
RUST_LOG=info
EOF

# 2. 重新启动容器
docker-compose up -d --force-recreate
```

### 问题5: 防火墙阻止端口

**症状**: 本地容器健康但外网打不开页面

**解决方案**:

```bash
# 1. 先确保宿主机 nginx 已加载 deploy/nginx-site.toxic-order-monitor.conf
sudo cp deploy/nginx-site.toxic-order-monitor.conf /etc/nginx/sites-enabled/toxic-order-monitor
sudo nginx -t && sudo systemctl reload nginx

# 2. Ubuntu/Debian - UFW
sudo ufw allow 5173

# 3. CentOS/RHEL - firewalld
sudo firewall-cmd --permanent --add-port=5173/tcp
sudo firewall-cmd --reload

# 或者暂时关闭防火墙测试
sudo ufw disable  # 仅用于测试
```

### 问题6: 容器间网络问题

**症状**: 前端无法连接后端

**解决方案**:

```bash
# 检查容器网络
docker network ls

# 测试容器间连接
docker exec toxic-frontend curl http://backend:3000

# 如果网络有问题，重新创建网络
docker-compose down
docker-compose up -d
```

### 问题7: 前端构建输出问题

**症状**: Nginx能启动但页面显示404

**解决方案**:

```bash
# 检查构建输出目录
docker exec toxic-frontend ls -la /usr/share/nginx/html/

# 检查是否有 index.html
docker exec toxic-frontend cat /usr/share/nginx/html/index.html

# 如果文件不存在，重新构建
docker-compose build --no-cache frontend
docker-compose up -d frontend
```

### 问题8: 端口绑定问题

**症状**: 5173/5174端口未被监听

**解决方案**:

```bash
# 检查端口占用
netstat -tlnp | grep 5173
# 或
ss -tlnp | grep 5173

# 检查 docker-compose.yml 中的端口映射
# 容器应只绑定宿主机本地: "127.0.0.1:5174:5173"
# 公网5173由宿主机nginx接管，不再由docker直接暴露

# 重新启动
docker-compose down
docker-compose up -d
```

## 手动排查步骤

如果脚本无法解决问题，请按以下步骤手动排查：

### 步骤1: 检查容器状态

```bash
# 查看所有容器
docker ps -a

# 查看容器详细状态
docker inspect toxic-frontend
```

### 步骤2: 查看容器日志

```bash
# 查看前端日志
docker logs -f toxic-frontend

# 查看后端日志
docker logs -f toxic-bot
```

### 步骤3: 进入容器检查

```bash
# 进入前端容器
docker exec -it toxic-frontend sh

# 检查nginx是否运行
ps aux | grep nginx

# 检查端口监听
netstat -tlnp

# 检查文件
ls -la /usr/share/nginx/html/
```

### 步骤4: 测试前端是否正常

```bash
# 在服务器本地测试宿主机入口
curl -v http://localhost:5173/dashboard

# 检查HTTP状态码
curl -o /dev/null -s -w "%{http_code}\n" http://localhost:5173/dashboard
```

### 步骤5: 检查网络和防火墙

```bash
# 检查防火墙状态
sudo ufw status

# 检查iptables规则
sudo iptables -L -n

# 检查端口是否可从外部访问
# 在另一台机器上运行
telnet 服务器IP 5173
# 或
nc -zv 服务器IP 5173
```

## 完整重新部署流程

如果以上方法都无法解决问题，尝试完全重新部署：

```bash
# 1. 停止并删除所有容器
cd /path/to/项目目录
docker-compose down

# 2. 删除旧的镜像（可选）
docker rmi $(docker images | grep toxic)

# 3. 清理Docker缓存
docker system prune -f

# 4. 确保.env文件存在且配置正确
if [ ! -f .env ]; then
    cp .env.example .env
    # 编辑.env文件，设置OPERATOR_TOKEN
fi

# 5. 重新构建和启动
docker-compose up -d --build

# 6. 查看日志
docker-compose logs -f
```

## 验证修复成功

修复完成后，进行以下验证：

1. **容器状态检查**:
   ```bash
   docker ps | grep toxic
   ```
   应该看到两个容器都在运行。

2. **端口监听检查**:
   ```bash
   netstat -tlnp | grep -E "5173|5174"
   ```
   应该看到宿主机 5173 和容器上游 5174 在监听。

3. **本地访问检查**:
   ```bash
   curl http://localhost:5173/dashboard
   ```
   应该能返回HTML内容。

4. **浏览器访问检查**:
   在浏览器中访问 `http://服务器IP:5173/dashboard`，应该能看到前端页面。

## 预防措施

1. **定期备份配置**:
   ```bash
   cp .env .env.backup.$(date +%Y%m%d)
   ```

2. **监控日志**:
   ```bash
   docker-compose logs -f --tail 100
   ```

3. **使用健康检查**:
   在 `docker-compose.yml` 中为前端添加健康检查。

4. **资源监控**:
   确保服务器有足够的内存和CPU资源。

## 获取帮助

如果仍然无法解决问题：

1. 收集以下信息：
   - 操作系统版本
   - Docker版本
   - `docker ps -a` 输出
   - `docker logs toxic-frontend` 输出
   - `docker logs toxic-bot` 输出

2. 检查项目文档：
   - `README.md`
   - `docs/server-deployment-runbook.md`

3. 检查项目Issue或文档。
