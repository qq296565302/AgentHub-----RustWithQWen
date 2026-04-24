$json = '{"test_type":"upper"}'
Write-Host "JSON: $json"
cargo run --quiet -- skill run test.skill $json
