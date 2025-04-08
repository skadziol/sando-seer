#!/bin/bash

echo "Setting up SandoSeer environment..."

# Install Rust if not already installed
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Check Solana CLI
if ! command -v solana &> /dev/null; then
    echo "Installing Solana CLI tools..."
    sh -c "$(curl -sSfL https://release.solana.com/v1.17.0/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
fi

# Clone dependencies
git clone https://github.com/piotrostr/rig

# Build dependencies
echo "Building dependencies..."
cd rig && cargo build && cd ..
cd listen-core && cargo build && cd ..

# Create .env file with default configuration
echo "Creating .env file..."
cat > .env << EOL
# Solana RPC endpoint
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com
# Optional: Use your own paid endpoint for better performance
# SOLANA_RPC_URL=https://your-quicknode-or-triton-endpoint.com

# Wallet config (IMPORTANT: Use a test wallet only!)
# Path to your keypair file
WALLET_PATH=~/.config/solana/id.json

# Rig configuration
RIG_API_KEY=your_openai_api_key

# Optional: Telegram alerts
TELEGRAM_BOT_TOKEN=
TELEGRAM_CHAT_ID=
EOL

echo "Setting up project structure..."
mkdir -p src/{listen_bot,rig_agent,evaluator,monitoring}

echo "Setup complete! Next steps:"
echo "1. Edit the .env file with your API keys and settings"
echo "2. Create a test wallet with 'solana-keygen new'"
echo "3. Run 'cargo build' to compile the project"