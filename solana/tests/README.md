# Solana Program Testing Guide

## Overview

Solana programs use Anchor framework with comprehensive test suites including unit tests, integration tests, and E2E flow tests.

## Test Structure

```
solana/tests/
├── events.ts              # Event program tests
├── friend-groups.ts      # Friend group program tests
├── treasury.ts           # Treasury program tests
├── integration.spec.ts   # E2E integration tests
├── harness.ts           # Test harness utilities
└── helpers.ts            # Test helper functions
```

## Running Tests

### All Tests
```bash
cd solana
anchor test
```

### Specific Test File
```bash
anchor test --skip-local-validator
npm test -- tests/integration.spec.ts
```

### With Verbose Output
```bash
anchor test -- --verbose
```

## Test Categories

### Unit Tests
- Individual instruction tests
- Error case testing
- Edge case validation

### Integration Tests (`integration.spec.ts`)
- Complete E2E flows
- Multi-instruction sequences
- Cross-program interactions

### E2E Flow Tests
- Create group → Create event → Place bets → Settle
- Member management flows
- Treasury operations
- Error scenarios

## Writing Tests

### Example Integration Test
```typescript
describe("E2E Flow: Create Group → Create Event → Place Bets → Settle", () => {
  it("Step 1: Create friend group", async () => {
    const tx = await program.methods
      .createFriendGroup("Test Group", new BN(bump))
      .accounts({
        group: groupPubkey,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();

    const groupAccount = await program.account.friendGroup.fetch(groupPubkey);
    expect(groupAccount.name).to.equal("Test Group");
  });
});
```

## Test Harness

Use `FriendGroupTestHarness` for common test setup:
```typescript
const harness = new FriendGroupTestHarness(program, provider);
await harness.init("Test Group");
```

## Best Practices

1. **Isolation**: Each test should be independent
2. **Cleanup**: Clean up test accounts after tests
3. **Fixtures**: Use test harness for reusable setup
4. **Assertions**: Use descriptive Chai assertions
5. **Error Testing**: Test both success and error cases

## Bankrun Integration

For advanced integration testing, use Bankrun:
```typescript
import { startAnchor } from "@coral-xyz/anchor";

const { bankrun, provider } = await startAnchor("./Anchor.toml", []);
```

## Continuous Integration

Tests run in CI/CD:
```yaml
- name: Run Anchor tests
  run: |
    cd solana
    anchor test
```

