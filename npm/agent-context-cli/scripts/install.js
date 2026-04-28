#!/usr/bin/env node
"use strict";

const crypto = require("node:crypto");
const fs = require("node:fs");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const packageRoot = path.resolve(__dirname, "..");
const pkg = JSON.parse(fs.readFileSync(path.join(packageRoot, "package.json"), "utf8"));
const githubToken =
  process.env.CTX_NPM_GITHUB_TOKEN ||
  process.env.GH_TOKEN ||
  process.env.GITHUB_TOKEN ||
  "";

const TARGETS = {
  "darwin-arm64": "aarch64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-gnu"
};

function fail(message) {
  console.error(`ctx npm install failed: ${message}`);
  process.exit(1);
}

function targetTriple() {
  const key = `${process.platform}-${process.arch}`;
  const target = TARGETS[key];
  if (!target) {
    const supported = Object.keys(TARGETS).join(", ");
    fail(`unsupported platform ${key}; supported prebuilt targets: ${supported}`);
  }
  return target;
}

function sha256(file) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(file));
  return hash.digest("hex");
}

function verifyChecksum(archive, checksumFile) {
  if (!fs.existsSync(checksumFile)) {
    fail(`checksum file missing: ${checksumFile}`);
  }

  const expected = fs.readFileSync(checksumFile, "utf8").trim().split(/\s+/)[0];
  const actual = sha256(archive);
  if (expected !== actual) {
    fail(`checksum mismatch for ${path.basename(archive)}`);
  }
}

function requestHeaders(accept = "application/octet-stream", withAuth = false) {
  const headers = {
    "Accept": accept,
    "User-Agent": "agent-context-cli-npm-installer"
  };
  if (githubToken && withAuth) {
    headers.Authorization = `Bearer ${githubToken}`;
    headers["X-GitHub-Api-Version"] = "2022-11-28";
  }
  return headers;
}

function downloadFile(
  url,
  destination,
  redirects = 0,
  accept = "application/octet-stream",
  withAuth = false
) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: requestHeaders(accept, withAuth) }, (response) => {
      if (
        response.statusCode >= 300 &&
        response.statusCode < 400 &&
        response.headers.location &&
        redirects < 5
      ) {
        response.resume();
        const nextUrl = new URL(response.headers.location, url).toString();
        downloadFile(nextUrl, destination, redirects + 1, accept, false).then(resolve, reject);
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`download failed (${response.statusCode}): ${url}`));
        return;
      }

      const file = fs.createWriteStream(destination);
      response.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", reject);
    }).on("error", reject);
  });
}

function getJson(url, redirects = 0, withAuth = true) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: requestHeaders("application/vnd.github+json", withAuth) }, (response) => {
      if (
        response.statusCode >= 300 &&
        response.statusCode < 400 &&
        response.headers.location &&
        redirects < 5
      ) {
        response.resume();
        const nextUrl = new URL(response.headers.location, url).toString();
        getJson(nextUrl, redirects + 1, false).then(resolve, reject);
        return;
      }

      let body = "";
      response.setEncoding("utf8");
      response.on("data", (chunk) => {
        body += chunk;
      });
      response.on("end", () => {
        if (response.statusCode !== 200) {
          reject(new Error(`GitHub API request failed (${response.statusCode}): ${url}`));
          return;
        }
        try {
          resolve(JSON.parse(body));
        } catch (error) {
          reject(error);
        }
      });
    }).on("error", reject);
  });
}

async function downloadGitHubReleaseAsset(tag, assetName, destination) {
  const apiUrl =
    process.env.CTX_NPM_RELEASE_API_URL ||
    `https://api.github.com/repos/AmirTlinov/ctx/releases/tags/${encodeURIComponent(tag)}`;
  const release = await getJson(apiUrl);
  const asset = (release.assets || []).find((candidate) => candidate.name === assetName);
  if (!asset || !asset.url) {
    fail(`release asset not found for ${tag}: ${assetName}`);
  }
  await downloadFile(asset.url, destination, 0, "application/octet-stream", true);
}

function extractArchive(archive, archiveBase, vendorDir) {
  const stage = fs.mkdtempSync(path.join(os.tmpdir(), "ctx-npm-"));
  const result = spawnSync("tar", ["-xzf", archive, "-C", stage], {
    stdio: "inherit"
  });
  if (result.status !== 0) {
    fail("failed to extract release archive with tar");
  }

  const nativeName = process.platform === "win32" ? "ctx.exe" : "ctx";
  const extractedBinary = path.join(stage, archiveBase, nativeName);
  if (!fs.existsSync(extractedBinary)) {
    fail(`release archive did not contain ${archiveBase}/${nativeName}`);
  }

  fs.mkdirSync(vendorDir, { recursive: true });
  const installedBinary = path.join(vendorDir, nativeName);
  fs.copyFileSync(extractedBinary, installedBinary);
  fs.chmodSync(installedBinary, 0o755);
}

async function main() {
  const target = targetTriple();
  const tag = process.env.CTX_NPM_INSTALL_TAG || `v${pkg.version}`;
  const archiveBase = `ctx-${tag}-${target}`;
  const archiveName = `${archiveBase}.tar.gz`;
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "ctx-npm-download-"));
  const vendorDir = process.env.CTX_NPM_VENDOR_DIR
    ? path.resolve(process.env.CTX_NPM_VENDOR_DIR)
    : path.join(packageRoot, "vendor");

  let archive = process.env.CTX_NPM_INSTALL_ARCHIVE
    ? path.resolve(process.env.CTX_NPM_INSTALL_ARCHIVE)
    : "";
  let checksum = archive ? `${archive}.sha256` : "";

  if (!archive) {
    archive = path.join(tempDir, archiveName);
    checksum = `${archive}.sha256`;
    if (githubToken && !process.env.CTX_NPM_RELEASE_BASE_URL) {
      await downloadGitHubReleaseAsset(tag, archiveName, archive);
      await downloadGitHubReleaseAsset(tag, `${archiveName}.sha256`, checksum);
    } else {
      const baseUrl = (
        process.env.CTX_NPM_RELEASE_BASE_URL ||
        `https://github.com/AmirTlinov/ctx/releases/download/${tag}`
      ).replace(/\/$/, "");
      await downloadFile(`${baseUrl}/${archiveName}`, archive);
      await downloadFile(`${baseUrl}/${archiveName}.sha256`, checksum);
    }
  }

  if (!fs.existsSync(archive)) {
    fail(`archive not found: ${archive}`);
  }

  verifyChecksum(archive, checksum);
  extractArchive(archive, archiveBase, vendorDir);
  console.log(`ctx ${tag} installed for ${target}`);
}

main().catch((error) => fail(error.message));
