pub fn test_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("test http client")
}

#[allow(dead_code)]
pub async fn test_http_get<U: reqwest::IntoUrl>(url: U) -> reqwest::Result<reqwest::Response> {
    test_http_client().get(url).send().await
}
