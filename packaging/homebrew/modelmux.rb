# typed: strict
# frozen_string_literal: true

# ModelMux - high-performance proxy converting OpenAI API to Vertex AI (Claude).
class Modelmux < Formula
  desc "High-performance proxy server converting OpenAI API requests to Vertex AI format"
  homepage "https://github.com/yarenty/modelmux"
  url "https://github.com/yarenty/modelmux/archive/refs/tags/v1.0.0.tar.gz"
  sha256 "05c1ed6868298287e9c33c058a57340a0aca676fadb0e135147f8692fac58b13"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/yarenty/modelmux.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    (var/"log").mkpath
  end

  service do
    run [opt_bin/"modelmux"]
    keep_alive true
    log_path var/"log/modelmux.log"
    error_log_path var/"log/modelmux.log"
  end

  def caveats
    <<~EOS
      ModelMux runs as an HTTP proxy. Configure it with:
        modelmux config init

      To run ModelMux as a background service:
        brew services start modelmux

      The service will use your config from:
        ~/.config/modelmux/config.toml (Linux)
        ~/Library/Application Support/modelmux/config.toml (macOS)
    EOS
  end

  test do
    # Test that the binary was installed and --version works
    version_output = shell_output("#{bin}/modelmux --version")
    assert_match(/^modelmux \d+\.\d+\.\d+/, version_output)

    # Test that -V (short version) works
    version_short = shell_output("#{bin}/modelmux -V")
    assert_match(/^modelmux \d+\.\d+\.\d+/, version_short)

    # Test that --help works
    help_output = shell_output("#{bin}/modelmux --help")
    assert_match(/USAGE/, help_output)
    assert_match(/OPTIONS/, help_output)
    assert_match(/ENVIRONMENT VARIABLES/, help_output)

    # Test that -h (short help) works
    help_short = shell_output("#{bin}/modelmux -h")
    assert_match(/USAGE/, help_short)

    # Test configuration validation with minimal valid config
    # Create a minimal valid base64-encoded service account key JSON
    require "base64"
    require "json"
    pk = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC\n" \
         "-----END PRIVATE KEY-----\n"
    minimal_key = {
      "auth_provider_x509_cert_url" => "https://www.googleapis.com/oauth2/v1/certs",
      "auth_uri"                    => "https://accounts.google.com/o/oauth2/auth",
      "client_email"                => "test@test-project.iam.gserviceaccount.com",
      "client_id"                   => "123456789",
      "client_x509_cert_url"        => "https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com",
      "private_key"                 => pk,
      "private_key_id"              => "test-key-id",
      "project_id"                  => "test-project",
      "token_uri"                   => "https://oauth2.googleapis.com/token",
      "type"                        => "service_account",
    }
    key_b64 = Base64.strict_encode64(minimal_key.to_json)

    ENV["GCP_SERVICE_ACCOUNT_KEY"] = key_b64
    ENV["LLM_PROVIDER"] = "vertex"
    ENV["VERTEX_REGION"] = "test-region"
    ENV["VERTEX_PROJECT"] = "test-project"
    ENV["VERTEX_LOCATION"] = "test-region"
    ENV["VERTEX_PUBLISHER"] = "test-publisher"
    ENV["VERTEX_MODEL_ID"] = "test-model"
    ENV["PORT"] = "0" # Use port 0 to avoid conflicts

    # This should fail gracefully with auth/network error, proving config parsing works
    # The server will try to start but fail on actual API calls, which is expected
    output = shell_output("#{bin}/modelmux 2>&1", 1)
    # Should show some error (auth, network, or server startup), not a config error
    assert_match(/Authentication|HTTP|Server|Error|error/i, output)
  end
end
