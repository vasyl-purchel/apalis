use anyhow::Result;
use apalis::prelude::*;
use apalis::{layers::TraceLayer, mysql::MysqlStorage};
use email_service::{send_email, Email};

async fn produce_jobs(storage: &MysqlStorage<Email>) -> Result<()> {
    let mut storage = storage.clone();
    for i in 0..100 {
        storage
            .push(Email {
                to: format!("test{i}@example.com"),
                text: "Test background job from apalis".to_string(),
                subject: "Background email job".to_string(),
            })
            .await?;
    }
    Ok(())
}

#[async_std::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "debug,sqlx::query=error");
    tracing_subscriber::fmt::init();
    let database_url = std::env::var("DATABASE_URL").expect("Must specify path to db");

    let mysql: MysqlStorage<Email> = MysqlStorage::connect(database_url).await?;
    mysql
        .setup()
        .await
        .expect("unable to run migrations for mysql");

    produce_jobs(&mysql).await?;
    Monitor::new()
        .executor(AsyncStdExecutor::new())
        .register_with_count(2, move |c| {
            WorkerBuilder::new(format!("tasty-avocado-{c}"))
                .layer(TraceLayer::new())
                .with_storage(mysql.clone())
                .build_fn(send_email)
        })
        .run()
        .await?;
    Ok(())
}
