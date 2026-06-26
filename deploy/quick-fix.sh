#!/bin/bash

# 有毒订单监控系统 - 快速修复脚本
# 自动检测并修复最常见的问题

echo "=== 有毒订单监控系统 - 快速修复脚本 ==="
echo "开始时间: $(date)"
echo ""

# 检查是否在项目目录
if [ ! -f "docker-compose.yml" ]; then
    echo "错误: 请在项目根目录下运行此脚本"
    exit 1
fi

# 1. 确保 .env 文件存在
echo "1. 检查环境变量配置..."
if [ ! -f ".env" ]; then
    echo "创建 .env 文件..."
    if [ -f ".env.example" ]; then
        cp .env.example .env
        echo "已从 .env.example 创建 .env 文件"
    else
        cat > .env << 'EOF'
OPERATOR_TOKEN=change_this_to_a_secure_token
INTERNAL_API_ORIGIN=http://127.0.0.1:3000
DASHBOARD_BIND_HOST=127.0.0.1
RUST_LOG=info
EOF
        echo "已创建默认 .env 文件"
    fi
    echo "请编辑 .env 文件设置正确的 OPERATOR_TOKEN"
    echo ""
fi

# 2. 停止并重新构建所有服务
echo "2. 停止现有服务..."
docker-compose down 2>/dev/null || docker compose down 2>/dev/null
echo ""

echo "3. 清理Docker缓存..."
docker system prune -f 2>/dev/null
echo ""

echo "4. 重新构建并启动所有服务..."
if docker-compose up -d --build 2>&1; then
    echo "服务启动成功"
elif docker compose up -d --build 2>&1; then
    echo "服务启动成功"
else
    echo "启动失败，尝试不使用缓存重新构建..."
    docker-compose build --no-cache 2>/dev/null || docker compose build --no-cache
    docker-compose up -d 2>/dev/null || docker compose up -d
fi
echo ""

# 5. 等待服务启动
echo "5. 等待服务启动..."
sleep 10

# 6. 检查服务状态
echo "6. 检查服务状态..."
echo ""
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | grep toxic
echo ""

# 7. 检查端口监听
echo "7. 检查端口监听..."
if command -v netstat &> /dev/null; then
    netstat -tlnp 2>/dev/null | grep -E "5173|5174" || echo "5173/5174端口可能未监听"
elif command -v ss &> /dev/null; then
    ss -tlnp 2>/dev/null | grep -E "5173|5174" || echo "5173/5174端口可能未监听"
else
    echo "无法检查端口状态"
fi
echo ""

# 8. 测试本地访问
echo "8. 测试本地访问..."
if curl -s -o /dev/null -w "%{http_code}" http://localhost:5173/dashboard 2>/dev/null | grep -q "200\|301\|302"; then
    echo "宿主机 nginx 公网入口可以在本地访问"
    echo "现在可以在浏览器中访问: http://$(hostname -I | awk '{print $1}'):5173/dashboard"
else
    echo "宿主机 nginx 入口无法在本地访问，请查看日志"
    echo ""
    echo "前端日志:"
    docker logs --tail 30 toxic-frontend 2>/dev/null
    echo ""
    echo "后端日志:"
    docker logs --tail 30 toxic-bot 2>/dev/null
fi
echo ""

echo "快速修复脚本执行完成: $(date)"
echo ""
echo "如果问题仍然存在，请运行: ./deploy/troubleshoot.sh"
