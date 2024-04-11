# The `nxrm_two_portal` Project

## `nxrm_two_portal`

This is the core service focused on implementing a subset of the NXRM2 API that
is used by various publishing plugins and tools. The service is currently
implemented as an `axum` server, but in the future the intent is to pull out the
core functionality into an AWS Lambda Function.

## `portal_api`

This is the API related to publishing via the new Central Publisher Portal.

- [OpenAPI Specification / Swagger UI](https://central.sonatype.com/api-doc)
- [Publishing Guide](https://central.sonatype.org/publish-ea/publish-ea-guide/)

## `example_projects`

This is a collection of projects that demonstrate real-world usages of the
plugins we intend to support with the translation API.

### `nexus-staging-maven-plugin`

- [`README.md`](example_projects/nexus-staging-maven-plugin/README.md)
- [Plugin
  `README.md`](https://github.com/sonatype/nexus-maven-plugins/blob/main/staging/maven-plugin/README.md)

## Local Setup

For local development, we recommend using
[Nix](https://github.com/DeterminateSystems/nix-installer) +
[Direnv](https://direnv.net/). This will provide a development environment with
all required dependencies.

Using the Nix setup provides convenience wrappers for Maven have been provided
(`mvnLocalProxy`, `mvnStagingProxy`, & `mvnProductionProxy`). They expect valid
settings files with a server `central.testing` and a name
`settings-<environment>.xml` (`local`, `staging`, & `production`). There is a
`settings-example.xml` included for convenience.

### Common commands

#### Build & Run Tests

``` shell
nix flake check
```

### Run The Local Proxy

``` shell
just run-local
```

Note: This reduces some of the noise around HTTPS requests to the Portal

### Non-Nix

You'll need the following installed:

- Rust (`cargo`)
- Java (`java`, `mvn`)
- GPG (`gpg`)

## License

This code is licensed under the dual-license approach of [MIT](./LICENSE-MIT) OR
[Apache-2.0](./LICENSE-APACHE).
