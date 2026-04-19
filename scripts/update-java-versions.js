#!/usr/bin/env node

/**
 * 从 Adoptium GitHub Releases 查询最新 Java 版本，
 * 生成简化的 config/java_versions.toml（LTS + 最新非 LTS）
 */

const https = require('https');
const http = require('http');

const REPOS = [
  { repo: 'temurin8-binaries', major: 8, lts: true },
  { repo: 'temurin11-binaries', major: 11, lts: true },
  { repo: 'temurin17-binaries', major: 17, lts: true },
  { repo: 'temurin21-binaries', major: 21, lts: true },
  { repo: 'temurin25-binaries', major: 25, lts: false },
];

const PLATFORMS = [
  { key: 'windows-x64', pattern: /jdk_x64_windows_hotspot/ },
  { key: 'linux-x64', pattern: /jdk_x64_linux_hotspot/ },
  { key: 'linux-aarch64', pattern: /jdk_aarch64_linux_hotspot/ },
  { key: 'macos-x64', pattern: /jdk_x64_mac_hotspot/ },
  { key: 'macos-aarch64', pattern: /jdk_aarch64_mac_hotspot/ },
];

function fetchJSON(url) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { 'User-Agent': 'fnva-ci' } }, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        try { resolve(JSON.parse(data)); }
        catch (e) { reject(new Error(`Parse error: ${e.message}`)); }
      });
    }).on('error', reject);
  });
}

function parseVersion(tagName) {
  // jdk-21.0.10+7 -> { version: "21.0.10", tag: "jdk-21.0.10+7" }
  // jdk8u482-b08 -> { version: "8u482b08", tag: "jdk8u482-b08" }
  const match = tagName.match(/^jdk-?([\d.]+u?\d*)(?:\+|b|-b?)(\d+)$/i);
  if (!match) return null;
  const base = match[1].replace(/\.0(?=\d)/g, '.');
  const build = match[2];
  if (tagName.includes('jdk8u')) {
    return { version: `8u${base.replace(/^8u?/, '')}b${build.padStart(2, '0')}`, tag: tagName };
  }
  return { version: `${base}+${build}`, tag: tagName };
}

async function getLatestVersion(repo, major) {
  const url = `https://api.github.com/repos/adoptium/${repo}/releases?per_page=5`;
  const releases = await fetchJSON(url);

  for (const release of releases) {
    if (release.prerelease) continue;
    const parsed = parseVersion(release.tag_name);
    if (!parsed) continue;

    const assets = {};
    for (const asset of release.assets) {
      for (const p of PLATFORMS) {
        if (p.pattern.test(asset.name) && (asset.name.endsWith('.zip') || asset.name.endsWith('.tar.gz'))) {
          assets[p.key] = asset.name;
        }
      }
    }

    if (Object.keys(assets).length >= 3) {
      return { ...parsed, major, assets };
    }
  }
  return null;
}

function toToml(entry) {
  let lines = [
    `[[versions]]`,
    `version = "${entry.version}"`,
    `major = ${entry.major}`,
    `lts = ${entry.lts}`,
    `tag_name = "${entry.tag}"`,
    `[versions.assets]`,
  ];
  for (const [key, name] of Object.entries(entry.assets)) {
    lines.push(`${key} = "${name}"`);
  }
  return lines.join('\n');
}

// --- Mirror URL verification ---

const MIRRORS = [
  { name: 'tsinghua', template: '{base_url}/{major}/jdk/{arch}/{os}/{filename}',
    base_url: 'https://mirrors.tuna.tsinghua.edu.cn/Adoptium' },
  { name: 'aliyun', template: '{base_url}/{major}/{tag}/{filename}',
    base_url: 'https://mirrors.aliyun.com/eclipse/temurin-compliance/temurin' },
  { name: 'github', template: 'https://github.com/adoptium/temurin{major}-binaries/releases/download/{tag}/{filename}',
    base_url: '' },
];

function renderMirrorUrl(template, base_url, major, tag, filename, os, arch) {
  return template
    .replace('{base_url}', base_url)
    .replace('{major}', String(major))
    .replace('{tag}', tag)
    .replace('{filename}', filename)
    .replace('{os}', os)
    .replace('{arch}', arch);
}

function headCheck(url) {
  return new Promise((resolve) => {
    const mod = url.startsWith('https') ? https : http;
    const req = mod.request(url, { method: 'HEAD', timeout: 10000 }, (res) => {
      resolve(res.statusCode >= 200 && res.statusCode < 400);
    });
    req.on('error', () => resolve(false));
    req.on('timeout', () => { req.destroy(); resolve(false); });
    req.end();
  });
}

async function verifyMirrorUrls(entries) {
  console.log('\n--- Mirror URL verification ---');
  let hasFailure = false;

  for (const entry of entries) {
    const filename = entry.assets['linux-x64'];
    if (!filename) {
      console.log(`  [SKIP] Java ${entry.major}: no linux-x64 asset`);
      continue;
    }

    let anyOk = false;
    for (const mirror of MIRRORS) {
      const url = renderMirrorUrl(mirror.template, mirror.base_url, entry.major, entry.tag, filename, 'linux', 'x64');
      const ok = await headCheck(url);
      const mark = ok ? 'OK' : 'FAIL';
      console.log(`  [${mark}] Java ${entry.major} ${mirror.name}: ${url.substring(0, 80)}...`);
      if (ok) anyOk = true;
    }

    if (!anyOk) {
      console.log(`  [ERROR] Java ${entry.major}: ALL mirrors unreachable!`);
      hasFailure = true;
    }
  }

  return !hasFailure;
}

async function main() {
  console.log('🔍 查询 Adoptium 最新版本...');

  const entries = [];
  for (const { repo, major, lts } of REPOS) {
    console.log(`  检查 ${repo}...`);
    try {
      const entry = await getLatestVersion(repo, major);
      if (entry) {
        entries.push({ ...entry, lts });
        console.log(`  ✅ Java ${major}: ${entry.version}`);
      } else {
        console.log(`  ⚠️  Java ${major}: 未找到可用版本`);
      }
    } catch (e) {
      console.log(`  ❌ Java ${major}: ${e.message}`);
    }
  }

  entries.sort((a, b) => b.major - a.major);

  const toml = entries.map(toToml).join('\n\n') + '\n';

  const fs = require('fs');
  const path = require('path');
  const targetPath = path.join(__dirname, '..', 'config', 'java_versions.toml');

  const existing = fs.readFileSync(targetPath, 'utf-8');
  if (existing.trim() === toml.trim()) {
    console.log('\n📋 版本注册表无变化');
    process.exit(0);
  }

  fs.writeFileSync(targetPath, toml, 'utf-8');
  console.log(`\n✅ 已更新 ${targetPath}`);

  // Verify all mirror URLs are reachable
  const ok = await verifyMirrorUrls(entries);
  if (!ok) {
    console.error('\n❌ Mirror URL verification failed! Check templates in config.rs');
    process.exit(1);
  }
  console.log('\n✅ All mirror URLs verified');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
