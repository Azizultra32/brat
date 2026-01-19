"""
Brat CLI - Multi-agent coding orchestrator

This package provides a thin wrapper around the native brat binary.
The binary is downloaded on first run.
"""

import os
import sys
import platform
import urllib.request
import tarfile
import zipfile
import tempfile
import stat
from pathlib import Path

__version__ = "0.1.0"
REPO = "YOUR_ORG/brat"


def get_binary_dir() -> Path:
    """Get the directory where the binary is stored."""
    return Path(__file__).parent / "bin"


def get_binary_path() -> Path:
    """Get path to the brat binary."""
    binary_name = "brat.exe" if platform.system() == "Windows" else "brat"
    return get_binary_dir() / binary_name


def get_platform_info() -> tuple:
    """Get platform and architecture info."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    platform_map = {"darwin": "macos", "linux": "linux", "windows": "windows"}
    arch_map = {
        "x86_64": "x86_64",
        "amd64": "x86_64",
        "arm64": "aarch64",
        "aarch64": "aarch64",
    }

    plat = platform_map.get(system)
    arch = arch_map.get(machine)

    if not plat:
        raise RuntimeError(f"Unsupported platform: {system}")
    if not arch:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    ext = "zip" if system == "windows" else "tar.gz"
    return plat, arch, ext


def download_binary() -> Path:
    """Download the native binary if not present."""
    binary_path = get_binary_path()

    if binary_path.exists():
        return binary_path

    plat, arch, ext = get_platform_info()
    artifact_name = f"brat-{plat}-{arch}.{ext}"
    url = f"https://github.com/{REPO}/releases/download/v{__version__}/{artifact_name}"

    print(f"Downloading brat v{__version__} for {plat}-{arch}...", file=sys.stderr)

    bin_dir = get_binary_dir()
    bin_dir.mkdir(parents=True, exist_ok=True)

    with tempfile.NamedTemporaryFile(suffix=f".{ext}", delete=False) as tmp:
        try:
            urllib.request.urlretrieve(url, tmp.name)

            if ext == "tar.gz":
                with tarfile.open(tmp.name, "r:gz") as tar:
                    tar.extractall(bin_dir)
            else:
                with zipfile.ZipFile(tmp.name, "r") as zip_ref:
                    zip_ref.extractall(bin_dir)
        finally:
            os.unlink(tmp.name)

    # Make executable on Unix
    if platform.system() != "Windows":
        binary_path.chmod(binary_path.stat().st_mode | stat.S_IEXEC)

    print(f"brat installed to {binary_path}", file=sys.stderr)
    return binary_path


def main():
    """Entry point that delegates to the native binary."""
    try:
        binary = download_binary()
    except Exception as e:
        print(f"Error downloading brat: {e}", file=sys.stderr)
        sys.exit(1)

    # Execute the binary with the same arguments
    os.execv(str(binary), [str(binary)] + sys.argv[1:])


if __name__ == "__main__":
    main()
