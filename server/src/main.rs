use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use hackaton_checker::api::{self, Ctx};
use hackaton_checker::questions;
use hackaton_checker::store::Store;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let questions_path = PathBuf::from(
        std::env::var("QUESTIONS_DIR").unwrap_or_else(|_| "questions".to_string()),
    );
    let state_path =
        PathBuf::from(std::env::var("DB_FILE").unwrap_or_else(|_| "data/hackaton.db".to_string()));
    let bind: SocketAddr = match (std::env::var("BIND"), std::env::var("PORT")) {
        (Ok(bind), _) => bind.parse()?,
        (Err(_), Ok(port)) => format!("0.0.0.0:{port}").parse()?,
        _ => "127.0.0.1:8080".parse()?,
    };

    let questions = questions::load(&questions_path)?;
    for question in &questions {
        println!(
            "  {} · {} · {} caso(s) · {:?}",
            question.id,
            question.title,
            question.cases.len(),
            question.mode
        );
    }
    println!(
        "loaded {} questions from {}",
        questions.len(),
        questions_path.display()
    );

    let store = Store::open(state_path.clone())?;
    println!("database: {}", state_path.display());

    let ctx = Arc::new(Ctx { store, questions });
    let listener = tokio::net::TcpListener::bind(bind).await?;
    println!("checker listening on http://{bind}");

    axum::serve(listener, api::router(ctx))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
