# Gradle-nexus's `publish-plugin` plugin

- [Source
  Code](https://github.com/gradle-nexus/publish-plugin/)

## Relevant Configurations

- Gradle version: 8.5
- Plugin version: 2.0.0
- Run command: `gradle publishToSonatype closeAndReleaseSonatypeStagingRepository`

## Plugin Requests

### Interesting Headers

- `Authorization`: `Basic`
- `Accept`: `application/json`
- `User-agent`: `gradle-nexus-publish-plugin/2.0.0`

### `GET /service/local/staging/profiles`

- [NXRM2
  Documentation](https://s01.oss.sonatype.org/nexus-staging-plugin/default/docs/path__staging_profiles.html)
- [Calling
  code](https://github.com/gradle-nexus/publish-plugin/blob/c9b561160883d9e230e087dcb4fa45f6aeb44874/src/main/kotlin/io/github/gradlenexus/publishplugin/internal/NexusClient.kt#L74)

This appears to be an approach to getting all of the profiles that a user has
access to, and then filtering based on the profile name being a prefix of the
package group.

### `POST /service/local/staging/profiles/<profileId>/start`

Same as documented in [the `nexus-staging-maven-plugin`
README](../nexus-staging-maven-plugin/README.md) (but JSON).

### `PUT /service/local/staging/deployByRepositoryId/<filePath>`

Same as documented in [the `nexus-staging-maven-plugin`
README](../nexus-staging-maven-plugin/README.md).

### `POST /service/local/staging/bulk/close`

### `GET /service/local/staging/repository/<repositoryId>` (Get close status)

Expects a response with a `type` of `closed` ([Calling
Code](https://github.com/gradle-nexus/publish-plugin/blob/bdb9be94aa411e1b62b153cf4d1b316e36ea5f77/src/main/kotlin/io/github/gradlenexus/publishplugin/internal/StagingRepositoryTransitioner.kt#L30)).

### `POST /service/local/staging/bulk/promote`

Same as documented in [the `nexus-staging-maven-plugin`
README](../nexus-staging-maven-plugin/README.md) (but JSON).

### `GET /service/local/staging/repository/<repositoryId>` (Get promote status)

Expects a response with a `type` of `released` or `not_found` ([Calling Code](https://github.com/gradle-nexus/publish-plugin/blob/bdb9be94aa411e1b62b153cf4d1b316e36ea5f77/src/main/kotlin/io/github/gradlenexus/publishplugin/internal/StagingRepositoryTransitioner.kt#L34)).
