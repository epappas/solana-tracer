use crate::graph_builder::GraphBuilder;
use crate::rpc_client::SolanaRpcClient;
use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use solana_sdk::signature::Signature;
use std::collections::HashSet;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub struct SolanaTracer {
    rpc_client: SolanaRpcClient,
    graph_builder: Mutex<GraphBuilder>,
    max_depth: usize,
    max_concurrency: usize,
}

impl SolanaTracer {
    pub async fn new(max_depth: usize, max_concurrency: usize) -> Result<Self> {
        if max_depth == 0 || max_depth > 10 {
            return Err(anyhow::anyhow!(
                "Invalid max_depth. Must be between 1 and 10."
            ));
        }
        if max_concurrency == 0 || max_concurrency > 20 {
            return Err(anyhow::anyhow!(
                "Invalid max_concurrency. Must be between 1 and 20."
            ));
        }

        let rpc_client = SolanaRpcClient::new().await?;
        let graph_builder = Mutex::new(GraphBuilder::new());

        Ok(Self {
            rpc_client,
            graph_builder,
            max_depth,
            max_concurrency,
        })
    }

    pub async fn trace_transaction(&self, signature: &str) -> Result<String> {
        let signature =
            Signature::from_str(signature).context("Invalid transaction signature format")?;

        info!("Starting transaction trace for signature: {}", signature);
        let visited = Mutex::new(HashSet::new());
        self.recursive_trace(&signature, 0, &visited).await?;
        info!("Transaction trace completed");

        self.graph_builder.lock().await.export_json()
    }

    async fn recursive_trace(
        &self,
        signature: &Signature,
        depth: usize,
        visited: &Mutex<HashSet<Signature>>,
    ) -> Result<()> {
        if depth >= self.max_depth {
            warn!("Max depth reached for signature: {}", signature);
            return Ok(());
        }

        {
            let mut visited = visited.lock().await;
            if visited.contains(signature) {
                info!("Already visited signature: {}", signature);
                return Ok(());
            }
            visited.insert(*signature);
        }

        let transaction = self.rpc_client.get_transaction(signature).await?;
        self.graph_builder
            .lock()
            .await
            .process_transaction(&transaction)?;

        if let Some(meta) = &transaction.transaction.meta {
            let futures = meta.pre_balances.iter().map(|pre_balance| {
                let rpc_client = &self.rpc_client;
                let visited = &visited;
                async move {
                    if let Some(pre_tx) = rpc_client
                        .get_signatures_for_address(&pre_balance.to_string(), Some(1))
                        .await?
                    {
                        stream::iter(pre_tx)
                            .map(|sig| self.recursive_trace(&sig, depth + 1, visited))
                            .buffer_unordered(self.max_concurrency)
                            .collect::<Vec<_>>()
                            .await;
                    }
                    Ok(())
                }
            });

            futures::future::join_all(futures).await;
        }

        Ok(())
    }
}
