#!/bin/bash

# 有毒订单监控系统 - 前端问题排查和修复脚本
# 使用方法: ./deploy/troubleshoot.sh

echo "=== 有毒订单监控系统前端问题排查和修复脚本 ==="
echo "开始时间: $(date)"
echo ""

# 1. 检查Docker和Docker Compose是否安装
echo "1. 检查Docker和Docker Compose..."
if ! command -v docker &> /dev/null; then
    echo "错误: Docker未安装"
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "错误: Docker Compose未安装"
    exit 1
fi

echo "Docker和Docker Compose已安装"
echo ""

# 2. 检查容器状态
echo "2. 检查容器状态..."
docker ps -a
echo ""

# 3. 检查容器日志
echo "3. 检查前端容器日志..."
docker logs toxic-frontend --tail 50 2>/dev/null || echo "前端容器可能未运行"
echo ""

echo "3. 检查后端容器日志..."
docker logs toxic-bot --tail 50 2>/dev/null || echo "后端容器可能未运行"
echo ""

# 4. 检查前端镜像
echo "4. 检查前端镜像..."
docker images | grep toxic
echo ""

# 5. 检查端口监听
echo "5. 检查端口监听状态..."
if command -v netstat &> /dev/null; then
    netstat -tlnp 2>/dev/null | grep -E "5173|5174"
elif command -v ss &> /dev/null; then
    ss -tlnp 2>/dev/null | grep -E "5173|5174"
else
    echo "无法找到网络工具，请手动检查"
fi
echo ""

# 6. 检查防火墙
echo "6. 检查防火墙状态..."
if command -v sudo &> /dev/null && command -v ufw &> /dev/null; then
    sudo ufw status 2>/dev/null || echo "UFW未安装或未运行"
elif command -v firewall-cmd &> /dev/null; then
    firewall-cmd --list-all 2>/dev/null || echo "firewalld未安装或未运行"
fi
echo ""

# 7. 测试容器间连接
echo "7. 测试容器间网络连接..."
if docker ps | grep -q toxic-frontend && docker ps | grep -q toxic-bot; then
    if docker exec toxic-frontend curl -s -o /dev/null -w "%{http_code}" http://backend:3000 2>/dev/null; then
        echo "前端容器可以访问后端容器"
    else
        echo "警告: 前端容器无法访问后端容器"
    fi
else
    echo "容器可能未运行，跳过网络测试"
fi
echo ""

# 8. 检查环境变量
echo "8. 检查环境变量..."
if docker ps | grep -q toxic-frontend; then
    echo "前端容器环境变量:"
    docker exec toxic-frontend env 2>/dev/null | grep -E "(OPERATOR_TOKEN|INTERNAL_API_ORIGIN)" || echo "未找到相关环境变量"
else
    echo "前端容器未运行"
fi
echo ""

# 9. 检查nginx配置
echo "9. 检查nginx配置..."
if docker ps | grep -q toxic-frontend; then
    docker exec toxic-frontend nginx -t 2>&1
    echo "当前nginx配置:"
    docker exec toxic-frontend cat /etc/nginx/conf.d/default.conf 2>/dev/null || cat /etc/nginx/templates/default.conf.template 2>/dev/null
else
    echo "前端容器未运行"
fi
echo ""

# 10. 检查前端构建结果
echo "10. 检查前端构建结果..."
if docker ps | grep -q toxic-frontend; then
    if docker exec toxic-frontend test -d /usr/share/nginx/html && docker exec toxic-frontend test -f /usr/share/nginx/html/index.html; then
        echo "前端文件存在"
        docker exec toxic-frontend ls -la /usr/share/nginx/html/
    else
        echo "警告: 前端文件可能不存在或构建不完整"
    fi
else
    echo "前端容器未运行"
fi
echo ""

# 11. 修复建议
echo "=== 修复建议 ==="
echo ""

# 检查容器是否运行
if ! docker ps | grep -q toxic-frontend; then
    echo "1. 前端容器未运行，尝试启动..."
    docker-compose up -d frontend 2>&1 || docker compose up -d frontend 2>&1
    echo ""
fi

if ! docker ps | grep -q toxic-bot; then
    echo "2. 后端容器未运行，尝试启动..."
    docker-compose up -d backend 2>&1 || docker compose up -d backend 2>&1
    echo ""
fi

# 检查环境变量
if [ ! -f .env ]; then
    echo "3. .env文件不存在，创建示例..."
    if [ -f .env.example ]; then
        cp .env.example .env
        echo "已从.env.example创建.env文件"
        echo "请编辑.env文件设置OPERATOR_TOKEN"
    else
        echo "请创建.env文件并设置OPERATOR_TOKEN"
    fi
    echo ""
fi

# 重新构建前端容器
echo "4. 尝试重新构建前端容器..."
if docker-compose build --no-cache frontend 2>&1; then
    echo "构建成功"
    docker-compose up -d frontend 2>&1 || docker compose up -d frontend 2>&1
elif docker compose build --no-cache frontend 2>&1; then
    echo "构建成功"
    docker compose up -d frontend 2>&1
else
    echo "构建失败，请检查Dockerfile.frontend"
fi
echo ""

# 等待服务启动
echo "5. 等待服务启动..."
sleep 5

# 最终检查
echo "=== 最终检查 ==="
echo "检查前端服务是否运行..."
if docker ps | grep -q toxic-frontend; then
    echo "前端容器正在运行"
else
    echo "前端容器未运行"
fi

echo "检查宿主机 5173 和容器上游 5174 是否监听..."
if (command -v netstat &> /dev/null && netstat -tlnp 2>/dev/null | grep -Eq "5173|5174") || (command -v ss &> /dev/null && ss -tlnp 2>/dev/null | grep -Eq "5173|5174"); then
    echo "5173/5174端口正在监听"
else
    echo "5173/5174端口未监听"
fi

echo "检查前端页面是否可访问..."
if curl -s -o /dev/null -w "%{http_code}" http://localhost:5173/contract-whale 2>/dev/null | grep -q "200\|301\|302"; then
    echo "宿主机 nginx 公网入口可以访问"
else
    echo "宿主机 nginx 公网入口无法访问，请进一步检查"
fi

echo "脚本执行完成: $(date)"
