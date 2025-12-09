import { NextRequest, NextResponse } from 'next/server';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

// API Route that bridges frontend to gRPC backend using grpcurl
export async function POST(request: NextRequest) {
  try {
    const { method, data } = await request.json();

    if (!method) {
      return NextResponse.json({ error: 'Method required' }, { status: 400 });
    }

    // Convert camelCase to snake_case for proto
    const protoData = JSON.stringify(data);
    
    // Use grpcurl to call the backend
    const grpcUrl = process.env.GRPC_BACKEND_URL || 'localhost:50051';
    
    const command = `grpcurl -plaintext -d '${protoData.replace(/'/g, "\\'")}' ${grpcUrl} mitra.MitraService/${method}`;
    
    console.log('Executing gRPC call:', method);
    
    const { stdout, stderr } = await execAsync(command, {
      timeout: 30000,
    });

    if (stderr && !stdout) {
      console.error('gRPC error:', stderr);
      return NextResponse.json({ error: stderr }, { status: 500 });
    }

    // Parse the JSON response
    try {
      const result = JSON.parse(stdout);
      
      // Convert snake_case response to camelCase
      const camelCaseResult = convertKeysToCamelCase(result);
      
      return NextResponse.json(camelCaseResult);
    } catch {
      // If not JSON, return raw response
      return NextResponse.json({ raw: stdout });
    }
  } catch (error) {
    console.error('API route error:', error);
    
    const errorMessage = error instanceof Error ? error.message : 'Unknown error';
    
    // Check if it's a gRPC-specific error
    if (errorMessage.includes('Code:')) {
      const match = errorMessage.match(/Message: (.+)/);
      return NextResponse.json(
        { error: match ? match[1] : errorMessage },
        { status: 400 }
      );
    }
    
    return NextResponse.json(
      { error: errorMessage },
      { status: 500 }
    );
  }
}

// Convert snake_case keys to camelCase
function convertKeysToCamelCase(obj: unknown): unknown {
  if (Array.isArray(obj)) {
    return obj.map(convertKeysToCamelCase);
  }
  
  if (obj !== null && typeof obj === 'object') {
    return Object.keys(obj as Record<string, unknown>).reduce((result, key) => {
      const camelKey = key.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
      result[camelKey] = convertKeysToCamelCase((obj as Record<string, unknown>)[key]);
      return result;
    }, {} as Record<string, unknown>);
  }
  
  return obj;
}

