use sqlx::{Pool, Postgres};

#[derive(Debug, Clone)]
pub struct TenantScope<'a> {
    pub tenant_id: &'a str,
    pub shop_id: &'a str,
    pub user_id: &'a str,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct RiskOrderRow {
    pub order_id: String,
    pub buyer_id: Option<String>,
    pub shop_id: String,
    pub risk_level: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_risk_orders(
    pool: &Pool<Postgres>,
    scope: &TenantScope<'_>,
    risk_level: Option<&str>,
    page: Page,
) -> Result<Vec<RiskOrderRow>, sqlx::Error> {
    let limit = page.limit.clamp(1, 100);
    let offset = page.offset.max(0);

    sqlx::query_as::<_, RiskOrderRow>(
        r#"
        SELECT order_id, buyer_id, shop_id, risk_level, created_at
        FROM risk_orders
        WHERE tenant_id = $1
          AND shop_id = $2
          AND user_id = $3
          AND ($4::text IS NULL OR risk_level = $4)
        ORDER BY created_at DESC, order_id DESC
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(scope.tenant_id)
    .bind(scope.shop_id)
    .bind(scope.user_id)
    .bind(risk_level)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub const INDEXES: &[&str] = &[
    "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_risk_orders_order_id ON risk_orders(order_id)",
    "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_risk_orders_buyer_id ON risk_orders(buyer_id)",
    "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_risk_orders_shop_id ON risk_orders(shop_id)",
    "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_risk_orders_risk_level ON risk_orders(risk_level)",
    "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_risk_orders_created_at ON risk_orders(created_at)",
];
