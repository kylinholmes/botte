use anyhow::Ok;

use std::net::SocketAddr;
use axum::Router;

use crate::config::CONFIG;


pub async fn startup(router: Router) -> anyhow::Result<()> {
    let addr = CONFIG.listen.clone();
    println!("running on: http://{}", addr);
    let listner = tokio::net::TcpListener::bind(addr.clone()).await?;
    axum::serve(
        listner,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    ).await.unwrap();

    Ok(())
}