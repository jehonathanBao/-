# 有毒订单监控系统 - 前端问题排查和修复脚本 (Windows PowerShell)
# 使用方法: .\deploy\troubleshoot.ps1

Write-Host "=== 有毒订单监控系统前端问题排查和修复脚本 ===" -ForegroundColor Green
Write-Host "开始时间: $(Get-Date)" -ForegroundColor Gray
Write-Host ""

# 1. 检查Docker和Docker Compose是否安装
Write-Host "1. 检查Docker和Docker Compose..." -ForegroundColor Cyan
if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    Write-Host "错误: Docker未安装" -ForegroundColor Red
    exit 1
}

if (-not (Get-Command docker-compose -ErrorAction SilentlyContinue) -and -not (Get-Command docker compose -ErrorAction SilentlyContinue)) {
    Write-Host "错误: Docker Compose未安装" -ForegroundColor Red
    exit 1
}

Write-Host "Docker和Docker Compose已安装" -ForegroundColor Green
Write-Host ""

# 2. 检查容器状态
Write-Host "2. 检查容器状态..." -ForegroundColor Cyan
docker ps -a
Write-Host ""

# 3. 检查容器日志
Write-Host "3. 检查前端容器日志..." -ForegroundColor Cyan
try {
    docker logs toxic-frontend --tail 50
} catch {
    Write-Host "前端容器可能未运行" -ForegroundColor Yellow
}
Write-Host ""

Write-Host "3. 检查后端容器日志..." -ForegroundColor Cyan
try {
    docker logs toxic-bot --tail 50
} catch {
    Write-Host "后端容器可能未运行" -ForegroundColor Yellow
}
Write-Host ""

# 4. 检查前端镜像
Write-Host "4. 检查前端镜像..." -ForegroundColor Cyan
docker images | Select-String toxic
Write-Host ""

# 5. 检查端口监听
Write-Host "5. 检查端口监听状态..." -ForegroundColor Cyan
try {
    netstat -ano | Select-String "5173|5174"
} catch {
    Write-Host "无法检查端口，请手动查看" -ForegroundColor Yellow
}
Write-Host ""

# 6. 检查环境变量
Write-Host "6. 检查环境变量..." -ForegroundColor Cyan
try {
    Write-Host "前端容器环境变量:" -ForegroundColor Gray
    docker exec toxic-frontend env | Select-String "OPERATOR_TOKEN|INTERNAL_API_ORIGIN"
} catch {
    Write-Host "前端容器未运行" -ForegroundColor Yellow
}
Write-Host ""

# 7. 检查nginx配置
Write-Host "7. 检查nginx配置..." -ForegroundColor Cyan
try {
    docker exec toxic-frontend nginx -t
    Write-Host "当前nginx配置:" -ForegroundColor Gray
    docker exec toxic-frontend cat /etc/nginx/conf.d/default.conf 2>$null || docker exec toxic-frontend cat /etc/nginx/templates/default.conf.template
} catch {
    Write-Host "前端容器未运行" -ForegroundColor Yellow
}
Write-Host ""

# 8. 检查前端构建结果
Write-Host "8. 检查前端构建结果..." -ForegroundColor Cyan
try {
    if (docker exec toxic-frontend test -d /usr/share/nginx/html) {
        Write-Host "前端文件存在" -ForegroundColor Green
        docker exec toxic-frontend ls -la /usr/share/nginx/html/
    } else {
        Write-Host "警告: 前端文件可能不存在或构建不完整" -ForegroundColor Yellow
    }
} catch {
    Write-Host "前端容器未运行" -ForegroundColor Yellow
}
Write-Host ""

# 9. 修复建议
Write-Host "=== 修复建议 ===" -ForegroundColor Yellow
Write-Host ""

# 检查容器是否运行
$frontendRunning = docker ps --format "{{.Names}}" | Select-String "toxic-frontend"
$backendRunning = docker ps --format "{{.Names}}" | Select-String "toxic-bot"

if (-not $frontendRunning) {
    Write-Host "1. 前端容器未运行，尝试启动..." -ForegroundColor Cyan
    try {
        docker-compose up -d frontend
    } catch {
        try {
            docker compose up -d frontend
        } catch {
            Write-Host "启动失败" -ForegroundColor Red
        }
    }
    Write-Host ""
}

if (-not $backendRunning) {
    Write-Host "2. 后端容器未运行，尝试启动..." -ForegroundColor Cyan
    try {
        docker-compose up -d backend
    } catch {
        try {
            docker compose up -d backend
        } catch {
            Write-Host "启动失败" -ForegroundColor Red
        }
    }
    Write-Host ""
}

# 检查环境变量
if (-not (Test-Path .env)) {
    Write-Host "3. .env文件不存在，创建示例..." -ForegroundColor Cyan
    if (Test-Path .env.example) {
        Copy-Item .env.example .env
        Write-Host "已从.env.example创建.env文件" -ForegroundColor Green
        Write-Host "请编辑.env文件设置OPERATOR_TOKEN" -ForegroundColor Yellow
    } else {
        Write-Host "请创建.env文件并设置OPERATOR_TOKEN" -ForegroundColor Yellow
    }
    Write-Host ""
}

# 重新构建前端容器
Write-Host "4. 尝试重新构建前端容器..." -ForegroundColor Cyan
try {
    docker-compose build --no-cache frontend
    Write-Host "构建成功" -ForegroundColor Green
    docker-compose up -d frontend
} catch {
    try {
        docker compose build --no-cache frontend
        Write-Host "构建成功" -ForegroundColor Green
        docker compose up -d frontend
    } catch {
        Write-Host "构建失败，请检查Dockerfile.frontend" -ForegroundColor Red
    }
}
Write-Host ""

# 等待服务启动
Write-Host "5. 等待服务启动..." -ForegroundColor Cyan
Start-Sleep -Seconds 5

# 最终检查
Write-Host "=== 最终检查 ===" -ForegroundColor Yellow
Write-Host "检查前端服务是否运行..." -ForegroundColor Cyan
$frontendRunning = docker ps --format "{{.Names}}" | Select-String "toxic-frontend"
if ($frontendRunning) {
    Write-Host "前端容器正在运行" -ForegroundColor Green
} else {
    Write-Host "前端容器未运行" -ForegroundColor Red
}

Write-Host "检查宿主机5173和容器上游5174端口是否监听..." -ForegroundColor Cyan
$portListening = netstat -ano | Select-String "5173|5174"
if ($portListening) {
    Write-Host "5173/5174端口正在监听" -ForegroundColor Green
} else {
    Write-Host "5173/5174端口未监听" -ForegroundColor Red
}

Write-Host "脚本执行完成: $(Get-Date)" -ForegroundColor Gray
