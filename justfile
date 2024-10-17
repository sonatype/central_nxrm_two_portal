default:
  @just --list

apply-license-headers:
  licensure --project

# Run the CI tests locally
ci:
  @echo 'Run the test suite'
  cargo nextest run --profile ci

  @echo 'Check formatting'
  cargo fmt --check

  @echo 'Check licenses'
  licensure --project --check

  @echo 'Check Clippy'
  cargo clippy

# Spin up the proxy pointing at production
run-prod:
  RUST_LOG=debug,nxrm_two_portal::endpoints::fallback=trace,h2=info,hyper=info,reqwest=info,rustls=info \
  nix run

# Spin up the proxy pointing at staging
run-staging:
  RUST_LOG=debug,nxrm_two_portal::endpoints::fallback=trace,h2=info,hyper=info,reqwest=info,rustls=info \
  NXRM_TWO_PORTAL_CENTRAL_URL=https://staging.portal.central.sonatype.dev \
  nix run

# Spin up the proxy pointing at a locally running server
run-local:
  RUST_LOG=debug,nxrm_two_portal::endpoints::fallback=trace,nxrm_two_portal::auth=trace,user_auth=trace,h2=info,hyper=info,reqwest=info,rustls=info \
  NXRM_TWO_PORTAL_CENTRAL_URL=http://localhost:3000 \
  NXRM_TWO_PORTAL_JWT_PUBLIC_KEY_PATH=ossrh-proxy-service-dev-public-key.pem \
  nix run
