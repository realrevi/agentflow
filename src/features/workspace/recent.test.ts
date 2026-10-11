import { describe, expect, it } from 'vitest';
import { addRecent, folderName } from './recent';

describe('addRecent', () => {
  it('moves the path to the front without duplicates', () => {
    expect(addRecent(['/a', '/b', '/c'], '/b')).toEqual(['/b', '/a', '/c']);
  });

  it('keeps at most 10 entries', () => {
    const many = Array.from({ length: 12 }, (_, i) => `/p${i}`);
    const result = addRecent(many, '/new');
    expect(result).toHaveLength(10);
    expect(result[0]).toBe('/new');
  });
});

describe('folderName', () => {
  it('returns the last segment', () => {
    expect(folderName('/home/me/projects/agentflow')).toBe('agentflow');
    expect(folderName('/home/me/projects/agentflow/')).toBe('agentflow');
    expect(folderName('C:\\Users\\me\\code')).toBe('code');
  });

  it('keeps roots readable', () => {
    expect(folderName('/')).toBe('/');
    expect(folderName('C:\\')).toBe('C:\\');
  });
});
