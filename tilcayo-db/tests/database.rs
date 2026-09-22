use tilcayo_db::Database;

#[tokio::test]
async fn database_connects() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");

    Database::connect(&url)
        .await
        .expect("failed to connect to database");
}
