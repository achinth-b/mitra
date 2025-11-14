# Mitra Solana Programs

Solana on-chain programs for the Mitra prediction market platform. This directory contains the Friend Groups program (Section 2.1) that manages friend group creation, member invitations, and fund management.

## 📋 Commands

### Build & Deploy
```bash
# Build all programs
anchor build

# Build and deploy to devnet
anchor deploy --provider.cluster devnet

# Build and deploy to mainnet-beta
anchor deploy --provider.cluster mainnet-beta

# Clean build artifacts
anchor clean
```

### Testing
```bash
# Run all tests (starts local validator automatically)
npm test
# or
anchor test

# Run tests without starting local validator (use existing validator)
npm run test:local
# or
anchor test --skip-local-validator

# Run tests on specific cluster
anchor test --provider.cluster devnet
```

### Development
```bash
# Install dependencies
npm install

# Lint TypeScript files
npm run lint

# Generate TypeScript types from IDL
anchor build  # Types generated in target/types/
```

### Key Management
```bash
# Generate new keypair
solana-keygen new

# Set keypair for deployment
anchor keys list

# Show program ID
anchor keys show
```

## 📁 Directory Structure

```
solana/
├── Anchor.toml              # Anchor workspace configuration
├── package.json             # Node.js dependencies and scripts
├── package-lock.json        # Locked dependency versions
├── programs/                # Solana program source code
│   └── friend_groups/      # Friend Groups program
│       ├── Cargo.toml       # Rust dependencies
│       └── src/             # Program source files
│           ├── lib.rs       # Program entry point & instruction handlers
│           ├── state.rs     # Account structure definitions
│           ├── errors.rs    # Custom error types
│           └── instructions/ # Individual instruction implementations
│               ├── mod.rs
│               ├── create_group.rs
│               ├── invite_member.rs
│               ├── accept_invite.rs
│               ├── remove_member.rs
│               ├── deposit_funds.rs
│               └── withdraw_funds.rs
└── tests/                   # TypeScript test files
    ├── friend-groups.ts     # Main test suite
    └── helpers.ts           # Test utility functions
```

## 📄 File Descriptions

### Configuration Files

#### `Anchor.toml`
- **Purpose**: Anchor workspace configuration file
- **Contains**: 
  - Program IDs and addresses
  - Build settings
  - Test validator configuration
  - Cluster settings (devnet/testnet/mainnet)
- **Status**: Currently empty - needs configuration

#### `package.json`
- **Purpose**: Node.js project configuration
- **Contains**:
  - Project metadata (name, version, description)
  - NPM scripts (test, build, lint)
  - Dependencies: `@coral-xyz/anchor`, `@solana/web3.js`, `@solana/spl-token`
  - Dev dependencies: TypeScript, Chai, Mocha, type definitions
- **Usage**: Run `npm install` to install dependencies

#### `package-lock.json`
- **Purpose**: Locked versions of npm dependencies
- **Contains**: Exact versions of all installed packages
- **Note**: Auto-generated, do not edit manually

### Program Source Files (`programs/friend_groups/`)

#### `Cargo.toml`
- **Purpose**: Rust/Cargo project configuration
- **Contains**: 
  - Rust dependencies (anchor-lang, anchor-spl)
  - Program metadata
  - Build settings

#### `src/lib.rs`
- **Purpose**: Program entry point and instruction router
- **Contains**:
  - Program ID declaration
  - Module exports (state, instructions, errors)
  - Public instruction handlers:
    - `create_group` - Initialize new friend group
    - `invite_member` - Admin invites a user
    - `accept_invite` - User accepts invitation
    - `remove_member` - Admin removes member
    - `deposit_funds` - Member deposits SOL/USDC
    - `withdraw_funds` - Member withdraws SOL/USDC

#### `src/state.rs`
- **Purpose**: Account structure definitions
- **Contains**:
  - `FriendGroup` - Main group account (admin, name, member_count, treasuries)
  - `GroupMember` - Individual member account (user, group, balances, role)
  - `Invite` - Invitation account (group, invited_user, inviter, expiration)
  - `MemberRole` - Enum (Admin, Member)
  - Constants: `MIN_MEMBERS` (3), `MAX_MEMBERS` (30), `EXPIRY_SECONDS` (7 days)

#### `src/errors.rs`
- **Purpose**: Custom error definitions
- **Contains**: Error enum with variants:
  - `Unauthorized` - Permission denied
  - `NameTooLong` - Group name exceeds 50 chars
  - `MemberAlreadyExists` - Duplicate member
  - `MemberNotFound` - Member doesn't exist
  - `InsufficientBalance` - Not enough funds
  - `InvalidAmount` - Invalid input amount
  - `MaxMembersReached` - Group at capacity (30)
  - `MinMembersRequired` - Can't go below 3 members
  - `InviteInvalid` - Invite doesn't exist
  - `InviteExpired` - Invite past expiration
  - `FundsLocked` - Funds locked due to active bets

