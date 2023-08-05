# Arcmind Autonomous AI Agent

ArcMind is a Long-Running Agent equipped with a primary main loop that
can orchestrate various tools and memory stores to accomplish numerous sub-tasks that
make up a larger task. Utilizing the power of LLMs such as GPT-3/4 or other open-source
alternatives, these agents can retain both short-term and long-term memory for optimized
task execution. For ArcMind, we leverage Canister as a long-term memory vector store for
semantic search, enabling efficient and accurate task execution.

## Prerequisites

- Install Rust Toolchain using rustup  
  Follows https://www.rust-lang.org/tools/install
- Install cargo-audit

```
cargo install cargo-audit
```

- Install dfx sdk  
  Follow https://github.com/dfinity/sdk

## Quick Start

If you want to test your project locally, you can use the following commands:

```bash
# Starts the replica, running in the background
dfx start --background

# Deploys controller and brain canisters to the local replica
./scripts/provision.sh
```

The provision script will deploy a `controller` canister and a `brain` canister which is owned solely by the `controller`

## Diagrams

See [Architecture](diagram/architecture.png)  
See [Chain Of Thoughts](diagram/chainofthoughts.png)

## Canisters

ArcMind is composed of 2 canisters in a parent-child relationship:

1. [Main loop controller](src/arcmindai_controller/)
1. [Brain connecting to LLM](src/arcmindai_brain/)

The `brain` canister could either connect to LLM remotely or locally hosted open-source LLM like [LLama2](https://github.com/facebookresearch/llama) in the future.

# Author

Henry Chan henry@controlaltdevelop.com  
Kairic Cheng kairic.cheng@gmail.com
