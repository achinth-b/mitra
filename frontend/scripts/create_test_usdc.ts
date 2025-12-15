import { clusterApiUrl, Connection, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from '@solana/spl-token';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

// Path to backend keypair (standard location)
// If running from frontend/scripts, need to resolve home dir
const HOME_DIR = os.homedir();
const KEYPAIR_PATH = path.join(HOME_DIR, '.config', 'solana', 'id.json');

async function main() {
    console.log("Connecting to Devnet...");
    const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');

    // Load Keypair
    if (!fs.existsSync(KEYPAIR_PATH)) {
        console.error(`Keypair not found at ${KEYPAIR_PATH}`);
        process.exit(1);
    }
    const keypairData = JSON.parse(fs.readFileSync(KEYPAIR_PATH, 'utf-8'));
    const payer = Keypair.fromSecretKey(new Uint8Array(keypairData));
    console.log(`Using Payer: ${payer.publicKey.toBase58()}`);

    // Check Balance
    const balance = await connection.getBalance(payer.publicKey);
    console.log(`Balance: ${balance / LAMPORTS_PER_SOL} SOL`);
    if (balance < 0.01 * LAMPORTS_PER_SOL) {
        console.log("Requesting Airdrop...");
        const sig = await connection.requestAirdrop(payer.publicKey, 1 * LAMPORTS_PER_SOL);
        await connection.confirmTransaction(sig);
        console.log("Airdrop confirmed");
    }

    // Create Mint (6 decimals for USDC)
    console.log("Creating Test USDC Mint...");
    const mint = await createMint(
        connection,
        payer,
        payer.publicKey, // Mint Authority
        null, // Freeze Authority
        6 
    );
    console.log(`\n✅ Test USDC Mint Created: ${mint.toBase58()}\n`);

    // Create ATA for Payer (Treasury/Faucet source)
    const ata = await getOrCreateAssociatedTokenAccount(
        connection,
        payer,
        mint,
        payer.publicKey
    );

    // Mint 1,000,000 USDC (1M * 10^6)
    console.log("Minting 1,000,000 USDC to payer ATA...");
    await mintTo(
        connection,
        payer,
        mint,
        ata.address,
        payer.publicKey,
        1_000_000 * 1_000_000
    );
    console.log("Minting complete!");

    // Save Mint Address to a file for reference
    const configPath = path.join(__dirname, '..', '..', 'devnet_config.json');
    fs.writeFileSync(configPath, JSON.stringify({
        usdc_mint: mint.toBase58(),
        faucet_authority: payer.publicKey.toBase58()
    }, null, 2));
    console.log(`Config saved to ${configPath}`);
}

main().catch(console.error);
