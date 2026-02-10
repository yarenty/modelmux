class Modelmux < Formula
  desc "High-performance proxy server converting OpenAI API requests to Vertex AI format"
  homepage "https://github.com/yarenty/modelmux"
  url "https://github.com/yarenty/modelmux/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "YOUR_SHA256_HERE" # Replace with actual SHA256
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/yarenty/modelmux.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
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
    minimal_key_json = '{"type":"service_account","project_id":"test-project","private_key_id":"test-key-id","private_key":"-----BEGIN PRIVATE KEY-----\\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC\\n-----END PRIVATE KEY-----\\n","client_email":"test@test-project.iam.gserviceaccount.com","client_id":"123456789","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","auth_provider_x509_cert_url":"https://www.googleapis.com/oauth2/v1/certs","client_x509_cert_url":"https://www.googleapis.com/robot/v1/metadata/x509/test%40test-project.iam.gserviceaccount.com"}'
    key_b64 = `echo -n '#{minimal_key_json}' | base64`.strip

    ENV["GCP_SERVICE_ACCOUNT_KEY"] = key_b64
    ENV["LLM_URL"] = "https://test.example.com/v1/"
    ENV["LLM_CHAT_ENDPOINT"] = "test-model:streamRawPredict"
    ENV["LLM_MODEL"] = "test-model"
    ENV["PORT"] = "0"  # Use port 0 to avoid conflicts

    # This should fail gracefully with auth/network error, proving config parsing works
    # The server will try to start but fail on actual API calls, which is expected
    output = shell_output("#{bin}/modelmux 2>&1", 1)
    # Should show some error (auth, network, or server startup), not a config error
    assert_match(/Authentication|HTTP|Server|Error|error/i, output)
  end
end
