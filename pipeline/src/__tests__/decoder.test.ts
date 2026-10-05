import { describe, it, expect } from 'vitest';
import { xdr, Address } from '@stellar/stellar-sdk';
import { decodeDataKey, decodeStorageValue, decodeEventTopic, decodeEventData, tryDecodeStorageEntry } from '../decoder';
import { DataKeyVariant } from '../types';

function makeSingletonKey(name: string): xdr.ScVal {
  return xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(name)]);
}

function makeAddressScVal(addr: string): xdr.ScVal {
  return Address.fromString(addr).toScVal();
}

function makeU64ScVal(val: number): xdr.ScVal {
  return xdr.ScVal.scvU64(new xdr.Uint64(val));
}

describe('decodeDataKey', () => {
  it('decodes Admin singleton key', () => {
    const result = decodeDataKey(makeSingletonKey('Admin'));
    expect(result.variant).toBe(DataKeyVariant.Admin);
  });

  it('decodes AdminPubKey singleton key', () => {
    const result = decodeDataKey(makeSingletonKey('AdminPubKey'));
    expect(result.variant).toBe(DataKeyVariant.AdminPubKey);
  });

  it('decodes Wrap key with address and period', () => {
    const userAddr = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
    const key = xdr.ScVal.scvVec([
      xdr.ScVal.scvSymbol('Wrap'),
      makeAddressScVal(userAddr),
      makeU64ScVal(202501),
    ]);
    const result = decodeDataKey(key);
    expect(result.variant).toBe(DataKeyVariant.Wrap);
    if (result.variant === DataKeyVariant.Wrap) {
      expect(result.user).toBe(userAddr);
      expect(result.period).toBe(202501);
    }
  });

  it('decodes WrapCount key with address', () => {
    const userAddr = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
    const key = xdr.ScVal.scvVec([
      xdr.ScVal.scvSymbol('WrapCount'),
      makeAddressScVal(userAddr),
    ]);
    const result = decodeDataKey(key);
    expect(result.variant).toBe(DataKeyVariant.WrapCount);
    if (result.variant === DataKeyVariant.WrapCount) {
      expect(result.user).toBe(userAddr);
    }
  });

  it('decodes SlashCount key with address', () => {
    const userAddr = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
    const key = xdr.ScVal.scvVec([
      xdr.ScVal.scvSymbol('SlashCount'),
      makeAddressScVal(userAddr),
    ]);
    const result = decodeDataKey(key);
    expect(result.variant).toBe(DataKeyVariant.SlashCount);
    if (result.variant === DataKeyVariant.SlashCount) {
      expect(result.user).toBe(userAddr);
    }
  });

  it('decodes Slashed key with address', () => {
    const userAddr = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
    const key = xdr.ScVal.scvVec([
      xdr.ScVal.scvSymbol('Slashed'),
      makeAddressScVal(userAddr),
    ]);
    const result = decodeDataKey(key);
    expect(result.variant).toBe(DataKeyVariant.Slashed);
    if (result.variant === DataKeyVariant.Slashed) {
      expect(result.user).toBe(userAddr);
    }
  });

  it('decodes SlashThreshold singleton key', () => {
    const result = decodeDataKey(makeSingletonKey('SlashThreshold'));
    expect(result.variant).toBe(DataKeyVariant.SlashThreshold);
  });

  it('decodes TotalWrapCount key', () => {
    const result = decodeDataKey(makeSingletonKey('TotalWrapCount'));
    expect(result.variant).toBe(DataKeyVariant.TotalWrapCount);
  });

  it('decodes FeeParams key', () => {
    const result = decodeDataKey(makeSingletonKey('FeeParams'));
    expect(result.variant).toBe(DataKeyVariant.FeeParams);
  });

  it('throws on unknown variant', () => {
    expect(() => {
      decodeDataKey(makeSingletonKey('NonExistent'));
    }).toThrow();
  });

  it('throws on empty vec', () => {
    expect(() => {
      decodeDataKey(xdr.ScVal.scvVec([]));
    }).toThrow();
  });
});

