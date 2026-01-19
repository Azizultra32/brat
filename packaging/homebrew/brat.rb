class Brat < Formula
  desc "Multi-agent coding orchestrator CLI"
  homepage "https://github.com/neul-labs/brat"
  version "0.1.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/neul-labs/brat/releases/download/v#{version}/brat-macos-aarch64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"
    end
    on_intel do
      url "https://github.com/neul-labs/brat/releases/download/v#{version}/brat-macos-x86_64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_X64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/neul-labs/brat/releases/download/v#{version}/brat-linux-aarch64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    end
    on_intel do
      url "https://github.com/neul-labs/brat/releases/download/v#{version}/brat-linux-x86_64.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X64"
    end
  end

  def install
    bin.install "brat"
  end

  test do
    assert_match "brat #{version}", shell_output("#{bin}/brat --version")
  end
end
