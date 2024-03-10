# `nexus-staging-maven-plugin`

## Relevant Configurations

- `version`: 1.6.13
- `autoReleaseAfterClose`: without this set to `true`, publishing requires
  logging in to the NXRM2 instance to manually propagate a build to Central

## Plugin Requests

### Interesting Headers

- `Authorization`: `Basic`
- `Accept`: `application/xml; charset=UTF-8`
- `Content-type`: `application/xml; charset=UTF-8`

### `GET /service/local/status`

- [NXRM2
  Documentation](https://s01.oss.sonatype.org/nexus-restlet1x-plugin/default/docs/path__status.html)
- [Calling
  code](https://github.com/sonatype/nexus-maven-plugins/blob/43a9940b134c3f87ebe4daa82552e844d9c578b8/staging/maven-plugin/src/main/java/org/sonatype/nexus/maven/staging/remote/RemoteNexus.java#L194)

This seems to be used as a basic health-check endpoint, so hardcoding the
response appears to work correctly.

### `GET /service/local/staging/profile_evaluate?a=<artifact>&t=maven2&v=<version>&g=<group>`

- [NXRM2
  Documentation](https://s01.oss.sonatype.org/nexus-staging-plugin/default/docs/path__staging_profile_evaluate.html)
- [Calling
  code](https://github.com/sonatype/nexus-maven-plugins/blob/43a9940b134c3f87ebe4daa82552e844d9c578b8/staging/maven-plugin/src/main/java/org/sonatype/nexus/maven/staging/deploy/strategy/AbstractStagingDeployStrategy.java#L82)

This appears to create a staging profile and return the profile ID.

### `GET /service/local/staging/profiles/<profileId>`

- [NXRM2
  Documentation](https://s01.oss.sonatype.org/nexus-staging-plugin/default/docs/path__staging_profiles_-profileIdKey-.html)
- [Calling
  code](https://github.com/sonatype/nexus-maven-plugins/blob/43a9940b134c3f87ebe4daa82552e844d9c578b8/staging/maven-plugin/src/main/java/org/sonatype/nexus/maven/staging/deploy/strategy/StagingDeployStrategy.java#L121)

This appears to turn around and immediately request the profile we found by ID.
Although this is a number by convention, we appear to be able to smuggle the
namespace through this field to future endpoints without issue, in order to
avoid generating and retrieving ID values.

### `POST /service/local/staging/profiles/io.github.amy-keibler/start`

``` xml
<promoteRequest>
  <data>
    <description>io.github.amy-keibler:example_nexus_staging_maven_plugin:0.0.1</description>
  </data>
</promoteRequest>
```

- [NXRM2
  Documentation](https://s01.oss.sonatype.org/nexus-staging-plugin/default/docs/path__staging_profiles_-profileIdKey-_start.html)
- [Calling
  code](https://github.com/sonatype/nexus-maven-plugins/blob/43a9940b134c3f87ebe4daa82552e844d9c578b8/staging/maven-plugin/src/main/java/org/sonatype/nexus/maven/staging/deploy/strategy/AbstractStagingDeployStrategy.java#L107)

### `PUT /service/local/staging/deployByRepositoryId/<stagingRepositoryId>/<fullArtifactPath>`

There does not appear to be public documentation and the implementation gets
deep in an abstraction hole without a clear line of code to link.

The corresponding debug output from manual testing is:

``` shell
2024-03-10T00:37:02.503073Z DEBUG fallback: nxrm_two_portal::endpoints::fallback: Request to PUT: /service/local/staging/deployByRepositoryId/io.github.amy-keibler-1/io/github/amy-keibler/example_nexus_staging_maven_plugin/0.0.1/example_nexus_staging_maven_plugin-0.0.1-javadoc.jar.asc
2024-03-10T00:37:02.503186Z TRACE fallback: nxrm_two_portal::endpoints::fallback: Headers: {
    "cache-control": "no-cache, no-store",
    "pragma": "no-cache",
    "expect": "100-continue",
    "content-length": "488",
    "host": "localhost:2727",
    "connection": "Keep-Alive",
    "user-agent": "Apache-Maven/3.9.5 (Java 17.0.8.1; Mac OS X 14.2.1)",
    "accept-encoding": "gzip,deflate",
    "authorization": "Basic ZmFrZV91c2VybmFtZTpmYWtlX3Bhc3N3b3Jk",
}
2024-03-10T00:37:02.503312Z TRACE fallback: nxrm_two_portal::endpoints::fallback: Authority: None
2024-03-10T00:37:02.508538Z TRACE fallback: nxrm_two_portal::endpoints::fallback: Body: b"-----BEGIN PGP SIGNATURE-----\n\niQEzBAABCAAdFiEE2ZF+bRP9JGlcOQ1CnBySMa5hGNsFAmXtAKcACgkQnBySMa5h\nGNvgEgf+J3xGWdUL08GEPUW4Vtp7G+yTdjIDNX2TRxTgy0ysi5U88fMq1PzLWhZH\n4ZBnutrjsMFzCf5Dippddo8YK+4P72xleJp1VFQjUHVV1Jo1uMploEK7swtgdOpC\nW+bNz3ADJxhEqtiD/vuhCEpxKgxEI1J3+fIwuLjk88z4OIzMUUJAFXNKtWl1qAFY\nVfQela7eX0V6Kcp2pR+vnIRYHjoEBJ6AeH2f6JbOpUyH6EAS3nXjL4HpwRVLtS1r\nbiBxltClAhSO77llVp4q8ohU7Hg6bt/A/VsF/TbTMgyqqX+oFD2485wPvQkcRSws\nWV4slithZsHOG80JCw7JjHma2ipdaA==\n=5gxj\n-----END PGP SIGNATURE-----\n"
```