describe('decodeStorageValue', () => {
  it('decodes Admin address value', () => {
    const addr = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
    const key = { variant: DataKeyVariant.Admin as const };
    const val = makeAddressScVal(addr);
    const result = decodeStorageValue(key, val);
    expect(result.type).toBe('address');
    if (result.type === 'address') {
      expect(result.value).toBe(addr);
    }
  });

  it('decodes bool value', () => {
    const key = { variant: DataKeyVariant.Paused as const };
    const val = xdr.ScVal.scvBool(true);
    const result = decodeStorageValue(key, val);
    expect(result.type).toBe('bool');
    if (result.type === 'bool') {
      expect(result.value).toBe(true);
    }
  });

  it('decodes u32 value', () => {
    const key = { variant: DataKeyVariant.SlashThreshold as const };
    const val = xdr.ScVal.scvU32(5);
    const result = decodeStorageValue(key, val);
    expect(result.type).toBe('u32');
    if (result.type === 'u32') {
      expect(result.value).toBe(5);
    }
  });

  it('decodes u64 value', () => {
    const key = { variant: DataKeyVariant.StorageBytes as const };
    const val = makeU64ScVal(999);
    const result = decodeStorageValue(key, val);
    expect(result.type).toBe('u64');
    if (result.type === 'u64') {
      expect(result.value).toBe(999);
    }
  });

  it('decodes bytes32 value', () => {
    const key = { variant: DataKeyVariant.AdminPubKey as const };
    const buf = Buffer.alloc(32, 0xab);
    const val = xdr.ScVal.scvBytes(buf);
    const result = decodeStorageValue(key, val);
    expect(result.type).toBe('bytes32');
    if (result.type === 'bytes32') {
      expect(result.value).toBe('ab'.repeat(32));
    }
  });
});

describe('decodeEventTopic', () => {
  it('decodes symbol topic', () => {
    const val = xdr.ScVal.scvSymbol('mint');
    const result = decodeEventTopic(val);
    expect(result.type).toBe('symbol');
    expect(result.value).toBe('mint');
  });

  it('decodes address topic', () => {
    const addr = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
    const val = makeAddressScVal(addr);
    const result = decodeEventTopic(val);
    expect(result.type).toBe('address');
    expect(result.value).toBe(addr);
  });

  it('decodes u64 topic', () => {
    const val = makeU64ScVal(202501);
    const result = decodeEventTopic(val);
    expect(result.type).toBe('u64');
    expect(result.value).toBe(202501);
  });

  it('decodes bool topic', () => {
    const val = xdr.ScVal.scvBool(true);
    const result = decodeEventTopic(val);
    expect(result.type).toBe('bool');
    expect(result.value).toBe(true);
  });
});

describe('decodeEventData', () => {
  it('decodes symbol data', () => {
    const val = xdr.ScVal.scvSymbol('arch');
    const result = decodeEventData(val);
    expect(result.type).toBe('string');
    expect(result.value).toBe('arch');
  });

  it('decodes address data', () => {
    const addr = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
    const val = makeAddressScVal(addr);
    const result = decodeEventData(val);
    expect(result.type).toBe('address');
    expect(result.value).toBe(addr);
  });

  it('decodes bytes data', () => {
    const buf = Buffer.alloc(32, 0x01);
    const val = xdr.ScVal.scvBytes(buf);
    const result = decodeEventData(val);
    expect(result.type).toBe('bytes');
    expect(result.value).toBe('01'.repeat(32));
  });
});

// ─── Failure-mode tests ─────────────────────────────────────────────────

describe('tryDecodeStorageEntry', () => {
  it('returns error with reason for an unknown DataKey variant', () => {
    // A vec key whose first element is a symbol not in DataKeyVariant
    const keyScVal = xdr.ScVal.scvVec([xdr.ScVal.scvSymbol('UnknownVariant')]);
    const valScVal = xdr.ScVal.scvBool(true);
    const result = tryDecodeStorageEntry(keyScVal, valScVal, 100, 'persistent');
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.reason).toMatch(/Unknown DataKey variant/);
      expect(result.error.ledger).toBe(100);
      expect(result.error.keyBase64).toBeTruthy();
    }
  });

  it('returns error with reason when value type does not match key (bool for Admin address key)', () => {
    // Admin expects an address value; give it a bool instead
    const keyScVal = xdr.ScVal.scvVec([xdr.ScVal.scvSymbol('Admin')]);
    const valScVal = xdr.ScVal.scvBool(true);
    const result = tryDecodeStorageEntry(keyScVal, valScVal, 101, 'persistent');
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.reason).toBeTruthy();
      expect(result.error.ledger).toBe(101);
    }
  });

  it('returns error with reason for a Wrap record with missing required fields', () => {
    // A Wrap value that is an empty map — all required fields absent
    const userAddr = 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF';
    const keyScVal = xdr.ScVal.scvVec([
      xdr.ScVal.scvSymbol('Wrap'),
      Address.fromString(userAddr).toScVal(),
      xdr.ScVal.scvU64(new xdr.Uint64(1)),
    ]);
    const valScVal = xdr.ScVal.scvMap([]); // empty map — missing timestamp, data_hash, etc.
    const result = tryDecodeStorageEntry(keyScVal, valScVal, 102, 'persistent');
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.reason).toMatch(/WrapRecord missing/);
      expect(result.error.ledger).toBe(102);
    }
  });

  it('returns ok for a well-formed Paused entry', () => {
    const keyScVal = xdr.ScVal.scvVec([xdr.ScVal.scvSymbol('Paused')]);
    const valScVal = xdr.ScVal.scvBool(false);
    const result = tryDecodeStorageEntry(keyScVal, valScVal, 103, 'instance');
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.entry.key.variant).toBe(DataKeyVariant.Paused);
    }
  });
});
