mod backend;
mod balancer;
mod config;
mod logging;
mod pool;
mod proxy;

use backend::Backend;
use balancer::{LeastConnections, RoundRobin};
use config::Strategy;
use pool::Pool;
use proxy::{ProxyService, ProxyState};

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_owned());

    let config = config::Config::from_file(&config_path)?;

    println!("rust-proxy starting");
    println!("  listen:   {}", config.listen);
    println!("  strategy: {:?}", config.strategy);
    for b in &config.backends {
        println!("  backend:  {}", b.url);
    }

    let backends: Vec<_> = config
        .backends
        .iter()
        .map(|b| Backend::new(&b.url))
        .collect::<anyhow::Result<_>>()?;

    let uris: Vec<_> = backends.iter().map(|b| b.uri.clone()).collect();
    let pool = Pool::new(&uris);

    let balancer: Box<dyn balancer::LoadBalancer> = match config.strategy {
        Strategy::RoundRobin => Box::new(RoundRobin::new()),
        Strategy::LeastConnections => Box::new(LeastConnections),
    };

    let state = Arc::new(ProxyState {
        backends,
        balancer,
        pool,
    });

    let listener = TcpListener::bind(config.listen).await?;
    println!("Listening on http://{}", config.listen);

    loop {
        let (stream, peer) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let service = ProxyService {
            state: state.clone(),
        };

        tokio::spawn(async move {
            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("[{peer}] connection error: {e}");
            }
        });
    }
}
