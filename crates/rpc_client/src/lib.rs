use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};

pub struct SolanaRpcClient {
    client: RpcClient,
    rate_limiter: Arc<Semaphore>,
}

impl SolanaRpcClient {
    pub async fn new() -> Result<Self> {
        let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let rate_limiter = Arc::new(Semaphore::new(10)); // Allow 10 concurrent requests
        Ok(Self {
            client,
            rate_limiter,
        })
    }

    pub async fn get_transaction(
        &self,
        signature: &Signature,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
        self.with_retry(|| async {
            let _permit = self
                .rate_limiter
                .acquire()
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            self.client
                .get_transaction(signature, UiTransactionEncoding::JsonParsed)
                .map_err(|e| anyhow::anyhow!(e))
        })
        .await
    }

    pub async fn get_signatures_for_address(&self, address: &str) -> Result<Vec<Signature>> {
        self.with_retry(|| async {
            self.client
                .get_signatures_for_address(&Pubkey::from_str(address)?)
                .map(|result| {
                    result
                        .into_iter()
                        .map(|status| {
                            status
                                .signature
                                .parse::<Signature>()
                                .map_err(|e| anyhow::anyhow!(e))
                        })
                        .collect::<Result<Vec<Signature>, _>>()
                })
                .map_err(|e| anyhow::anyhow!(e))
        })
        .await?
    }

    async fn with_retry<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut backoff = Duration::from_millis(100);
        for _ in 0..5 {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    sleep(backoff).await;
                    backoff *= 2;
                    if backoff > Duration::from_secs(5) {
                        backoff = Duration::from_secs(5);
                    }
                }
            }
        }
        Err(anyhow::anyhow!("Max retries reached"))
    }
}
