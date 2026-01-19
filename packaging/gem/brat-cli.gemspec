Gem::Specification.new do |spec|
  spec.name          = "brat-cli"
  spec.version       = "0.1.0"
  spec.authors       = ["Brat Authors"]
  spec.email         = ["brat@example.com"]

  spec.summary       = "Multi-agent coding orchestrator CLI"
  spec.description   = "Brat is a CLI tool for orchestrating multiple AI coding agents including Claude Code, Codex, Aider, and more."
  spec.homepage      = "https://github.com/neul-labs/brat"
  spec.license       = "MIT OR Apache-2.0"
  spec.required_ruby_version = ">= 2.7.0"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/neul-labs/brat"
  spec.metadata["changelog_uri"] = "https://github.com/neul-labs/brat/releases"

  spec.files         = Dir["lib/**/*", "exe/*", "LICENSE*", "README.md"]
  spec.bindir        = "exe"
  spec.executables   = ["brat"]
  spec.require_paths = ["lib"]

  spec.add_dependency "rubyzip", "~> 2.3"

  spec.post_install_message = "Run 'brat --version' to verify installation."
end
