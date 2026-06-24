# evm-scan

`evm-scan` is a Rust CLI tool for inspecting and interacting with deployed EVM smart contracts directly from their on-chain bytecode.

The goal of this project is to understand how much information can be extracted from a contract without having its verified source code, using low-level EVM tooling and Foundry's `cast`.

## Features

- Fetch deployed bytecode from an EVM address
- Extract function selectors from runtime bytecode
- Resolve selectors using Foundry `cast 4byte`
- Display detected functions with mutability and return type when available
- Let the user select a function interactively
- Ask for function arguments from the terminal
- Execute the selected function with:
  - `cast call` for `view` / `pure` functions
  - `cast send` for state-changing functions

## Why this project?

When auditing or inspecting smart contracts, verified source code is not always available.

This tool explores the idea of building a lightweight contract inspection assistant from bytecode-level data:

1. Retrieve bytecode
2. Extract selectors
3. Resolve known signatures
4. Infer possible interaction modes
5. Execute calls through Foundry

It is not meant to replace a full decompiler or professional audit workflow, but it is a useful learning tool for EVM internals, selector analysis, and smart contract reconnaissance.

## Tech Stack

- Rust
- Foundry / Cast
- EVM bytecode
- 4byte selector resolution
- CLI tooling

## Requirements

You need Foundry installed:

```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

Check that `cast` is available:

```bash
cast --version
```

## Installation

```bash
git clone https://github.com/jerem0010/evm-scan.git
cd evm-scan
cargo build --release
```

## Usage

```bash
cargo run -- <CONTRACT_ADDRESS> --rpc-url <RPC_URL>
```

Example:

```bash
cargo run -- 0xYourContractAddress --rpc-url https://eth.llamarpc.com
```

The tool will:

1. Fetch the bytecode
2. Extract selectors
3. Resolve possible function signatures
4. Display the detected functions
5. Ask you which function to execute
6. Ask for arguments if needed
7. Run the corresponding `cast` command

## Example Flow

```txt
Detected functions:

[1] 0x70a08231 | balanceOf(address) | view
[2] 0x18160ddd | totalSupply() | view
[3] 0xa9059cbb | transfer(address,uint256) | nonpayable

Choose function index: 1
arg0 address: 0x...
Running:
cast call 0xContract balanceOf(address) 0xUser --rpc-url <RPC_URL>
```

## Current Limitations

- Selector resolution depends on known 4byte signatures
- Unknown selectors cannot be executed safely
- ABI reconstruction is approximate
- Complex types may require manual care
- Write calls require wallet/private-key handling through Foundry configuration

