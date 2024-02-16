# `nexus-staging-maven-plugin`

## Relevant Configurations

- `version`: 1.6.13
- `autoReleaseAfterClose`: without this set to `true`, publishing requires
  logging in to the NXRM2 instance to manually propagate a build to Central

## Plugin Requests

### `/service/local/status`

### Interesting Headers

- `Authorization`: `Basic`
- `Accept`: `application/xml; charset=UTF-8`
- `Content-type`: `application/xml; charset=UTF-8`
