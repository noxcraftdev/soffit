class Soffit < Formula
  desc "Customizable statusline manager for Claude Code"
  homepage "https://github.com/noxcraftdev/soffit"
  version "0.0.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/noxcraftdev/soffit/releases/download/v0.0.1/soffit-x86_64-apple-darwin.tar.gz"
      sha256 "61bb5eb3c2d4d257d730e20012855750544f50fb464e92fdda712921129be77f"
    elsif Hardware::CPU.arm?
      url "https://github.com/noxcraftdev/soffit/releases/download/v0.0.1/soffit-aarch64-apple-darwin.tar.gz"
      sha256 "b5d1a6651092ae991cb54ebfe94166a62ed61e9a831a6cdb01ff7e5fefc91a3a"
    end
  end

  on_linux do
    url "https://github.com/noxcraftdev/soffit/releases/download/v0.0.1/soffit-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "ccc24d57499fb0f1a84475ed12eaed7cc7e98b79ae3873cbad95143392a68134"
  end

  def install
    bin.install "soffit"
  end

  test do
    assert_match "soffit", shell_output("#{bin}/soffit --version")
  end
end
