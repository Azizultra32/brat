require "net/http"
require "uri"
require "fileutils"
require "tempfile"
require "rubygems/package"
require "zlib"

begin
  require "zip"
rescue LoadError
  # zip gem is optional, only needed for Windows
end

module BratCLI
  VERSION = "0.1.0"
  REPO = "YOUR_ORG/brat"

  class << self
    def binary_dir
      File.join(File.dirname(__FILE__), "..", "bin")
    end

    def binary_path
      binary_name = Gem.win_platform? ? "brat.exe" : "brat"
      File.join(binary_dir, binary_name)
    end

    def platform_info
      platform = case RUBY_PLATFORM
                 when /darwin/ then "macos"
                 when /linux/ then "linux"
                 when /mingw|mswin|cygwin/ then "windows"
                 else raise "Unsupported platform: #{RUBY_PLATFORM}"
                 end

      arch = case RUBY_PLATFORM
             when /x86_64|x64|amd64/ then "x86_64"
             when /arm64|aarch64/ then "aarch64"
             else "x86_64" # Default to x86_64
             end

      ext = platform == "windows" ? "zip" : "tar.gz"

      [platform, arch, ext]
    end

    def download_file(url)
      uri = URI(url)
      response = nil

      # Follow redirects (up to 5)
      5.times do
        http = Net::HTTP.new(uri.host, uri.port)
        http.use_ssl = uri.scheme == "https"

        request = Net::HTTP::Get.new(uri)
        request["User-Agent"] = "brat-gem-installer"

        response = http.request(request)

        if response.is_a?(Net::HTTPRedirection)
          uri = URI(response["location"])
        else
          break
        end
      end

      raise "Download failed: #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      response.body
    end

    def ensure_binary
      return binary_path if File.exist?(binary_path)

      platform, arch, ext = platform_info
      artifact = "brat-#{platform}-#{arch}.#{ext}"
      url = "https://github.com/#{REPO}/releases/download/v#{VERSION}/#{artifact}"

      warn "Downloading brat #{VERSION} for #{platform}-#{arch}..."

      FileUtils.mkdir_p(binary_dir)

      Tempfile.create(["brat", ".#{ext}"]) do |tmp|
        tmp.binmode
        tmp.write(download_file(url))
        tmp.flush

        if ext == "tar.gz"
          # Extract tar.gz
          Gem::Package::TarReader.new(Zlib::GzipReader.open(tmp.path)) do |tar|
            tar.each do |entry|
              if entry.file?
                dest = File.join(binary_dir, entry.full_name)
                File.open(dest, "wb") { |f| f.write(entry.read) }
              end
            end
          end
        else
          # Extract zip (requires rubyzip gem)
          Zip::File.open(tmp.path) do |zip|
            zip.each do |entry|
              dest = File.join(binary_dir, entry.name)
              entry.extract(dest) { true } # Overwrite existing
            end
          end
        end
      end

      # Make executable
      File.chmod(0o755, binary_path) unless Gem.win_platform?

      warn "brat installed to #{binary_path}"
      binary_path
    end

    def run(args)
      binary = ensure_binary
      exec(binary, *args)
    end
  end
end
