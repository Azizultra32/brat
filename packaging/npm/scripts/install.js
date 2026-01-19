#!/usr/bin/env node

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");
const os = require("os");

const VERSION = require("../package.json").version;
const REPO = "YOUR_ORG/brat";

function getPlatformInfo() {
  const platform = os.platform();
  const arch = os.arch();

  const platformMap = {
    darwin: "macos",
    linux: "linux",
    win32: "windows",
  };

  const archMap = {
    x64: "x86_64",
    arm64: "aarch64",
  };

  const mappedPlatform = platformMap[platform];
  const mappedArch = archMap[arch];

  if (!mappedPlatform) {
    throw new Error(`Unsupported platform: ${platform}`);
  }
  if (!mappedArch) {
    throw new Error(`Unsupported architecture: ${arch}`);
  }

  return {
    platform: mappedPlatform,
    arch: mappedArch,
    ext: platform === "win32" ? "zip" : "tar.gz",
    binExt: platform === "win32" ? ".exe" : "",
  };
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);

    const request = (url) => {
      https
        .get(url, { headers: { "User-Agent": "brat-npm-installer" } }, (response) => {
          // Handle redirects
          if (response.statusCode === 301 || response.statusCode === 302) {
            request(response.headers.location);
            return;
          }

          if (response.statusCode !== 200) {
            reject(new Error(`Download failed with status ${response.statusCode}`));
            return;
          }

          response.pipe(file);
          file.on("finish", () => {
            file.close(resolve);
          });
        })
        .on("error", (err) => {
          fs.unlink(dest, () => {});
          reject(err);
        });
    };

    request(url);
  });
}

async function install() {
  const info = getPlatformInfo();
  const artifactName = `brat-${info.platform}-${info.arch}.${info.ext}`;
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${artifactName}`;

  console.log(`Downloading brat v${VERSION} for ${info.platform}-${info.arch}...`);

  const binDir = path.join(__dirname, "..", "bin");
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const tempFile = path.join(os.tmpdir(), artifactName);

  try {
    await download(url, tempFile);

    // Extract
    if (info.ext === "tar.gz") {
      execSync(`tar -xzf "${tempFile}" -C "${binDir}"`, { stdio: "inherit" });
    } else {
      // Windows - use PowerShell to extract
      execSync(
        `powershell -command "Expand-Archive -Path '${tempFile}' -DestinationPath '${binDir}' -Force"`,
        { stdio: "inherit" }
      );
    }

    // Make executable on Unix
    const binaryPath = path.join(binDir, `brat${info.binExt}`);
    if (os.platform() !== "win32") {
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log("brat installed successfully!");
  } finally {
    // Cleanup
    if (fs.existsSync(tempFile)) {
      fs.unlinkSync(tempFile);
    }
  }
}

install().catch((err) => {
  console.error("Failed to install brat:", err.message);
  process.exit(1);
});
