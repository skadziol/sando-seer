# SandoSeer: AI MEV Oracle for Solana

![SandoSeer Logo](docs/sandoseer_logo.png)

SandoSeer is an AI-powered MEV (Maximal Extractable Value) oracle for the Solana blockchain, capable of autonomously detecting and exploiting MEV opportunities (sandwiches, arbitrage, token snipes) using a combination of real-time on-chain data and AI-powered decision making.

## 🧠 Core Components

- **Listen Bot**: Monitors Solana DEX transactions and mempool in real-time using `listen-engine`
- **RIG Agent**: LLM-powered forecasting and decision-making via `rig`
- **Opportunity Evaluator**: Analyzes potential MEV opportunities and calculates profitability/risk
- **Autonomous Executor**: Executes trades based on calculated MEV opportunities

## 🚀 Features

- Real-time monitoring of Solana DEX transactions
- AI-powered analysis of trading opportunities
- Automated execution of MEV strategies:
  - Sandwich trading
  - Arbitrage
  - Token sniping
- Risk-adjusted trading with configurable parameters
- Performance monitoring and feedback loop
- Optional Telegram notifications

## 🔧 Setup & Installation

### Prerequisites

- Rust toolchain (1.67 or higher)
- Solana CLI tools
- An RPC endpoint (QuickNode, Triton, or public endpoint)
- OpenAI API key for the RIG agent (optional)

### Installation

1. Clone this repository:
   ```bash
   git clone https://github.com/yourusername/sandoseer.git
   cd sandoseer