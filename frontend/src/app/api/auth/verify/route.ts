import { NextRequest, NextResponse } from 'next/server';

/**
 * Server-side Magic DID Token verification
 * 
 * Uses the SECRET KEY (sk_live_xxx or sk_test_xxx)
 * This key should ONLY be used server-side, NEVER exposed to the client
 */
export async function POST(request: NextRequest) {
  try {
    const { didToken } = await request.json();

    if (!didToken) {
      return NextResponse.json(
        { error: 'DID token required' },
        { status: 400 }
      );
    }

    // SECRET KEY - NEVER expose this to the frontend!
    const secretKey = process.env.MAGIC_SECRET_KEY;

    if (!secretKey || secretKey.includes('YOUR_KEY_HERE')) {
      console.warn('Magic secret key not configured - skipping verification');
      // In development, return mock validation
      return NextResponse.json({
        valid: true,
        issuer: 'mock_issuer',
        email: 'dev@example.com',
        publicAddress: 'DevWallet123',
      });
    }

    // Validate key format
    if (!secretKey.startsWith('sk_')) {
      console.error('Invalid Magic secret key format. Secret keys start with "sk_"');
      return NextResponse.json(
        { error: 'Server configuration error' },
        { status: 500 }
      );
    }

    // Use Magic Admin SDK to verify the DID token
    // Note: You need to install @magic-sdk/admin for production
    // npm install @magic-sdk/admin
    
    try {
      // Dynamic import to avoid bundling issues
      const { Magic } = await import('@magic-sdk/admin');
      const magic = new Magic(secretKey);
      
      // Validate the DID token
      magic.token.validate(didToken);
      
      // Get user metadata
      const metadata = await magic.users.getMetadataByToken(didToken);
      
      return NextResponse.json({
        valid: true,
        issuer: metadata.issuer,
        email: metadata.email,
        publicAddress: metadata.publicAddress,
      });
    } catch (validationError) {
      console.error('Token validation failed:', validationError);
      return NextResponse.json(
        { error: 'Invalid token', valid: false },
        { status: 401 }
      );
    }
  } catch (error) {
    console.error('Auth verification error:', error);
    return NextResponse.json(
      { error: 'Verification failed' },
      { status: 500 }
    );
  }
}

