default:
  @just --list

test:
  nix flake check

run-prod:
  RUST_LOG=debug,h2=info,hyper=info,reqwest=info,rustls=info \
  nix run

run-staging:
  RUST_LOG=debug,h2=info,hyper=info,reqwest=info,rustls=info \
  NXRM_TWO_PORTAL_CENTRAL_URL=https://staging.portal.central.sonatype.dev \
  nix run

run-local:
  RUST_LOG=debug,h2=info,hyper=info,reqwest=info,rustls=info \
  NXRM_TWO_PORTAL_CENTRAL_URL=http://localhost:3000 \
  nix run
