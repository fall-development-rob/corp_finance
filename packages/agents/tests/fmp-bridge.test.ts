import { describe, it, expect } from 'vitest';
import { FmpBridge } from '../bridge/fmp-bridge.js';

describe('FmpBridge', () => {
  it('starts disconnected', () => {
    const bridge = new FmpBridge();
    expect(bridge.isConnected).toBe(false);
  });

  it('throws when calling tool before connect', async () => {
    const bridge = new FmpBridge();
    await expect(bridge.callTool('fmp_quote', { symbol: 'AAPL' })).rejects.toThrow('FMP bridge not connected');
  });

  it('throws when disconnecting before connect is safe', async () => {
    const bridge = new FmpBridge();
    // disconnect on non-connected bridge should be a no-op
    await expect(bridge.disconnect()).resolves.toBeUndefined();
    expect(bridge.isConnected).toBe(false);
  });
});
