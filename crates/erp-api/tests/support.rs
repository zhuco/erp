#[allow(dead_code)]
pub fn test_database_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("TEST_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string())
}
