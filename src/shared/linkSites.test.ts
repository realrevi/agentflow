import { describe, expect, it } from 'vitest';
import { siteForUrl } from './linkSites';

const id = (url: string) => siteForUrl(url)?.id ?? null;

describe('link brand icons', () => {
  it('knows common sites, with and without www', () => {
    expect(id('https://github.com/realrevi/agentflow')).toBe('github');
    expect(id('https://www.youtube.com/watch?v=1')).toBe('youtube');
    expect(id('https://youtu.be/abc')).toBe('youtube');
    expect(id('https://x.com/user')).toBe('x');
    expect(id('https://twitter.com/user')).toBe('x');
    expect(id('https://www.npmjs.com/package/solid-js')).toBe('npm');
    expect(id('https://pypi.org/project/requests/')).toBe('pypi');
    expect(id('https://crates.io/crates/serde')).toBe('rust');
    expect(id('https://stackoverflow.com/q/1')).toBe('stackoverflow');
    expect(id('https://developer.mozilla.org/en-US/docs/Web')).toBe('mdnwebdocs');
    expect(id('https://docs.google.com/document/d/1')).toBe('googledocs');
  });

  it('matches subdomains to their site', () => {
    expect(id('https://gist.github.com/a/b')).toBe('github');
    expect(id('https://raw.githubusercontent.com/a/b/main/x')).toBe('github');
    expect(id('https://my-team.notion.site/page')).toBe('notion');
    expect(id('https://hub.docker.com/_/node')).toBe('docker');
    expect(id('https://en.wikipedia.org/wiki/Rust')).toBe('wikipedia');
  });

  it('returns null for unknown hosts, look-alikes and other schemes', () => {
    expect(id('https://example.com/github.com')).toBeNull();
    expect(id('https://notgithub.com/x')).toBeNull();
    expect(id('https://github.com.evil.dev/x')).toBeNull();
    expect(id('mailto:a@github.com')).toBeNull();
    expect(id('relative/path.md')).toBeNull();
  });
});
