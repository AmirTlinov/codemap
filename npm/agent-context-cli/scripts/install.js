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

function downloadFile(url, destination, redirects = 0) {
  return new Promise((resolve, reject) => {
    https.get(url, (response) => {
      if (
        response.statusCode >= 300 &&
        response.statusCode < 400 &&
        response.headers.location &&
        redirects < 5
      ) {
        response.resume();
        const nextUrl = new URL(response.headers.location, url).toString();
        downloadFile(nextUrl, destination, redirects + 1).then(resolve, reject);
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
    const baseUrl = (
      process.env.CTX_NPM_RELEASE_BASE_URL ||
      `https://github.com/AmirTlinov/ctx/releases/download/${tag}`
    ).replace(/\/$/, "");
    archive = path.join(tempDir, archiveName);
    checksum = `${archive}.sha256`;
    await downloadFile(`${baseUrl}/${archiveName}`, archive);
    await downloadFile(`${baseUrl}/${archiveName}.sha256`, checksum);
  }

  if (!fs.existsSync(archive)) {
    fail(`archive not found: ${archive}`);
  }

  verifyChecksum(archive, checksum);
  extractArchive(archive, archiveBase, vendorDir);
  console.log(`ctx ${tag} installed for ${target}`);
}

main().catch((error) => fail(error.message));
