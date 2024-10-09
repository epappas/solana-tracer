use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{
    ConfirmedTransactionWithStatusMeta, TransactionWithStatusMeta, UiInstruction,
    UiParsedInstruction, UiTransaction,
};
use std::collections::HashMap;

pub struct GraphBuilder {
    graph: DiGraph<String, f64>,
    node_map: HashMap<String, NodeIndex>,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    pub fn process_transaction(&mut self, transaction: &UiTransaction) -> Result<()> {
        let mut pre_balance = 0;
        let mut post_balance = 0;

        let tx = ConfirmedTransactionWithStatusMeta {
            slot: 0,
            tx_with_meta: TransactionWithStatusMeta::MissingMetadata(transaction.clone().into()),
            block_time: None,
        };

        if let Some(meta) = &tx.tx_with_meta.get_status_meta() {
            pre_balance = meta.pre_balances[0];
            post_balance = meta.post_balances[0];

            for (index, instruction) in meta.parsed.instructions.iter().enumerate() {
                self.process_parsed_instruction(instruction, &pre_balance, &post_balance)?;
            }
        }

        Ok(())
    }

    fn process_parsed_instruction(
        &mut self,
        instruction: &serde_json::Value,
        pre_balance: &u64,
        post_balance: &u64,
    ) -> Result<()> {
        match instruction["type"].as_str() {
            Some("transfer") => self.process_transfer(instruction, pre_balance, post_balance),
            Some("transferChecked") => self.process_spl_transfer(instruction),
            _ => Ok(()), // Ignore other instruction types for now
        }
    }

    fn process_transfer(
        &mut self,
        instruction: &serde_json::Value,
        pre_balance: &u64,
        post_balance: &u64,
    ) -> Result<()> {
        let source = instruction["info"]["source"].as_str().unwrap();
        let destination = instruction["info"]["destination"].as_str().unwrap();
        let amount = instruction["info"]["amount"]
            .as_str()
            .unwrap()
            .parse::<f64>()?;

        let source_node = self.add_node(source);
        let destination_node = self.add_node(destination);

        self.graph.add_edge(source_node, destination_node, amount);

        Ok(())
    }

    fn process_spl_transfer(&mut self, instruction: &serde_json::Value) -> Result<()> {
        let source = instruction["info"]["source"].as_str().unwrap();
        let destination = instruction["info"]["destination"].as_str().unwrap();
        let amount = instruction["info"]["amount"]
            .as_str()
            .unwrap()
            .parse::<f64>()?;

        let source_node = self.add_node(source);
        let destination_node = self.add_node(destination);

        self.graph.add_edge(source_node, destination_node, amount);

        Ok(())
    }

    fn add_node(&mut self, account: &str) -> NodeIndex {
        *self
            .node_map
            .entry(account.to_string())
            .or_insert_with(|| self.graph.add_node(account.to_string()))
    }

    pub fn export_json(&self) -> Result<String> {
        let mut nodes = vec![];
        let mut edges = vec![];

        for node in self.graph.node_indices() {
            nodes.push(json!({
                "id": self.graph[node],
                "label": self.graph[node],
            }));
        }

        for edge in self.graph.edge_indices() {
            let (source, destination) = self.graph.edge_endpoints(edge).unwrap();
            edges.push(json!({
                "source": self.graph[source],
                "target": self.graph[destination],
                "amount": self.graph[edge],
            }));
        }

        let graph = json!({
            "nodes": nodes,
            "edges": edges,
        });

        Ok(serde_json::to_string_pretty(&graph)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::Signature;
    use solana_transaction_status::{UiTransaction, UiTransactionEncoding};

    #[test]
    fn test_add_node() {
        let mut builder = GraphBuilder::new();
        let node1 = builder.add_node("account1");
        let node2 = builder.add_node("account2");
        let node1_again = builder.add_node("account1");

        assert_ne!(node1, node2);
        assert_eq!(node1, node1_again);
    }

    #[test]
    fn test_process_transfer() {
        let mut builder = GraphBuilder::new();
        let instruction = json!({
            "type": "transfer",
            "info": {
                "source": "source_account",
                "destination": "dest_account",
                "amount": "1000"
            }
        });
        builder.process_transfer(&instruction, &1000, &0).unwrap();

        let json = builder.export_json().unwrap();
        let graph: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(graph["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(graph["edges"].as_array().unwrap().len(), 1);
    }
}
