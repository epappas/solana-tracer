# Solana Transaction Tracer

This Rust application traces transactions on the Solana blockchain, creating a graph of fund flows starting from a given transaction signature.

## Features

- Traces Solana transactions recursively up to a specified depth
- Handles both system transfers and SPL token transfers
- Implements rate limiting and backoff strategies for RPC calls
- Uses concurrent processing for improved performance
- Caches transaction and signature data for efficiency
- Provides comprehensive error handling and logging

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.