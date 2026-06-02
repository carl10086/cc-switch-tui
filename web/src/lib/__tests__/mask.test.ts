import { describe, expect, it } from 'vitest';
import { isSensitiveKey, maskValue } from '../mask';

describe('isSensitiveKey', () => {
  it.each([
    ['ANTHROPIC_AUTH_TOKEN', true],
    ['KIMI_API_KEY', true],
    ['MINIMAX_API_KEY', true],
    ['AUTH_TOKEN', true],
    ['CLIENT_SECRET', true],
    ['DB_PASSWORD', true],
    ['GCP_CREDENTIALS', true],
    // 误报（这些值其实是公开 endpoint）：
    ['ANTHROPIC_BASE_URL', false],
    ['ANTHROPIC_MODEL', false],
    ['DISABLE_COMPACT', false],
    ['CLAUDE_CODE_MAX_CONTEXT_TOKENS', false],
    ['CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV', false],
    ['CC_SWITCH_ALIAS', false],
  ])('isSensitiveKey(%j) === %j', (key, expected) => {
    expect(isSensitiveKey(key)).toBe(expected);
  });
});

describe('maskValue', () => {
  it('masks short values to ***', () => {
    expect(maskValue('sk')).toBe('***');
    expect(maskValue('12345678')).toBe('***');
  });

  it('masks with first 3 + *** + last 4 for values longer than 8', () => {
    expect(maskValue('sk-test-1234-abcdef')).toBe('sk-***cdef');
    expect(maskValue('KIMI_API_KEY_VALUE_1234567890')).toBe('KIM***7890');
  });

  it('preserves the structure of sk- prefixed keys', () => {
    const masked = maskValue('sk-ant-1234567890abcdef');
    expect(masked).toMatch(/^sk-\*\*\*[a-z0-9]{0,4}$/);
  });
});
