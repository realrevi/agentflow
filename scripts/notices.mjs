#!/usr/bin/env node
// `pnpm notices`: writes THIRD_PARTY_NOTICES.md (shown in Settings > About) from
//  - the Rust crates the app links (cargo metadata, normal dependencies, every platform),
//  - the npm packages bundled into the frontend (pnpm licenses --prod),
//  - the bundled fonts and icon sets (their license files in the repo).
// License texts come from each package's own files; identical texts are printed once.
// Run it after changing dependencies or assets; CONTRIBUTING.md lists it in the release steps.
import { execSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

const OUT = 'THIRD_PARTY_NOTICES.md';
const LICENSE_FILE = /^(licen[cs]e|copying|notice|unlicense)([-._].*)?$/i;

const ASSETS = [
  { name: 'Inter (font)', license: 'OFL-1.1', files: ['src/assets/fonts/Inter-OFL.txt'] },
  {
    name: 'JetBrains Mono (font)',
    license: 'OFL-1.1',
    files: ['src/assets/fonts/JetBrainsMono-OFL.txt'],
  },
  {
    name: 'Pixelify Sans (font)',
    license: 'OFL-1.1',
    files: ['src/assets/fonts/PixelifySans-OFL.txt'],
  },
  { name: 'Lucide (icons)', license: 'ISC', files: ['src/assets/icons/LICENSE-lucide.txt'] },
  {
    name: 'Material Icon Theme (file icons)',
    license: 'MIT',
    files: ['src/shared/ui/file-icons/LICENSE-material-icon-theme.txt'],
  },
  {
    name: 'Simple Icons (brand icons)',
    license: 'CC0-1.0',
    files: ['src/assets/icons/LICENSE-simple-icons.md'],
  },
];

/** A fixed command line (no user input), through the shell so pnpm's .cmd shim starts. */
function run(command) {
  return execSync(command, { encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 });
}

function licenseTexts(dir) {
  if (!dir || !existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((file) => LICENSE_FILE.test(file))
    .sort()
    .map((file) => readFileSync(join(dir, file), 'utf8').replace(/\r\n/g, '\n').trim())
    .filter(Boolean);
}

/** Crates linked into the app: normal dependencies of `agentflow`, for every target. */
function rustPackages() {
  const meta = JSON.parse(
    run('cargo metadata --format-version 1 --locked --manifest-path src-tauri/Cargo.toml'),
  );
  const nodes = new Map(meta.resolve.nodes.map((node) => [node.id, node]));
  const packages = new Map(meta.packages.map((pkg) => [pkg.id, pkg]));
  const seen = new Set();
  const queue = [meta.resolve.root];
  while (queue.length > 0) {
    const id = queue.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dep of nodes.get(id)?.deps ?? []) {
      if (dep.dep_kinds.some((kind) => kind.kind === null)) queue.push(dep.pkg);
    }
  }
  seen.delete(meta.resolve.root);
  return [...seen]
    .map((id) => packages.get(id))
    .map((pkg) => ({
      name: pkg.name,
      version: pkg.version,
      license: pkg.license ?? (pkg.license_file ? 'see license file' : 'unknown'),
      url: pkg.repository ?? `https://crates.io/crates/${pkg.name}`,
      texts: licenseTexts(dirname(pkg.manifest_path)),
    }));
}

/** npm packages bundled into the frontend (production dependencies, transitive). */
function npmPackages() {
  const groups = JSON.parse(run('pnpm licenses list --prod --json'));
  return Object.values(groups)
    .flat()
    .flatMap((pkg) =>
      pkg.versions.map((version, i) => ({
        name: pkg.name,
        version,
        license: pkg.license,
        url: pkg.homepage ?? `https://www.npmjs.com/package/${pkg.name}`,
        texts: licenseTexts(pkg.paths[i]),
      })),
    );
}

function assetPackages() {
  return ASSETS.map((asset) => ({
    name: asset.name,
    version: '',
    license: asset.license,
    url: '',
    texts: asset.files.map((file) => readFileSync(file, 'utf8').replace(/\r\n/g, '\n').trim()),
  }));
}

const byName = (a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version);

const COPYRIGHT_LINE = /^\s*(copyright\b|\(c\)|©).*$/gim;

/**
 * Texts that only differ in their copyright lines (most MIT and BSD files) share one body;
 * every copyright line is kept and listed above it.
 */
export function splitCopyright(text) {
  const copyrights = (text.match(COPYRIGHT_LINE) ?? []).map((line) => line.trim());
  const body = text
    .replace(COPYRIGHT_LINE, '')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
  return { copyrights, body };
}

/** The notices document: package lists per group, then each distinct license text once. */
export function render(groups) {
  const texts = new Map();
  const ref = (text, label) => {
    const { copyrights, body } = splitCopyright(text);
    const key = createHash('sha256').update(body.replace(/\s+/g, ' ')).digest('hex');
    const entry = texts.get(key) ?? { body, users: [], copyrights: new Set() };
    entry.users.push(label);
    copyrights.forEach((line) => entry.copyrights.add(line));
    texts.set(key, entry);
  };
  const lines = [
    '# Third-party notices',
    '',
    'AgentFlow is MIT licensed. It includes the following third-party software and assets,',
    'under their own licenses. Generated by `pnpm notices`.',
  ];
  for (const group of groups) {
    lines.push('', `## ${group.title}`, '');
    for (const pkg of [...group.packages].sort(byName)) {
      const label = pkg.version ? `${pkg.name} ${pkg.version}` : pkg.name;
      const url = pkg.url ? ` (${pkg.url})` : '';
      const note = pkg.texts.length === 0 ? ', no license file in the package' : '';
      lines.push(`- ${label}: ${pkg.license}${url}${note}`);
      pkg.texts.forEach((text) => ref(text, label));
    }
  }
  lines.push(
    '',
    '## License texts',
    '',
    'Packages marked "no license file" are under the standard text of the named license',
    '(https://spdx.org/licenses/).',
  );
  for (const { body, users, copyrights } of texts.values()) {
    lines.push('', '---', '', `Used by: ${[...new Set(users)].join(', ')}`);
    if (copyrights.size > 0) lines.push('', ...[...copyrights].map((line) => `    ${line}`));
    lines.push('', '```text', body, '```');
  }
  return `${lines.join('\n')}\n`;
}

function main() {
  const doc = render([
    { title: 'Bundled fonts and icons', packages: assetPackages() },
    { title: 'Rust crates', packages: rustPackages() },
    { title: 'npm packages (frontend)', packages: npmPackages() },
  ]);
  writeFileSync(OUT, doc);
  console.log(`${OUT}: ${Math.round(doc.length / 1024)} KB`);
}

main();
