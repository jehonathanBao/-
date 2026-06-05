# Production Reverse Proxy Example

The browser must not receive `OPERATOR_TOKEN`. For a static production frontend, terminate browser traffic at a reverse proxy and inject the operator token server-side.

Do not commit real tokens, webhook URLs, or secrets. The examples below use placeholders only.

## Nginx Example

```nginx
server {
    listen 80;
    server_name example.com;

    root /srv/toxic-order-monitor/frontend-dist;
    index index.html;

    location / {
        try_files $uri /index.html;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:3000/api/;
        proxy_set_header x-operator-api-token "REPLACE_WITH_SERVER_SIDE_SECRET";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }

    location /ws/ {
        proxy_pass http://127.0.0.1:3000/ws/;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
        proxy_set_header x-operator-api-token "REPLACE_WITH_SERVER_SIDE_SECRET";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Safer Secret Injection

Prefer loading the token from a server-local file, environment-rendered template, or secret manager. Keep the rendered config out of git.

Example template placeholder:

```nginx
proxy_set_header x-operator-api-token "${TOXIC_ORDER_OPERATOR_TOKEN}";
```

Use TLS for remote access. Add your normal authentication layer in front of the Dashboard, such as VPN, SSO, mTLS, Basic Auth, or an authenticated gateway.

## Forbidden

- Do not put `OPERATOR_TOKEN` in `VITE_*` variables.
- Do not embed `OPERATOR_TOKEN` in frontend JavaScript.
- Do not connect a browser directly to `http://backend:3000/ws/signals` with a token in the URL.
- Do not store real Discord or Telegram webhook secrets in this file.
