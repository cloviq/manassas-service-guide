use sqlx::postgres::PgPoolOptions;
use std::{env, time::Duration};
fn bind_host(value: Option<&str>) -> Result<std::net::IpAddr, &'static str> {
    value
        .unwrap_or("127.0.0.1")
        .parse()
        .map_err(|_| "Invalid BIND_HOST.")
}

#[tokio::main]
async fn main() {
    if let Err(message) = run().await {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
async fn run() -> Result<(), &'static str> {
    let command = env::args().nth(1).unwrap_or_else(|| "serve".into());
    if command == "--help" {
        println!(
            "service-guide [serve | migrate | index [--if-stale] | import PATH | validate-import PATH | create-user NAME user|admin]\nRequires DATABASE_URL; index also requires OPENSEARCH_URL and EMBEDDING_URL.\ncreate-user reads the password from the first line of stdin."
        );
        return Ok(());
    }
    if command == "validate-import" {
        let path = env::args()
            .nth(2)
            .ok_or("Usage: service-guide validate-import PATH")?;
        let docs = service_guide::catalog::validate_catalog_file(&path).await?;
        println!(
            "Validated {} catalog records; no database connection or writes.",
            docs.len()
        );
        return Ok(());
    }
    if !["serve", "migrate", "index", "import", "create-user"].contains(&command.as_str()) {
        return Err("Unknown command; use --help.");
    }
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL must be set explicitly; no credential defaults.")?;
    let search_url = env::var("OPENSEARCH_URL").ok();
    if command == "index" && search_url.is_none() {
        return Err("OPENSEARCH_URL must be set for indexing.");
    }
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .map_err(|_| "Database connection failed; check local configuration.")?;
    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(|_| "Database migration failed.")?;
    if command == "migrate" {
        println!("Migrations applied.");
        return Ok(());
    }
    if command == "create-user" {
        const USAGE: &str = "Usage: service-guide create-user NAME user|admin < password";
        let username = env::args().nth(2).ok_or(USAGE)?;
        let role = env::args().nth(3).ok_or(USAGE)?;
        let mut password = String::new();
        std::io::stdin()
            .read_line(&mut password)
            .map_err(|_| "Could not read password from stdin.")?;
        let password = password.trim_end_matches(['\r', '\n']).to_owned();
        let input = service_guide::auth::NewUser {
            username,
            password,
            role,
        };
        let account = service_guide::auth::create_user(&pool, input)
            .await
            .map_err(|error| error.1)?;
        println!("Created {} account '{}'.", account.role, account.username);
        return Ok(());
    }
    let index = env::var("OPENSEARCH_INDEX").unwrap_or_else(|_| "manassas-catalog".into());
    if command == "import" {
        let path = env::args()
            .nth(2)
            .ok_or("Usage: service-guide import catalog/records.json")?;
        let n = tokio::time::timeout(
            Duration::from_secs(300),
            service_guide::catalog::import_catalog(&pool, &path),
        )
        .await
        .map_err(|_| "Catalog import deadline exceeded")??;
        println!("Imported and read back {n} catalog IDs; seed tasks and tickets retained.");
        return Ok(());
    }
    if command == "index" {
        let config = service_guide::search::EmbeddingConfig::from_env()?;
        let url = search_url.as_deref().ok_or("OPENSEARCH_URL required")?;
        if env::args().nth(2).as_deref() == Some("--if-stale")
            && service_guide::search::index_is_current(&pool, url, &index, &config).await?
        {
            println!("Search index is current (schema, model and record count match); skipped.");
            return Ok(());
        }
        let n=tokio::time::timeout(Duration::from_secs(7200),service_guide::search::index_catalog(&pool,search_url.as_deref().ok_or("OPENSEARCH_URL required")?,&index,&config)).await.map_err(|_|"Index build deadline exceeded; previous alias retained unless switch already completed")??;
        println!(
            "Embedded and indexed {n} records; physical count and alias readback verified. Cosine + BM25 RRF enabled."
        );
        return Ok(());
    }
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "3001".into())
        .parse()
        .map_err(|_| "Invalid PORT.")?;
    let host = bind_host(env::var("BIND_HOST").ok().as_deref())?;
    let listener = tokio::net::TcpListener::bind((host, port))
        .await
        .map_err(|_| "Could not bind HTTP listener.")?;
    println!("Listening on http://{host}:{port}; demo tickets are NOT city submissions.");
    axum::serve(
        listener,
        service_guide::auth::with_auth(
            pool.clone(),
            service_guide::api::router_with_search(pool, search_url, index),
        ),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
    .map_err(|_| "HTTP server failed.")
}

#[cfg(test)]
mod tests {
    #[test]
    fn bind_host_defaults_private_and_accepts_container_address() {
        assert_eq!(super::bind_host(None).unwrap().to_string(), "127.0.0.1");
        assert_eq!(
            super::bind_host(Some("0.0.0.0")).unwrap().to_string(),
            "0.0.0.0"
        );
        assert!(super::bind_host(Some("bad-address")).is_err());
    }
}
