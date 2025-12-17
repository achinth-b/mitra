#!/bin/bash
# Mitra Devnet Setup Script
# Run this to set up a test user with dev USDC

set -e

echo "🚀 Mitra Devnet Setup Script"
echo "================================"

cd "$(dirname "$0")"

# Configuration
FAUCET_KEYPAIR="./keys/faucet-authority.json"
USDC_MINT="7jkMMe865C3U7s3hBHywWM6HjaTcFtfbbyGsFW1SMZX2"
DEFAULT_AMOUNT=1000000000  # 1000 USDC (6 decimals)

# Check if test wallet already exists
if [ ! -f "./keys/test-user.json" ]; then
    echo "📝 Creating test user wallet..."
    solana-keygen new --outfile ./keys/test-user.json --no-bip39-passphrase --force
else
    echo "✓ Test user wallet exists"
fi

TEST_USER=$(solana-keygen pubkey ./keys/test-user.json)
echo "👤 Test User: $TEST_USER"

# Create token account for test user
echo "🏦 Creating USDC token account for test user..."
spl-token create-account $USDC_MINT --owner ./keys/test-user.json --fee-payer ~/.config/solana/id.json 2>/dev/null || echo "Token account may already exist"

# Get the associated token account address
TOKEN_ACCOUNT=$(spl-token accounts $USDC_MINT --owner $TEST_USER --output json 2>/dev/null | jq -r '.accounts[0].address // empty')

if [ -z "$TOKEN_ACCOUNT" ]; then
    echo "❌ Failed to get token account. Creating with default..."
    TOKEN_ACCOUNT=$(spl-token address --token $USDC_MINT --owner $TEST_USER)
fi

echo "💳 Token Account: $TOKEN_ACCOUNT"

# Mint dev USDC to test user
echo "💰 Minting 1000 dev USDC to test user..."
spl-token mint $USDC_MINT $DEFAULT_AMOUNT $TOKEN_ACCOUNT --mint-authority $FAUCET_KEYPAIR

# Check balance
echo ""
echo "✅ Setup Complete!"
echo "================================"
echo "Test User Wallet: $TEST_USER"
echo "Token Account:    $TOKEN_ACCOUNT"
echo ""
echo "USDC Balance:"
spl-token balance $USDC_MINT --owner $TEST_USER
echo ""
echo "To use this wallet for testing, set:"
echo "  user_wallet=$TEST_USER"
echo "  user_usdc_account=$TOKEN_ACCOUNT"