#### `src/instructions/mod.rs`
- **Purpose**: Module exports for instructions
- **Contains**: Re-exports of all instruction modules

#### `src/instructions/create_group.rs`
- **Purpose**: Create new friend group instruction
- **Contains**:
  - `CreateGroup` context struct (accounts needed)
  - `handler` function - Creates group, SOL treasury PDA, validates USDC treasury
  - Sets admin as first member (member_count = 1)

#### `src/instructions/invite_member.rs`
- **Purpose**: Admin invites a user to join group
- **Contains**:
  - `InviteMember` context struct
  - `handler` function - Creates invite account with 7-day expiration
  - Validates admin-only, member limit, prevents self-invite

#### `src/instructions/accept_invite.rs`
- **Purpose**: User accepts invitation to join group
- **Contains**:
  - `AcceptInvite` context struct
  - `handler` function - Creates GroupMember account, closes invite, increments member_count
  - Validates invite expiration and signer

#### `src/instructions/remove_member.rs`
- **Purpose**: Admin removes member from group
- **Contains**:
  - `RemoveMember` context struct
  - `handler` function - Refunds balances, closes account, decrements member_count
  - Handles locked funds (active bets) - marks as locked instead of deleting
  - Validates minimum member requirement (3)

#### `src/instructions/deposit_funds.rs`
- **Purpose**: Member deposits SOL/USDC to group treasury
- **Contains**:
  - `DepositFunds` context struct
  - `handler` function - Transfers funds, updates member balances
  - Supports both SOL (lamports) and USDC (SPL tokens)
  - Validates member belongs to group, checks locked funds

#### `src/instructions/withdraw_funds.rs`
- **Purpose**: Member withdraws SOL/USDC from group treasury
- **Contains**:
  - `WithdrawFunds` context struct
  - `handler` function - Transfers funds back, updates balances
  - Validates sufficient balance, checks locked funds
  - Uses PDA signing for USDC transfers

### Test Files (`tests/`)

#### `tests/friend-groups.ts`
- **Purpose**: Main test suite for Friend Groups program
- **Contains**:
  - Test setup (provider, program, keypairs)
  - Test suites for each instruction:
    - `create_group` - Group creation, name validation
    - `invite_member` - Admin invite, authorization checks
    - `accept_invite` - Accept flow, member creation
    - `deposit_funds` - SOL/USDC deposits, validation
    - `withdraw_funds` - Withdrawals, balance checks
    - `remove_member` - Removal flow, refund logic
    - `edge_cases` - Max members, edge conditions
  - Uses Chai for assertions, Mocha for test framework

#### `tests/helpers.ts`
- **Purpose**: Reusable test utility functions
- **Contains**:
  - `airdropSol` - Request SOL airdrop for test accounts
  - `deriveFriendGroupPda` - Derive friend group PDA address
  - `deriveTreasurySolPda` - Derive SOL treasury PDA
  - `deriveMemberPda` - Derive member account PDA
  - `deriveInvitePda` - Derive invite account PDA

## 🔑 Key Concepts

### Account Types
- **FriendGroup**: Main group account storing admin, name, member count, treasury addresses
- **GroupMember**: Per-user account tracking balances and membership status
- **Invite**: Temporary account for pending invitations (auto-closed on accept)

### PDAs (Program Derived Addresses)
- Friend groups: `[b"friend_group", admin]`
- SOL treasury: `[b"treasury_sol", friend_group]`
- Members: `[b"member", friend_group, user]`
- Invites: `[b"invite", friend_group, invited_user]`

### Constraints
- Minimum members: 3
- Maximum members: 30
- Invite expiration: 7 days
- Group name max length: 50 characters

## 🚀 Getting Started

1. **Install dependencies**:
   ```bash
   npm install
   ```

2. **Build the program**:
   ```bash
   anchor build
   ```

3. **Run tests**:
   ```bash
   npm test
   ```

4. **Deploy to devnet**:
   ```bash
   anchor deploy --provider.cluster devnet
   ```

## 📝 Notes

- Program ID: `Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS`
- Uses Anchor framework v0.30+
- Supports both SOL and USDC (SPL tokens)
- All instructions include comprehensive validation and error handling
- Test suite covers happy paths and error cases

## 🔗 Related Documentation

- [Anchor Documentation](https://www.anchor-lang.com/)
- [Solana Cookbook](https://solanacookbook.com/)
- [SPL Token Program](https://spl.solana.com/token)

